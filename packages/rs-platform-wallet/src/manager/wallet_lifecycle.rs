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
use crate::wallet::core::WalletBalance;
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

        let balance = Arc::new(WalletBalance::new());

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
            balance: Arc::clone(&balance),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
        };

        wallet.downgrade_to_external_signable();

        // Everything from here to the `self.wallets` publish below is one
        // lifecycle transition and must not interleave with another. The
        // steps are individually locked but separately so: the wallet is
        // live in `wallet_manager` from `insert_wallet` onward, yet
        // absent from `wallets` until the very end, and the rollback arms
        // in between undo only the former.
        //
        // A concurrent `remove_wallet` of the SAME deterministic id would
        // read that torn state — the id is registered, so it drops the
        // `wallet_manager` entry out from under this in-flight
        // registration, and its own `wallets.remove` then either misses
        // (this registration has not published yet, so the removal
        // reports `WalletNotFound` after having already destroyed the
        // wallet-manager entry) or detaches the generation this call is
        // about to return to its caller. Serializing the whole transition
        // is what makes "registered" and "published" the same instant to
        // every other lifecycle caller.
        //
        // Held across `persister.store` and `load_persisted` (both of
        // which reenter the host synchronously on iOS) because the
        // rollback arms they guard are part of the same transition;
        // deliberately NOT across the best-effort `identity().sync()`
        // network round-trip at the bottom, which is outside it.
        let lifecycle = self.lock_wallet_lifecycle_serial().await;

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
            balance,
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

        // Register the PlatformWallet handle. This publish closes the
        // lifecycle transition: from here the wallet is consistently
        // present in both maps, so a queued `remove_wallet` sees a whole
        // wallet rather than a half-built one.
        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(wallet_id, Arc::clone(&platform_wallet));
        }
        drop(lifecycle);

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
    /// # Asset-lock manager lifecycle
    ///
    /// The wallet's [`AssetLockManager`](crate::AssetLockManager) is
    /// retired *before* the shared `WalletManager` entry is dropped, and
    /// only then is anything else torn down. That ordering is
    /// load-bearing, not tidiness:
    ///
    /// * Subordinate handles outlive this call. `PlatformWallet` hands out
    ///   `Arc<AssetLockManager<_>>` clones (the FFI parks them in its own
    ///   handle storage), so a caller can still hold — and drive — the old
    ///   manager after the wallet is gone. The manager resolves state
    ///   through the shared `WalletManager` by `wallet_id` alone.
    /// * `wallet_id` is deterministic in (seed, network). Re-importing the
    ///   same mnemonic recreates the *same* id over a brand-new
    ///   `PlatformWalletInfo` and a brand-new `AssetLockManager` with its
    ///   own `status_persist_serial`. A retained old manager would then be
    ///   live against replacement state, and the two managers would mutate
    ///   and persist the same asset-lock rows under *different* mutexes —
    ///   reintroducing across instances exactly the stale-snapshot enqueue
    ///   reversal that serialization fixes within one instance.
    ///
    /// `AssetLockManager::deactivate` closes that window from both sides:
    /// it takes the old manager's `status_persist_serial`, so it cannot
    /// land in the middle of somebody's mutate→enqueue unit (removal waits
    /// for the unit to finish), and every such unit re-reads the flag once
    /// it holds that same mutex, so anything parked on an await when the
    /// flag flipped fails before it can touch a wallet row or the
    /// persister. Because re-registration must first `insert_wallet` into
    /// the shared `WalletManager` — which still holds this wallet's entry
    /// at this point — no replacement can exist until deactivation is
    /// already done.
    ///
    /// `deactivate` is called with no other lock held. It acquires
    /// `status_persist_serial`, and holders of that mutex go on to take
    /// `wallet_manager.write()`; calling it under `wallet_manager` would
    /// invert the documented lock order and deadlock.
    ///
    /// # Why retirement is not enough on its own
    ///
    /// `deactivate` is per-*instance*. It makes the handle a caller
    /// already holds harmless; it says nothing about which generation
    /// owns the map entry. The three steps below (retire → drop the
    /// `WalletManager` entry → detach from `wallets`) take their locks
    /// one at a time, and the moment the middle step completes the
    /// deterministic `wallet_id` is free — so a concurrent
    /// same-mnemonic `register_wallet` legitimately succeeds and
    /// publishes a *replacement* generation into `wallets` before this
    /// call reaches its own detach. The unqualified removal that
    /// followed then took the replacement out: a live wallet, with a
    /// live asset-lock manager, registered in `wallet_manager` but
    /// invisible in `wallets` (so no balance update or sync coordinator
    /// touches it) and un-removable, because the next `remove_wallet`
    /// takes the `WalletNotFound` arm.
    ///
    /// The whole transition therefore runs under
    /// [`wallet_lifecycle_serial`](PlatformWalletManager::wallet_lifecycle_serial),
    /// which `register_wallet` and `load_from_persistor` also hold
    /// across their own publish spans, and the detach is additionally
    /// generation-checked (`Arc::ptr_eq` against the handle actually
    /// retired) so no future path can reintroduce the swap silently.
    ///
    /// Idempotency is unchanged: a wallet absent from `self.wallets` still
    /// returns [`PlatformWalletError::WalletNotFound`] after the shared
    /// `WalletManager` entry is cleaned up, and `deactivate` is itself a
    /// no-op on an already-retired manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        // The whole removal is one lifecycle transition: retire, drop the
        // shared `WalletManager` entry, detach from `wallets`. Held from
        // before the very first read so no registration of the same
        // deterministic id can publish a replacement generation into the
        // window this call walks through — see
        // [`wallet_lifecycle_serial`](PlatformWalletManager::wallet_lifecycle_serial)
        // for why retirement alone cannot cover it.
        //
        // Taken before `deactivate` (which takes the retired manager's
        // `status_persist_serial`) and before either map lock, matching
        // the documented outermost-first order.
        let _lifecycle = self.lock_wallet_lifecycle_serial().await;

        // Retire the asset-lock manager first, holding no map lock. Note
        // the read guard is dropped before `deactivate` awaits.
        let existing = {
            let wallets = self.wallets.read().await;
            wallets.get(wallet_id).map(Arc::clone)
        };
        if let Some(wallet) = &existing {
            wallet.asset_locks().deactivate().await;
        }

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

        // Test-only pause point: the window a replacement generation
        // could be published into. Consumed on arrival so a nested
        // removal cannot re-park on it.
        #[cfg(test)]
        {
            let gate = self
                .remove_pre_detach_gate
                .lock()
                .expect("remove pre-detach gate mutex")
                .take();
            if let Some(gate) = gate {
                gate.arrived.notify_one();
                gate.release.notified().await;
            }
        }

        // Detach the handle — but only the generation we actually
        // retired above. `wallet_lifecycle_serial` already guarantees no
        // replacement can have been published since, so a mismatch here
        // means that invariant was broken by a future caller mutating
        // `wallets` outside a lifecycle transition. Detaching blindly in
        // that case is the damaging outcome (a live wallet vanishes from
        // the map while staying registered in `wallet_manager`), so
        // leave the current entry alone and hand back the generation
        // this call retired.
        let Some(retired) = existing else {
            // Never present in `wallets`. Idempotency contract: report
            // `WalletNotFound`, having still cleaned up the shared
            // `WalletManager` entry above.
            return Err(PlatformWalletError::WalletNotFound(hex::encode(wallet_id)));
        };
        {
            let mut wallets = self.wallets.write().await;
            match wallets.get(wallet_id) {
                Some(current) if Arc::ptr_eq(current, &retired) => {
                    wallets.remove(wallet_id);
                }
                Some(_) => {
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        "remove_wallet: a different wallet generation was published \
                         under this id mid-removal — leaving it registered rather \
                         than detaching a wallet this call never retired"
                    );
                }
                None => {}
            }
        }
        let removed = retired;

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

/// Cross-instance lifecycle: an `AssetLockManager` handle retained across
/// `remove_wallet` must not become live against the replacement wallet a
/// same-mnemonic re-import installs under the same deterministic id.
#[cfg(test)]
mod retained_asset_lock_manager_tests {
    use std::sync::{Arc, Mutex};

    use dashcore::OutPoint;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use key_wallet::Network;

    use crate::changeset::{
        AssetLockEntry, ClientStartState, PersistenceError, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector. Deterministic, which is
    // the whole point here: re-importing it yields the SAME wallet id.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// Records every changeset so the test can assert what a retired
    /// handle did — and did not — push into the SHARED persistence
    /// pipeline the replacement wallet also writes through.
    #[derive(Default)]
    struct CapturingPersister {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl CapturingPersister {
        /// The durable asset-lock row for `out_point` after replaying
        /// every stored round in arrival order (last-write-wins per
        /// outpoint; a `removed` entry deletes it). Same replay model as
        /// the asset-lock module's own persistence-order tests.
        fn durable_asset_lock(&self, out_point: &OutPoint) -> Option<AssetLockEntry> {
            let stored = self.stored.lock().expect("capturing persister mutex");
            let mut row: Option<AssetLockEntry> = None;
            for cs in stored.iter() {
                let Some(al) = cs.asset_locks.as_ref() else {
                    continue;
                };
                if let Some(entry) = al.asset_locks.get(out_point) {
                    row = Some(entry.clone());
                }
                if al.removed.contains(out_point) {
                    row = None;
                }
            }
            row
        }

        /// Count of rounds carrying an asset-lock sub-changeset. Wallet
        /// registration queues plenty of other rounds, so the lifecycle
        /// assertions filter on this rather than the total.
        fn asset_lock_rounds(&self) -> usize {
            self.stored
                .lock()
                .expect("capturing persister mutex")
                .iter()
                .filter(|cs| cs.asset_locks.is_some())
                .count()
        }
    }

    impl PlatformWalletPersistence for CapturingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("capturing persister mutex")
                .push(changeset);
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

    /// A synthetic tracked lock. The lifecycle assertions are about which
    /// manager may mutate the row, not about how it was funded, so the
    /// transaction can be an empty one — nothing here broadcasts.
    fn tracked_lock(out_point: OutPoint, status: AssetLockStatus) -> TrackedAssetLock {
        TrackedAssetLock {
            out_point,
            transaction: dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount: 1_000_000,
            status,
            proof: None,
        }
    }

    /// Regression: `remove_wallet` must retire the wallet's
    /// `AssetLockManager`, so a handle retained across removal cannot
    /// mutate or persist the state of the wallet a re-import installs
    /// under the same id.
    ///
    /// `wallet_id` is deterministic in (seed, network) and the FFI parks
    /// `Arc<AssetLockManager<_>>` clones in its own handle storage, which
    /// outlive `platform_wallet_manager_remove_wallet`. The manager
    /// resolves everything through the shared `WalletManager` by
    /// `wallet_id` alone, so before this fix the old handle silently
    /// re-attached to the replacement `PlatformWalletInfo` — and the two
    /// managers then mutated and persisted the same rows under *different*
    /// `status_persist_serial` mutexes, reintroducing across instances the
    /// snapshot reordering serialization fixes within one.
    ///
    /// The `untrack` arm is the sharpest: its changeset's `removed` set
    /// DELETES the durable row, so a stale handle taking that path would
    /// erase an asset lock the replacement wallet had just tracked.
    #[tokio::test]
    async fn retained_asset_lock_manager_cannot_touch_a_reimported_wallet() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(CapturingPersister::default());
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            event_handler,
        ));

        let network = Network::Testnet;
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let seed_bytes = mnemonic.to_seed("");

        // `Some(0)` skips the SPV birth-height lookup, so nothing here
        // consults SPV or the network.
        let original = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("first create should succeed");
        let wallet_id = original.wallet_id();
        let retained = Arc::clone(original.asset_locks());

        // Negative control: while the wallet is registered, the very
        // operations asserted-refused below succeed through this exact
        // handle. Without it, "the retained handle failed" could just
        // mean the test never had a working mutation path.
        let control_out_point = OutPoint::null();
        retained
            .track_asset_lock(tracked_lock(control_out_point, AssetLockStatus::Built))
            .await
            .expect("a live manager must be able to track");
        assert_eq!(
            persister
                .durable_asset_lock(&control_out_point)
                .expect("the control row must be durable")
                .status,
            AssetLockStatus::Built,
        );

        // Remove the wallet, then re-import the SAME mnemonic/network.
        manager
            .remove_wallet(&wallet_id)
            .await
            .expect("remove should return the wallet");
        let replacement = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("re-import of the same mnemonic should succeed");

        assert_eq!(
            replacement.wallet_id(),
            wallet_id,
            "the re-import must reuse the deterministic id — otherwise this \
             test is not exercising the collision the fix is about"
        );
        assert!(
            !Arc::ptr_eq(replacement.asset_locks(), &retained),
            "the re-import must have built a fresh asset-lock manager with \
             its own ordering mutex — that is what makes the retained handle \
             dangerous"
        );

        // The replacement wallet tracks a row of its own. This is the
        // state a stale handle must not be able to reach.
        let replacement_out_point = OutPoint {
            txid: dashcore::Txid::from_raw_hash(dashcore::hashes::Hash::all_zeros()),
            vout: 7,
        };
        replacement
            .asset_locks()
            .track_asset_lock(tracked_lock(
                replacement_out_point,
                AssetLockStatus::Broadcast,
            ))
            .await
            .expect("the replacement's own manager must be live");

        let rounds_before_stale_attempts = persister.asset_lock_rounds();

        // Every status mutate→enqueue primitive refuses through the
        // retired handle.
        let track = retained
            .track_asset_lock(tracked_lock(control_out_point, AssetLockStatus::Built))
            .await;
        assert!(
            matches!(track, Err(PlatformWalletError::AssetLockManagerInactive(_))),
            "a retired handle must not insert rows into the replacement \
             wallet, got {track:?}"
        );

        let untrack = retained.untrack_asset_lock(&replacement_out_point).await;
        assert!(
            matches!(
                untrack,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a retired handle must not untrack the replacement wallet's row — \
             the changeset's `removed` set would DELETE it, got {untrack:?}"
        );

        let advance = retained
            .advance_asset_lock_status(&replacement_out_point, AssetLockStatus::ChainLocked, None)
            .await;
        assert!(
            matches!(
                advance,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a retired handle must not advance the replacement wallet's row, \
             got {advance:?}"
        );

        let promote = retained
            .promote_built_to_broadcast(&replacement_out_point)
            .await;
        assert!(
            matches!(
                promote,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a retired handle must not promote the replacement wallet's row, \
             got {promote:?}"
        );

        let consume = retained.consume_asset_lock(&replacement_out_point).await;
        assert!(
            matches!(
                consume,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a retired handle must not consume the replacement wallet's row, \
             got {consume:?}"
        );

        // Neither in-memory nor durable replacement state moved.
        {
            let wm = manager.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&wallet_id)
                .expect("the replacement wallet is registered");
            let row = info
                .tracked_asset_locks
                .get(&replacement_out_point)
                .expect("the replacement's row must still be tracked");
            assert_eq!(
                row.status,
                AssetLockStatus::Broadcast,
                "no refused operation may have mutated the replacement row"
            );
            assert!(
                !info.tracked_asset_locks.contains_key(&control_out_point),
                "the retired handle's re-track must not have injected a row \
                 into the replacement wallet"
            );
        }
        assert_eq!(
            persister.asset_lock_rounds(),
            rounds_before_stale_attempts,
            "no refused operation may enqueue through the SHARED persister — \
             changesets are last-write-wins and `removed` deletes rows, so a \
             single stale round is enough to corrupt the replacement wallet"
        );
        assert_eq!(
            persister
                .durable_asset_lock(&replacement_out_point)
                .expect("the replacement's durable row must survive")
                .status,
            AssetLockStatus::Broadcast,
        );

        // Idempotency of the removal path itself is unchanged: a second
        // removal of a wallet that is gone still reports WalletNotFound
        // (the FFI maps that to ok), and retiring an already-retired
        // manager is a no-op rather than a panic or a hang.
        let unknown = [0xABu8; 32];
        assert!(
            matches!(
                manager.remove_wallet(&unknown).await,
                Err(PlatformWalletError::WalletNotFound(_))
            ),
            "removing an unknown wallet must still surface WalletNotFound"
        );
        retained.deactivate().await;
    }
}

/// Cross-*generation* lifecycle: a removal and a same-mnemonic
/// re-import must not interleave, so a removal can never detach a
/// replacement wallet it did not retire.
#[cfg(test)]
mod wallet_lifecycle_serialization_tests {
    use std::sync::Arc;
    use std::time::Duration;

    use dashcore::OutPoint;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use key_wallet::Network;
    use tokio::sync::Notify;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::manager::RemovePreDetachGate;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector. Deterministic, which is
    // the whole point: re-importing it yields the SAME wallet id, so the
    // re-import genuinely collides with the removal in flight.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    #[derive(Default)]
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

    /// A synthetic tracked lock — used only to prove the replacement's
    /// asset-lock manager is still live, so how it was funded is
    /// irrelevant and nothing here broadcasts.
    fn tracked_lock(out_point: OutPoint) -> TrackedAssetLock {
        TrackedAssetLock {
            out_point,
            transaction: dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount: 1_000_000,
            status: AssetLockStatus::Built,
            proof: None,
        }
    }

    /// Regression: `remove_wallet` must not detach a wallet generation it
    /// never retired.
    ///
    /// `remove_wallet` is three separately-locked steps — retire the
    /// asset-lock manager, drop the shared `WalletManager` entry, detach
    /// from `wallets`. The moment the middle step lands, the
    /// deterministic `wallet_id` is free, so a concurrent same-mnemonic
    /// `create_wallet_from_seed_bytes` legitimately succeeds and
    /// publishes a *replacement* generation into `wallets`. The
    /// unqualified `wallets.remove(wallet_id)` that followed then took
    /// the replacement out and returned it as the wallet it had removed,
    /// leaving a live wallet registered in `wallet_manager` but absent
    /// from `wallets` — invisible to the balance handler and every sync
    /// coordinator, and un-removable, because the next `remove_wallet`
    /// takes the `WalletNotFound` arm.
    ///
    /// `AssetLockManager::deactivate` cannot cover this: it is
    /// per-instance, and the replacement's manager is a different
    /// instance. Retirement makes a stale handle harmless; it says
    /// nothing about which generation owns the map entry.
    ///
    /// The rendezvous is on an arrival signal, never a sleep. With the
    /// removal parked in the window, exactly one of two states must be
    /// observed: the re-import PUBLISHED a second generation (the
    /// unserialized behavior — the bug), or the re-import is QUEUED on
    /// `wallet_lifecycle_serial` (the fix). A sleep distinguished
    /// neither, since "no replacement yet" could just mean the re-import
    /// had not been scheduled.
    #[tokio::test]
    async fn removal_cannot_detach_a_replacement_registered_mid_removal() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(NoopPersister),
            event_handler,
        ));

        let network = Network::Testnet;
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let seed_bytes = mnemonic.to_seed("");

        // `Some(0)` skips the SPV birth-height lookup, so nothing here
        // consults SPV or the network.
        let original = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("first create should succeed");
        let wallet_id = original.wallet_id();

        // 1. Park a removal between the `WalletManager` drop and the
        //    `wallets` detach — the window in which the id is free.
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *manager
            .remove_pre_detach_gate
            .lock()
            .expect("remove pre-detach gate mutex") = Some(RemovePreDetachGate {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        });

        let manager_remover = Arc::clone(&manager);
        let remover = tokio::spawn(async move { manager_remover.remove_wallet(&wallet_id).await });
        arrived.notified().await;

        // 2. Re-import the SAME mnemonic while the removal is parked.
        //    Runs in its own task: with the fix it BLOCKS on
        //    `wallet_lifecycle_serial` until the removal completes, so
        //    awaiting it inline would deadlock against the release below.
        let manager_reimporter = Arc::clone(&manager);
        let reimporter = tokio::spawn(async move {
            manager_reimporter
                .create_wallet_from_seed_bytes(
                    network,
                    &seed_bytes,
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
        });

        let mut published_mid_removal = false;
        let mut queued = false;
        for _ in 0..2_000 {
            // A *different* Arc under the same id means a replacement
            // generation was published; the original is still mapped at
            // this point, so identity — not presence — is the signal.
            published_mid_removal = manager
                .wallets
                .read()
                .await
                .get(&wallet_id)
                .is_some_and(|current| !Arc::ptr_eq(current, &original));
            queued = manager
                .wallet_lifecycle_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1;
            if published_mid_removal || queued {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            published_mid_removal || queued,
            "timed out: the re-import neither published nor reached the \
             lifecycle boundary — the test never exercised the race"
        );
        assert!(
            !published_mid_removal,
            "a re-import published a replacement generation while a removal \
             was mid-flight; the removal's detach would take it back out"
        );
        assert!(
            queued,
            "the re-import must come to rest on wallet_lifecycle_serial while \
             the removal holds it — otherwise the two transitions can still \
             interleave"
        );

        // 3. Release the removal, then let the re-import complete.
        release.notify_one();
        let removed = remover
            .await
            .expect("remover task joined")
            .expect("the removal must return the wallet it retired");
        let replacement = reimporter
            .await
            .expect("reimporter task joined")
            .expect("the re-import must succeed once the removal is done");

        assert!(
            Arc::ptr_eq(&removed, &original),
            "the removal must hand back the generation it retired, not \
             whatever happened to be mapped when it reached the detach"
        );
        assert_eq!(
            replacement.wallet_id(),
            wallet_id,
            "the re-import must reuse the deterministic id — otherwise this \
             test is not exercising the collision the fix is about"
        );

        // The replacement is whole: mapped in `wallets`, registered in
        // the shared `WalletManager`, and driving a live asset-lock
        // manager.
        {
            let wallets = manager.wallets.read().await;
            let mapped = wallets
                .get(&wallet_id)
                .expect("the replacement must stay published in `wallets`");
            assert!(
                Arc::ptr_eq(mapped, &replacement),
                "`wallets` must map the id to the replacement generation"
            );
        }
        assert!(
            manager
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&wallet_id)
                .is_some(),
            "the replacement must stay registered in the shared WalletManager"
        );
        replacement
            .asset_locks()
            .track_asset_lock(tracked_lock(OutPoint::null()))
            .await
            .expect("the replacement's own asset-lock manager must be live");

        // And the retired generation stays retired.
        assert!(
            matches!(
                removed
                    .asset_locks()
                    .track_asset_lock(tracked_lock(OutPoint::null()))
                    .await,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "the removed generation's asset-lock manager must remain retired"
        );
    }
}
