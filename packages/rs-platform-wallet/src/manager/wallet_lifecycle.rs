//! Wallet creation, registration, and removal on [`PlatformWalletManager`].

use std::sync::Arc;

use dash_spv::chain::CheckpointManager;
use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

#[cfg(any(feature = "bls", feature = "eddsa"))]
use crate::changeset::ProviderKeyExtendedPubKey;
use crate::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, PlatformWalletChangeSet,
    PlatformWalletPersistence, ProviderKeyAccountEntry, WalletMetadataEntry,
};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletGeneration;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

/// Parse a BIP-39 mnemonic against every supported wordlist in turn,
/// returning the first language that yields a valid mnemonic.
///
/// `key_wallet::Mnemonic` only exposes language-tagged constructors,
/// so callers that take a user-supplied mnemonic must walk the
/// language list themselves to avoid rejecting non-English phrases as
/// "invalid English". BIP-39 wordlists are mutually exclusive per
/// phrase, so the first match is unambiguous.
fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    const LANGUAGES: [Language; 10] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::Italian,
        Language::Japanese,
        Language::Korean,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::Czech,
        Language::Portuguese,
    ];
    for lang in LANGUAGES {
        if let Ok(m) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(m);
        }
    }
    Err("phrase does not match any supported BIP-39 wordlist")
}

/// Test-only rendezvous fired inside [`PlatformWalletManager::remove_wallet_with_teardown`],
/// between the inner-manager removal and the public-map removal.
///
/// That window is exactly where a concurrent same-id `register_wallet` can
/// publish a NEW generation into both maps — the id is free in the inner
/// manager from the moment the removal above completes, and nothing gates
/// registration. Reproducing it deterministically from outside is not possible:
/// the window is bounded by two *different* locks, and the only lock a test
/// could hold to park the remover inside it (`self.wallets`) is the same lock
/// the registration must acquire to publish, so parking the remover would also
/// block the registration — and `tokio`'s `RwLock` hands the writer queue out
/// in FIFO order, which puts the remover first. A rendezvous is therefore the
/// only way to pin this ordering without a sleep or a completion-order race.
///
/// Compiled under `cfg(test)` only: neither this static nor its call site
/// exists in a production build, and it is not part of any public API.
#[cfg(test)]
pub(crate) type RemoveWalletMidpointHook = Box<
    dyn Fn(&WalletId) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

#[cfg(test)]
pub(crate) static REMOVE_WALLET_MIDPOINT_HOOK: std::sync::Mutex<Option<RemoveWalletMidpointHook>> =
    std::sync::Mutex::new(None);

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Create a PlatformWallet from a BIP39 mnemonic phrase.
    ///
    /// The mnemonic's language is auto-detected by trying each
    /// supported BIP-39 wordlist in turn (see
    /// [`parse_mnemonic_any_language`]). For passphrase-only flows or
    /// out-of-band seed material, derive the seed externally and use
    /// [`Self::create_wallet_from_seed_bytes`].
    ///
    /// `birth_height_override` controls SPV's compact-filter scan
    /// window for the new wallet. `None` (the default for fresh
    /// wallets) resolves the birth height from SPV's current
    /// confirmed header tip, so the scan window is `[H_now, ∞)` and
    /// anything funded before init is invisible — **but** when SPV is
    /// not running yet or header state is unavailable (e.g. wallet
    /// created before the SPV client is started), it falls back to
    /// the latest network checkpoint height, keeping the scan near
    /// the chain head instead of rescanning from genesis. `Some(0)`
    /// always requests a full historical scan from genesis (use
    /// sparingly — expensive on long-lived chains, but required when
    /// an address may have received funds before the wallet was first
    /// registered). `Some(h)` pins the scan start to a specific block
    /// height, useful when a known funding block is on record.
    pub async fn create_wallet_from_mnemonic(
        &self,
        mnemonic_phrase: &str,
        network: Network,
        accounts: WalletAccountCreationOptions,
        birth_height_override: Option<u32>,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let mnemonic = parse_mnemonic_any_language(mnemonic_phrase)
            .map_err(|e| PlatformWalletError::WalletCreation(format!("Invalid mnemonic: {}", e)))?;
        let wallet = Wallet::from_mnemonic(mnemonic, network, accounts).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from mnemonic: {}",
                e
            ))
        })?;
        self.register_wallet(wallet, birth_height_override).await
    }

    /// Create a PlatformWallet from raw seed bytes, initialize persisted
    /// state, register it with the manager and return an `Arc` handle.
    ///
    /// See [`Self::create_wallet_from_mnemonic`] for the
    /// `birth_height_override` semantics. `None` scans from the
    /// current SPV tip forward when SPV is running, otherwise from
    /// the latest network checkpoint; `Some(h)` is for callers that need to see funding
    /// deposited before the wallet was registered (e.g. a long-lived
    /// bank address pre-funded with testnet duffs).
    pub async fn create_wallet_from_seed_bytes(
        &self,
        network: Network,
        // By reference: the named stack copy we hold of the master secret is
        // wrapped in `Zeroizing` and scrubbed on drop. (`[u8; 64]` is `Copy`,
        // so a transient by-value copy still
        // crosses into key-wallet's `from_seed_bytes`, which consumes it into
        // its own zeroizing `Seed`; fully eliminating that residual copy
        // needs `from_seed_bytes` to take `&[u8; 64]` upstream in key-wallet.)
        seed_bytes: &[u8; 64],
        accounts: WalletAccountCreationOptions,
        birth_height_override: Option<u32>,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let seed = zeroize::Zeroizing::new(*seed_bytes);
        let wallet = Wallet::from_seed_bytes(*seed, network, accounts).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from seed bytes: {}",
                e
            ))
        })?;
        self.register_wallet(wallet, birth_height_override).await
    }

    /// Register a pre-built `Wallet` with the manager: insert into the
    /// `WalletManager`, build a `PlatformWallet` handle, load persisted
    /// state, and return an `Arc` to the managed wallet.
    ///
    /// `birth_height_override` flows through to both the in-memory
    /// `ManagedWalletInfo` sync checkpoint and the persisted
    /// `WalletMetadataEntry` so the SPV scan window is consistent
    /// across restarts. See [`Self::create_wallet_from_mnemonic`] for
    /// the contract.
    #[allow(clippy::type_complexity)]
    async fn register_wallet(
        &self,
        mut wallet: Wallet,
        birth_height_override: Option<u32>,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        // NOTE: the wallet id is NETWORK-SCOPED by construction.
        // `Wallet::from_mnemonic` / `from_seed_bytes` now stamp a
        // network-scoped id (key-wallet folds a domain-tagged,
        // wire-stable network byte into the digest), so the same
        // mnemonic yields a DISTINCT id per network. That makes every
        // downstream `walletId`-keyed structure network-correct by
        // construction — no per-network disambiguation needed in the
        // persistence layer, and network-blind child tables (UTXOs,
        // asset locks, platform addresses) can no longer cross-feed
        // between a mnemonic's per-network wallets. The watch-only
        // restore path (`Wallet::new_external_signable`) reuses the
        // persisted id verbatim, so it stays self-consistent across
        // launches.

        // Birth height resolution: explicit override wins; otherwise fall back
        // to SPV's confirmed header tip (default for fresh wallets — they only
        // need to see funding from now on). Before SPV has synced any headers
        // the tip is 0, so a brand-new wallet created at startup would otherwise
        // anchor at genesis and rescan the whole chain. Fall back to the latest
        // hardcoded checkpoint instead, keeping the scan near the chain head.
        let birth_height: u32 = match birth_height_override {
            Some(h) => h,
            None => {
                let tip = self
                    .spv_manager
                    .sync_progress()
                    .await
                    .and_then(|p| p.headers().ok().map(|h| h.tip_height()))
                    .unwrap_or(0);
                if tip > 0 {
                    tip
                } else {
                    CheckpointManager::for_network(self.sdk.network)
                        .last_checkpoint()
                        .map(|checkpoint| checkpoint.height)
                        .unwrap_or(0)
                }
            }
        };

        // `mut` so the platform-node (Ed25519) pool can be populated in
        // place below, BEFORE the address-pool snapshot is taken.
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, birth_height);

        let generation = Arc::new(WalletGeneration::new());

        // Snapshot per-account xpubs and address-pool entries BEFORE
        // the wallet / managed-info are moved into insert_wallet. The
        // persister sees everything needed to rebuild the wallet
        // watch-only (via `Wallet::new_watch_only`) plus populate
        // SwiftData's address table on next launch.
        let account_specs: Vec<(
            key_wallet::account::AccountType,
            key_wallet::bip32::ExtendedPubKey,
        )> = wallet
            .accounts
            .all_accounts()
            .iter()
            .map(|a| (a.account_type, a.account_xpub))
            .collect();
        // Provider key-material accounts (BLS operator keys / EdDSA
        // platform-node keys) live in dedicated `Option` fields on the
        // `AccountCollection` that `all_accounts()` deliberately
        // excludes, so snapshot them separately. They carry a
        // non-secp256k1 extended public key; the persister bincode-
        // encodes it and the restore path rebuilds them watch-only.
        #[allow(unused_mut)]
        let mut provider_key_account_registrations: Vec<ProviderKeyAccountEntry> = Vec::new();
        #[cfg(feature = "bls")]
        if let Some(bls) = wallet
            .accounts
            .bls_account_of_type(key_wallet::account::AccountType::ProviderOperatorKeys)
        {
            provider_key_account_registrations.push(ProviderKeyAccountEntry {
                account_type: key_wallet::account::AccountType::ProviderOperatorKeys,
                extended_public_key: ProviderKeyExtendedPubKey::Bls(bls.bls_public_key.clone()),
            });
        }
        #[cfg(feature = "eddsa")]
        if let Some(eddsa) = wallet
            .accounts
            .eddsa_account_of_type(key_wallet::account::AccountType::ProviderPlatformKeys)
        {
            // Pre-derive a fixed batch of platform-node public keys while
            // the wallet is still seed-bearing (`downgrade_to_external_signable`
            // hasn't run yet). Ed25519/SLIP-10 is hardened-only, so this
            // pool can never be extended later from the watch-only restore —
            // populating the managed pool now lets those keys ride the
            // normal typed-address persistence pipeline (persisted as
            // `PublicKeyType::EdDSA` core-address rows, rehydrated on load
            // by `restore_core_address_pools`) so the Node Keys screen lists
            // them from persistence with no keychain prompt. A derivation
            // failure here is non-fatal: fall back to an empty batch (the UI
            // then uses its resolver-based "Load Keys" path) rather than
            // aborting the whole wallet registration.
            let derived_platform_node_keys =
                crate::wallet::provider_key_at_index::derive_platform_node_public_keys(
                    &wallet,
                    wallet.network,
                    crate::wallet::provider_key_at_index::PLATFORM_NODE_KEY_PREDERIVE_COUNT,
                )
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        error = %e,
                        "failed to pre-derive platform-node keys at registration; \
                         the Node Keys screen will fall back to the resolver path"
                    );
                    Vec::new()
                });
            // Populate the managed platform-node pool in place, BEFORE the
            // address-pool snapshot below reads `all_managed_accounts()`, so
            // the EdDSA keys are captured as typed core-address rows. A
            // population failure is non-fatal for the same reason the
            // derivation failure above is.
            if let Err(e) = crate::wallet::provider_key_at_index::populate_platform_node_pool(
                &mut wallet_info,
                &derived_platform_node_keys,
                wallet.network,
            ) {
                tracing::warn!(
                    error = %e,
                    "failed to populate the managed platform-node pool at registration; \
                     the Node Keys screen will fall back to the resolver path"
                );
            }
            provider_key_account_registrations.push(ProviderKeyAccountEntry {
                account_type: key_wallet::account::AccountType::ProviderPlatformKeys,
                extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                    eddsa.ed25519_public_key.clone(),
                ),
            });
        }
        // Snapshot core (BIP44/CoinJoin/identity/provider/DashPay)
        // address pools. PlatformPayment accounts live in a separate
        // collection on `ManagedWalletInfo` and are handled below.
        let mut address_snapshots: Vec<(
            key_wallet::account::AccountType,
            Vec<(
                key_wallet::managed_account::address_pool::AddressPoolType,
                Vec<key_wallet::AddressInfo>,
            )>,
        )> = wallet_info
            .all_managed_accounts()
            .iter()
            .map(|managed| {
                // `all_managed_accounts()` returns `ManagedAccountRef`;
                // the upstream split made `managed_account_type` a
                // delegating method (it was a field on the pre-split
                // unified `ManagedCoreAccount`).
                let account_type = managed.managed_account_type().to_account_type();
                let pools = managed
                    .managed_account_type()
                    .address_pools()
                    .iter()
                    .map(|pool| {
                        let infos: Vec<key_wallet::AddressInfo> =
                            pool.addresses.values().cloned().collect();
                        (pool.pool_type, infos)
                    })
                    .collect();
                (account_type, pools)
            })
            .collect();

        // Platform payment (DIP-17) accounts sit in their own
        // collection and each owns a single `AddressPool`. Snapshot
        // them here so the Storage Explorer can show them under the
        // PlatformPayment account row alongside core pools.
        for managed in wallet_info.all_platform_payment_managed_accounts() {
            let account_type = key_wallet::account::AccountType::PlatformPayment {
                account: managed.account,
                key_class: managed.key_class,
            };
            let pool = &managed.addresses;
            let infos: Vec<key_wallet::AddressInfo> = pool.addresses.values().cloned().collect();
            address_snapshots.push((account_type, vec![(pool.pool_type, infos)]));
        }

        // Network-INDEPENDENT group id, snapshotted BEFORE `wallet` is
        // moved into `insert_wallet` below. The per-network `wallet_id`
        // differs per network for the same seed (see the scoping note
        // above); this digest deliberately omits the network byte
        // (`None`), so every network's wallet for one seed shares it and
        // the persister can group a seed's sibling-network rows by it.
        // Watch-only / external-signable wallets carry no root key, so
        // there's nothing to hash — fall back to the scoped `wallet_id`
        // (a group of one).
        let wallet_group_id = wallet
            .root_extended_pub_key_cow()
            .map(|root| Wallet::compute_wallet_id_from_root_extended_pub_key(&root, None))
            .unwrap_or(wallet.wallet_id);

        let platform_info = PlatformWalletInfo {
            core_wallet: wallet_info,
            generation: Arc::clone(&generation),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
        };

        wallet.downgrade_to_external_signable();

        // Insert into WalletManager. A duplicate (same network-scoped
        // wallet id already registered) surfaces as the typed
        // `WalletAlreadyExists` so the create FFI / Swift call sites can
        // treat re-registering an existing wallet as a benign no-op
        // instead of substring-matching the error text. Everything else
        // stays `WalletCreation`.
        let wallet_id = {
            let mut wm = self.wallet_manager.write().await;
            wm.insert_wallet(wallet, platform_info).map_err(|e| {
                if matches!(e, key_wallet_manager::WalletError::WalletExists(_)) {
                    PlatformWalletError::WalletAlreadyExists(e.to_string())
                } else {
                    PlatformWalletError::WalletCreation(format!(
                        "Failed to register wallet in WalletManager: {}",
                        e
                    ))
                }
            })?
        };

        // Emit metadata + per-account xpubs + per-pool address
        // snapshots to the persister so the watch-only restore path
        // has everything it needs on next launch. The whole
        // registration round travels as a single
        // [`PlatformWalletChangeSet`] through the canonical `store`
        // entry point — backends (FFI, SQLite, in-memory) see one
        // atomic round rather than three side-channel calls.
        //
        // This round is the only record a wallet can be rebuilt
        // watch-only from on next launch, so it is load-bearing: a
        // `store` error rolls back the in-memory insert and aborts
        // registration — same shape as the `load_persisted` /
        // `initialize_from_persisted` failure paths below.

        // `birth_height` was resolved at the top of `register_wallet`
        // and seeded into `ManagedWalletInfo`; reuse it here so the
        // persisted `WalletMetadataEntry` agrees with the in-memory
        // sync checkpoint.
        let mut registration_changeset = PlatformWalletChangeSet {
            wallet_metadata: Some(WalletMetadataEntry {
                network: self.sdk.network,
                wallet_group_id,
                birth_height,
            }),
            account_registrations: account_specs
                .iter()
                .map(|(account_type, account_xpub)| AccountRegistrationEntry {
                    account_type: *account_type,
                    account_xpub: *account_xpub,
                })
                .collect(),
            provider_key_account_registrations,
            ..Default::default()
        };

        // Every account type contributes at least one pool (external,
        // or a single `Absent` pool for degenerate types); Standard
        // accounts contribute two. Ordering within a pool is by
        // derivation index via `BTreeMap::values`. Empty pools are
        // dropped here so the FFI receiver can match the previous
        // "skip empty pools" semantics without re-deciding it.
        for (account_type, pools) in &address_snapshots {
            for (pool_type, infos) in pools {
                if infos.is_empty() {
                    continue;
                }
                registration_changeset
                    .account_address_pools
                    .push(AccountAddressPoolEntry {
                        account_type: *account_type,
                        pool_type: *pool_type,
                        addresses: infos.clone(),
                    });
            }
        }

        if let Err(e) = self.persister.store(wallet_id, registration_changeset) {
            tracing::error!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "failed to persist wallet registration changeset"
            );
            let mut wm = self.wallet_manager.write().await;
            if let Err(e) = wm.remove_wallet(&wallet_id) {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "rollback: remove_wallet failed while unwinding a failed wallet registration"
                );
            }
            return Err(PlatformWalletError::WalletCreation(format!(
                "Failed to persist wallet registration changeset: {}",
                e
            )));
        }

        // Build the PlatformWallet handle.
        let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
            &self.spv_manager,
        )));

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;
        let platform_wallet = PlatformWallet::new(
            Arc::clone(&self.sdk),
            wallet_id,
            Arc::clone(&self.wallet_manager),
            generation,
            Arc::clone(&self.lock_notify),
            persister_dyn,
            broadcaster,
        );

        // Load persisted state. The only area wired up today is the
        // platform-address provider — `from_persisted` skips the live
        // `AddressPool` scan `initialize` would otherwise do.
        // Per-wallet UTXOs / unused asset locks ship in the snapshot
        // but don't have an active restore path yet.
        //
        // The two `?` returns below would otherwise leave the wallet
        // half-registered (present in `wallet_manager` from the
        // earlier `insert_wallet`, absent from `self.wallets`),
        // poisoning every retry on `WalletAlreadyExists`. Roll back
        // before bailing — same shape as `manager::load`.
        let crate::changeset::ClientStartState {
            mut platform_addresses,
            wallets: _,
            #[cfg(feature = "shielded")]
                shielded: _,
        } = match platform_wallet.load_persisted() {
            Ok(state) => state,
            Err(e) => {
                let mut wm = self.wallet_manager.write().await;
                if let Err(e) = wm.remove_wallet(&wallet_id) {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "rollback: remove_wallet failed while unwinding a failed wallet setup"
                    );
                }
                return Err(PlatformWalletError::WalletCreation(format!(
                    "Failed to load persisted wallet state: {}",
                    e
                )));
            }
        };

        if let Some(persisted) = platform_addresses.remove(&wallet_id) {
            if let Err(e) = platform_wallet
                .platform()
                .initialize_from_persisted(persisted)
                .await
            {
                let mut wm = self.wallet_manager.write().await;
                if let Err(e) = wm.remove_wallet(&wallet_id) {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "rollback: remove_wallet failed while unwinding a failed wallet setup"
                    );
                }
                return Err(PlatformWalletError::WalletCreation(format!(
                    "Failed to restore persisted platform address state: {}",
                    e
                )));
            }
        } else {
            platform_wallet.platform().initialize().await;
        }

        let platform_wallet = Arc::new(platform_wallet);

        // Register the PlatformWallet handle.
        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(wallet_id, Arc::clone(&platform_wallet));
        }

        // Re-seed the lock-free balance atomic from the wallet's inner
        // balance now that the wallet is in `self.wallets`.
        //
        // A wallet added while SPV is already synced (e.g. importing an
        // existing mnemonic with `birth_height = 0`) has its historical
        // funds backfilled by the SPV rescan that `insert_wallet` above
        // triggers. That rescan can complete — emitting the
        // `BlockProcessed` event that carries the post-backfill balance —
        // *before* this wallet lands in `self.wallets`, so
        // `BalanceUpdateHandler` drops those events (the wallet isn't in
        // the map yet) and the atomic stays at zero even though the inner
        // `ManagedWalletInfo` balance is correct. Mirror the inner balance
        // into the atomic here (as `manager::load` does for restored
        // wallets); any later block events are applied normally now that
        // the wallet is mapped.
        {
            let wm = self.wallet_manager.read().await;
            if let Some(info) = wm.get_wallet_info(&wallet_id) {
                let b = &info.core_wallet.balance;
                platform_wallet.balance().set(
                    b.confirmed(),
                    b.unconfirmed(),
                    b.immature(),
                    b.locked(),
                );
            }
        }

        // Best-effort identity discovery. For a recovery flow (existing
        // mnemonic re-typed by the user) this hydrates every identity
        // the wallet had on Platform without the caller having to fire
        // `discover` manually. For a fresh wallet the gap-limit miss
        // loop bails out after a handful of empty queries (~seconds)
        // and produces nothing — same end state, slightly slower than
        // skipping. Failures here are logged but never block wallet
        // registration: a sync hiccup or offline DAPI shouldn't lose
        // the user the wallet they just imported.
        if let Err(e) = platform_wallet.identity().sync().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "Identity discovery failed during wallet registration; \
                 callers can retry via PlatformWallet::identity().discover()"
            );
        }

        Ok(platform_wallet)
    }

    /// Remove a wallet from the manager.
    ///
    /// Runs under the removed generation's lifecycle gate — see
    /// [`remove_wallet_with_teardown`](Self::remove_wallet_with_teardown), of
    /// which this is the no-extra-teardown case.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        self.remove_wallet_with_teardown(wallet_id, |_| {}).await
    }

    /// Remove a wallet from the manager and run `tear_down` on the removed
    /// wallet — both under that generation's exclusive lifecycle gate, as one
    /// linearization point.
    ///
    /// # Why the gate lives here rather than in the caller
    ///
    /// Removing the generation and tearing down the deferred state that names it
    /// (the [`SignedPaymentRegistry`](crate::SignedPaymentRegistry) tokens and
    /// the FFI's finalized-transaction handles) must be indivisible. If they are
    /// two steps, a retained handle can broadcast in the gap: the removal's own
    /// `.await`s (shielded-coordinator and identity-sync unregistration) sit
    /// inside it, `CoreWallet::is_same_generation` passes for a removed
    /// generation (a removed generation matches itself), and the reservation age
    /// guard is disabled once `last_processed_height` returns `None`. So a
    /// payment for a wallet the host already deleted reaches the network
    /// (`dashpay/platform#4185`).
    ///
    /// Taking the gate *inside* this method rather than leaving it to the caller
    /// is deliberate: `PlatformWalletManager` is public and `SignedPaymentRegistry`
    /// is re-exported, so a direct Rust embedder that never goes through the FFI
    /// would otherwise remove wallets with no exclusion at all, and could
    /// interleave between a payment operation's liveness check and its register
    /// or network action. `tear_down` is the hook that lets the FFI layer sweep
    /// its own process-global handle storages inside the same critical section
    /// without the gate ever being optional.
    ///
    /// `tear_down` is synchronous by design — it runs while the gate is held, and
    /// every sweep it needs (`remove_entries_for_wallet`,
    /// `HandleStorage::remove_matching`) is a synchronous map retain.
    ///
    /// ## Lock ordering
    ///
    /// The generation gate is always taken BEFORE the manager locks. The lookup
    /// that finds the gate takes `wallets` briefly and **drops it before**
    /// awaiting the gate, so no manager lock is ever held across a gate
    /// acquisition; payment operations likewise take the gate and only then await
    /// the manager. The order is total, so the two cannot deadlock.
    ///
    /// ## Removal is by generation identity, not by key
    ///
    /// The gate excludes *payment operations on this generation*. It does not
    /// exclude a fresh **registration** under the same `wallet_id`:
    /// [`register_wallet`](Self::register_wallet) mints its own
    /// [`WalletGeneration`] and takes no gate at all, by design — a create must
    /// never queue behind an unrelated wallet's teardown.
    ///
    /// So once this method has removed generation G1 from the inner
    /// `wallet_manager`, the id is free and a concurrent registration can publish
    /// a *different* generation G2 into both maps before this method reaches its
    /// own `self.wallets` removal — the two removals are separately locked, with
    /// no happens-before edge between them and the registration. Removing by key
    /// there would take G2 out of the public map (leaving it registered in the
    /// inner manager, invisible and unremovable) and hand G2 to `tear_down`,
    /// which would sweep G2's registry tokens and V2 handles while holding only
    /// G1's gate — i.e. with G2's payment operations *not* excluded, which is the
    /// exact property this gate exists to provide.
    ///
    /// The `Arc<PlatformWallet>` validated under the gate is therefore retained,
    /// and the public-map entry is removed only while it still names that same
    /// generation. Both maps, the returned handle and the `tear_down` argument
    /// are then all that one generation (`dashpay/platform#4185`). The one
    /// remaining id-keyed step is the shielded coordinator detach below, which
    /// has no generation concept at all; a generation that has just been
    /// registered has not run `bind_shielded` yet, so it holds no coordinator
    /// entry to detach.
    ///
    /// The inner-manager removal needs no such check: G1 can only leave
    /// `wallet_manager` through this method (which requires G1's gate, held here)
    /// or through a registration/load rollback for an insert that could not have
    /// happened while G1 occupied the id — so while the gate is held and before
    /// the removal below, the inner entry is still G1 by construction.
    pub async fn remove_wallet_with_teardown<F>(
        &self,
        wallet_id: &WalletId,
        tear_down: F,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError>
    where
        F: FnOnce(&Arc<PlatformWallet>),
    {
        // Find the generation registered under `wallet_id` and take ITS gate.
        // Re-validated after acquisition because the wallet could have been
        // removed and re-created under the same id while we waited: in that case
        // we hold the OLD generation's gate, which excludes nothing relevant to
        // the new one, so retry against the generation that is actually current.
        //
        // The validated handle is carried out of the loop: it is both what this
        // call returns and tears down, and the identity every mutation below is
        // matched against.
        let (removed, _teardown) = loop {
            let candidate = {
                let wallets = self.wallets.read().await;
                match wallets.get(wallet_id) {
                    None => {
                        return Err(PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))
                    }
                    Some(wallet) => Arc::clone(wallet),
                }
            };
            let guard = candidate.generation().teardown_guard().await;
            let still_current = {
                let wallets = self.wallets.read().await;
                wallets
                    .get(wallet_id)
                    .is_some_and(|wallet| Arc::ptr_eq(wallet.generation(), candidate.generation()))
            };
            if still_current {
                break (candidate, guard);
            }
            // Drop this generation's guard and re-resolve.
            drop(guard);
        };
        let generation = Arc::clone(removed.generation());

        let owned_identity_ids: Vec<dpp::prelude::Identifier> = {
            let mut wm = self.wallet_manager.write().await;
            let ids = match wm.get_wallet_info(wallet_id) {
                Some(info) => info
                    .identity_manager
                    .wallet_identities
                    .get(wallet_id)
                    .map(|inner| {
                        use dpp::identity::accessors::IdentityGettersV0;
                        inner
                            .values()
                            .map(|managed| managed.identity.id())
                            .collect()
                    })
                    .unwrap_or_default(),
                None => Vec::new(),
            };
            if let Err(e) = wm.remove_wallet(wallet_id) {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "remove_wallet: inner wallet-manager removal failed (state may be inconsistent)"
                );
            }
            ids
        };

        // Test-only rendezvous: the window a concurrent same-id registration can
        // publish a new generation into. See `REMOVE_WALLET_MIDPOINT_HOOK`.
        #[cfg(test)]
        {
            let pending = REMOVE_WALLET_MIDPOINT_HOOK
                .lock()
                .expect("remove-wallet midpoint hook mutex")
                .as_ref()
                .map(|hook| hook(wallet_id));
            if let Some(rendezvous) = pending {
                rendezvous.await;
            }
        }

        // Remove the public-map entry only while it still names the generation
        // validated under the gate. A concurrent same-id registration could have
        // published a NEW generation here in the window since the inner removal
        // above freed the id (see the "Removal is by generation identity" note on
        // this method); removing by key would evict that live wallet and hand it
        // to `tear_down` under the wrong gate.
        {
            let mut wallets = self.wallets.write().await;
            let entry_is_ours = wallets
                .get(wallet_id)
                .is_some_and(|wallet| Arc::ptr_eq(wallet.generation(), &generation));
            if entry_is_ours {
                wallets.remove(wallet_id);
            } else {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "remove_wallet: a new generation was registered under this id while the \
                     previous one was being removed; leaving the new registration in place"
                );
            }
        }

        // Detach the wallet's shielded state from the network
        // coordinator. After the Phase-2b refactor the coordinator
        // owns the per-`SubwalletId` viewing-key registry and the
        // per-wallet `WalletPersister`; without this call a deleted
        // wallet's shielded notes keep getting fetched,
        // trial-decrypted, and re-persisted through the stale
        // persister on the next `coordinator.sync()` pass —
        // resurrecting private shielded history on disk after the
        // host believed the wallet was gone. Drops the registry +
        // persister entries and the per-subwallet store state.
        //
        // The handle is marked detached first: a caller that resolved
        // this wallet before the removal can still be inside a bind
        // (the seed-backed path resolves a mnemonic through the host,
        // so it may take arbitrarily long), and its registration would
        // otherwise land after the unregister below and resurrect
        // exactly the state this call exists to drop. The flag is read
        // inside the coordinator's install transaction, which the
        // unregister also takes, so the two cannot interleave.
        // Unconditional: a removal with no coordinator yet has nothing
        // to unregister, but the handle must still be barred from
        // binding onto one a later `configure_shielded` installs.
        #[cfg(feature = "shielded")]
        removed.mark_shielded_detached();
        #[cfg(feature = "shielded")]
        if let Some(coordinator) = self.shielded_coordinator().await {
            coordinator.unregister_wallet(*wallet_id).await;
        }

        for identity_id in &owned_identity_ids {
            self.identity_sync_manager
                .unregister_identity(identity_id)
                .await;
        }

        // Still under the generation's teardown gate: any deferred state naming
        // this generation is dropped in the same critical section as the removal
        // itself, so no payment operation can observe the wallet as live and then
        // act on it after this returns.
        tear_down(&removed);

        Ok(removed)
    }
}

#[cfg(test)]
mod scoped_wallet_id_tests {
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    // Canonical all-`abandon` BIP-39 test vector. Deterministic, so the
    // ids below are reproducible across runs.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    fn wallet_id_for(network: Network) -> [u8; 32] {
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let wallet =
            Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
                .expect("wallet construction");
        // This is the id the manager keys on (insert_wallet returns it,
        // the create FFI hands it to Swift) — exercises the same
        // construction path `create_wallet_from_mnemonic` uses.
        wallet.wallet_id
    }

    /// The network-INDEPENDENT group id `register_wallet` computes and
    /// persists onto every per-network row, so the iOS Wallet Info
    /// "Networks" section can group a seed's sibling-network wallets.
    /// Mirrors the `register_wallet` derivation exactly.
    fn wallet_group_id_for(network: Network) -> [u8; 32] {
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let wallet =
            Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
                .expect("wallet construction");
        wallet
            .root_extended_pub_key_cow()
            .map(|root| Wallet::compute_wallet_id_from_root_extended_pub_key(&root, None))
            .unwrap_or(wallet.wallet_id)
    }

    /// The same mnemonic must yield a DISTINCT wallet id on each network.
    /// This is the property the whole per-network persistence model now
    /// relies on (rust-dashcore #793: network-scoped id by default).
    #[test]
    fn same_mnemonic_yields_distinct_ids_per_network() {
        let mainnet = wallet_id_for(Network::Mainnet);
        let testnet = wallet_id_for(Network::Testnet);
        let devnet = wallet_id_for(Network::Devnet);
        let regtest = wallet_id_for(Network::Regtest);

        let all = [mainnet, testnet, devnet, regtest];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(
                    all[i], all[j],
                    "wallet ids for two different networks must differ \
                     (index {i} vs {j}) — scoped-id regression"
                );
            }
        }
    }

    /// Re-deriving the same (mnemonic, network) must be stable, otherwise
    /// the watch-only restore path (which reuses the persisted id) would
    /// drift across launches.
    #[test]
    fn same_mnemonic_same_network_is_stable() {
        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Devnet,
            Network::Regtest,
        ] {
            assert_eq!(
                wallet_id_for(network),
                wallet_id_for(network),
                "wallet id must be stable across re-derivation for {network:?}"
            );
        }
    }

    /// The group id must be network-INDEPENDENT: the same seed yields
    /// the SAME group id on every network (this is what lets the Wallet
    /// Info "Networks" section discover a seed's sibling-network rows
    /// now that the scoped `walletId` differs per network). It must also
    /// differ from the scoped id, or grouping would collapse back into
    /// the per-network id and find nothing.
    #[test]
    fn group_id_is_network_independent_and_differs_from_scoped_id() {
        let g_main = wallet_group_id_for(Network::Mainnet);
        let g_test = wallet_group_id_for(Network::Testnet);
        let g_dev = wallet_group_id_for(Network::Devnet);
        let g_reg = wallet_group_id_for(Network::Regtest);

        // Same seed → identical group id across every network.
        assert_eq!(g_main, g_test, "group id must not depend on network");
        assert_eq!(g_main, g_dev, "group id must not depend on network");
        assert_eq!(g_main, g_reg, "group id must not depend on network");

        // …but the group id is NOT the scoped id (else grouping siblings
        // by it would degenerate to the per-network id and miss them).
        assert_ne!(
            g_main,
            wallet_id_for(Network::Mainnet),
            "group id must differ from the network-scoped id"
        );
    }
}

#[cfg(test)]
mod register_wallet_duplicate_tests {
    use std::sync::Arc;

    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// No-op persister: lifecycle tests don't need the real persistence
    /// pipeline, just a handle satisfying the constructor.
    struct NoopPersister;

    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// Build a manager wired to a no-op persister over a mock SDK. The
    /// duplicate-create path under test never reaches the network: the
    /// first `create` returns `Ok` (its only network touch — best-effort
    /// `identity().sync()` — is logged-and-ignored), and the second
    /// fails at `WalletManager::insert_wallet` before any query.
    fn make_manager() -> Arc<PlatformWalletManager<NoopPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        Arc::new(PlatformWalletManager::new(sdk, persister, event_handler))
    }

    /// Registering the SAME wallet (same mnemonic/seed + network) twice
    /// must surface the typed `WalletAlreadyExists` on the second call —
    /// NOT `WalletCreation`. This exercises the real producer path
    /// (`register_wallet` → `WalletManager::insert_wallet` →
    /// `WalletError::WalletExists` mapping) end-to-end; the prior
    /// isolated FFI-mapper test missed that nothing ever constructed
    /// `WalletAlreadyExists` on the create path.
    #[tokio::test]
    async fn duplicate_register_wallet_returns_wallet_already_exists() {
        let manager = make_manager();

        let network = Network::Testnet;
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let seed_bytes = mnemonic.to_seed("");

        // First registration succeeds. `Some(0)` skips the SPV-tip
        // birth-height lookup so the test never consults SPV.
        manager
            .create_wallet_from_seed_bytes(
                network,
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("first create should succeed");

        // Second registration of the identical (seed, network) wallet
        // collides on the network-scoped wallet id inside
        // `WalletManager::insert_wallet`.
        let err = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect_err("second create of the same wallet must fail");

        assert!(
            matches!(err, PlatformWalletError::WalletAlreadyExists(_)),
            "duplicate create must map to WalletAlreadyExists, got: {err:?}"
        );
    }
}

/// Removal versus a same-id re-registration that lands *during* the removal
/// (`dashpay/platform#4185` review).
///
/// The invariant: `remove_wallet_with_teardown` removes, returns and tears down
/// exactly the wallet generation it validated under that generation's lifecycle
/// gate — never a different generation that appeared under the same
/// `wallet_id` while the removal was in progress.
#[cfg(test)]
mod remove_versus_recreate_tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use super::REMOVE_WALLET_MIDPOINT_HOOK;
    use crate::test_support::test_platform_wallet_manager;
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::PlatformWallet;

    /// The mnemonic `test_platform_wallet_manager` builds its wallet from, so
    /// re-registering from the same seed collides on the same network-scoped
    /// `wallet_id` — which is the whole point of the scenario.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// Clears [`REMOVE_WALLET_MIDPOINT_HOOK`] on drop, including on panic, so a
    /// failing assertion can never leave the hook armed for another test in the
    /// same binary.
    struct MidpointHookGuard;

    impl Drop for MidpointHookGuard {
        fn drop(&mut self) {
            if let Ok(mut slot) = REMOVE_WALLET_MIDPOINT_HOOK.lock() {
                *slot = None;
            }
        }
    }

    /// Requirement: a wallet generation registered while a removal is in flight
    /// must survive that removal — in BOTH maps — and the removal must return
    /// and tear down the generation it actually validated.
    ///
    /// Deterministic by construction: the re-registration runs from a rendezvous
    /// fired inside the removal, in the exact window between the inner-manager
    /// removal and the public-map removal, so there is no completion order to
    /// race and no sleep. The registration itself is the real
    /// `create_wallet_from_seed_bytes` → `register_wallet` path, publishing into
    /// the inner `WalletManager` and then `self.wallets` in the production
    /// order.
    ///
    /// Why that window is reachable in production: the removal frees the id in
    /// the inner manager and only then acquires `self.wallets` — two separately
    /// locked stages with no happens-before edge to a concurrent registration,
    /// which takes no lifecycle gate at all (it mints its own generation). A
    /// remover descheduled in that gap resumes into a map that already names the
    /// new generation.
    ///
    /// Before the fix the removal took the public-map entry by KEY: it evicted
    /// the freshly registered generation — leaving it registered in the inner
    /// manager but invisible and unremovable through `self.wallets` — returned
    /// it to the caller, and handed it to `tear_down`, which sweeps that
    /// generation's registry tokens and V2 finalized-transaction handles while
    /// holding only the OLD generation's gate. The new generation's in-flight
    /// payment operations were therefore not excluded, which is the one property
    /// the gate exists to provide.
    #[tokio::test]
    async fn removal_leaves_a_generation_registered_during_it_intact() {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let original = manager
            .get_wallet(&wallet_id)
            .await
            .expect("fixture wallet is registered");

        // Filled by the rendezvous with the generation the re-registration
        // publishes, so the assertions can name it rather than infer it.
        let recreated: Arc<Mutex<Option<Arc<PlatformWallet>>>> = Arc::new(Mutex::new(None));

        let _hook_guard = MidpointHookGuard;
        {
            let manager_for_hook = Arc::clone(&manager);
            let recreated_slot = Arc::clone(&recreated);
            // One-shot: the re-registration must not recurse into a later
            // removal, and no other test in this binary may see the hook.
            let fired = AtomicBool::new(false);
            *REMOVE_WALLET_MIDPOINT_HOOK
                .lock()
                .expect("midpoint hook mutex") = Some(Box::new(move |id| {
                let already_fired = fired.swap(true, Ordering::SeqCst);
                let manager = Arc::clone(&manager_for_hook);
                let recreated_slot = Arc::clone(&recreated_slot);
                let id = *id;
                Box::pin(async move {
                    if already_fired {
                        return;
                    }
                    let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                        .expect("valid test mnemonic");
                    let seed_bytes = mnemonic.to_seed("");
                    // The real registration path: inner `WalletManager` first,
                    // then `self.wallets`. `Some(0)` skips the SPV-tip lookup.
                    let wallet = manager
                        .create_wallet_from_seed_bytes(
                            Network::Testnet,
                            &seed_bytes,
                            WalletAccountCreationOptions::Default,
                            Some(0),
                        )
                        .await
                        .expect(
                            "the id is free in the inner manager at this point, so a same-seed \
                             re-registration must succeed",
                        );
                    assert_eq!(wallet.wallet_id(), id, "the fixture seeds must collide");
                    *recreated_slot.lock().expect("recreated slot") = Some(wallet);
                })
            }));
        }

        // Capture what teardown was actually handed.
        let torn_down: Arc<Mutex<Option<Arc<WalletGeneration>>>> = Arc::new(Mutex::new(None));
        let torn_down_slot = Arc::clone(&torn_down);

        let removed = manager
            .remove_wallet_with_teardown(&wallet_id, move |wallet| {
                *torn_down_slot.lock().expect("torn-down slot") =
                    Some(Arc::clone(wallet.generation()));
            })
            .await
            .expect("removal of the validated generation succeeds");

        let recreated = recreated
            .lock()
            .expect("recreated slot")
            .clone()
            .expect("the rendezvous must have re-registered the wallet");
        assert!(
            !Arc::ptr_eq(original.generation(), recreated.generation()),
            "the fixture must produce two distinct generations under one wallet id"
        );

        // 1. The removal returns the generation it validated under the gate.
        assert!(
            Arc::ptr_eq(removed.generation(), original.generation()),
            "remove_wallet_with_teardown returned a generation it never validated — it took the \
             public-map entry by key and got the generation registered during the removal"
        );

        // 2. …and tears down that same generation. Sweeping the other one here
        //    would run without holding ITS gate, so its in-flight payment
        //    operations would not be excluded.
        let torn_down = torn_down
            .lock()
            .expect("torn-down slot")
            .clone()
            .expect("tear_down must have run");
        assert!(
            Arc::ptr_eq(&torn_down, original.generation()),
            "tear_down was handed a generation whose lifecycle gate this removal does not hold"
        );

        // 3. The generation registered during the removal is still published.
        let still_registered = manager
            .get_wallet(&wallet_id)
            .await
            .expect("a wallet registered during a removal must remain in the public map");
        assert!(
            Arc::ptr_eq(still_registered.generation(), recreated.generation()),
            "the public map must still name the generation the registration published"
        );

        // 4. …and both maps agree about it: `is_current_generation` compares the
        //    handle against the inner `WalletManager`, so this fails if the
        //    removal evicted it from one map only.
        assert!(
            recreated.core().is_current_generation().await,
            "the re-registered generation must be live in both the inner manager and the public \
             map — evicting it from one leaves an invisible, unremovable wallet"
        );

        // 5. The removed generation is gone.
        assert!(
            !original.core().is_current_generation().await,
            "the validated generation must be gone from the inner manager"
        );
    }
}
