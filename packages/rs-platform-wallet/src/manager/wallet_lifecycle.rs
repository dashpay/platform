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
    AccountAddressPoolEntry, AccountRegistrationEntry, PersistenceError, PlatformWalletChangeSet,
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

/// Total attempts (initial + retries) for a transient-classified persister
/// operation on the wallet-registration path. Small on purpose: this runs
/// inline while creating a wallet, not as a background job — a lock blip
/// should be ridden out in well under a second, and a genuinely stuck
/// backend must still surface promptly.
const PERSIST_RETRY_MAX_ATTEMPTS: u32 = 4;

/// Backoff before the first retry; doubles on each subsequent attempt.
const PERSIST_RETRY_INITIAL_BACKOFF: std::time::Duration = std::time::Duration::from_millis(20);

/// Ceiling for the doubling backoff so registration latency stays bounded
/// (worst case with the constants above: 20 + 40 + 80 ≈ 140 ms).
const PERSIST_RETRY_MAX_BACKOFF: std::time::Duration = std::time::Duration::from_millis(200);

/// Retry a synchronous persister operation while it fails *transiently*,
/// using bounded exponential backoff.
///
/// `op` runs once, then re-runs after a backoff sleep for as long as it
/// returns a [`PersistenceError`] whose
/// [`is_transient()`](PersistenceError::is_transient) is true, up to
/// [`PERSIST_RETRY_MAX_ATTEMPTS`]. A fatal error (or success) returns
/// immediately — a fatal failure never retries. The sleep is async so it
/// yields the Tokio worker instead of spinning the CPU, which is exactly
/// what the storage layer's `FlushRetryable` contract asks callers to do.
async fn retry_transient<T, F>(mut op: F) -> Result<T, PersistenceError>
where
    F: FnMut() -> Result<T, PersistenceError>,
{
    let mut backoff = PERSIST_RETRY_INITIAL_BACKOFF;
    let mut attempt: u32 = 1;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(e) if e.is_transient() && attempt < PERSIST_RETRY_MAX_ATTEMPTS => {
                tracing::debug!(
                    attempt,
                    max_attempts = PERSIST_RETRY_MAX_ATTEMPTS,
                    backoff_ms = backoff.as_millis() as u64,
                    error = %e,
                    "transient persister failure — backing off before retry"
                );
                tokio::time::sleep(backoff).await;
                backoff = backoff.saturating_mul(2).min(PERSIST_RETRY_MAX_BACKOFF);
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
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

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet, birth_height);

        let balance = Arc::new(WalletBalance::new());

        // Snapshot per-account xpubs and address-pool entries BEFORE
        // the wallet / managed-info are moved into insert_wallet. The
        // persister sees everything needed to rebuild the wallet
        // external-signable (via `Wallet::new_external_signable`) plus populate
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
                // The BLS operator pool extends on demand from the
                // account xpub (non-hardened `ckd_pub`, no seed), so it
                // needs no pre-derived batch.
                derived_platform_node_keys: Vec::new(),
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
            // capturing the public parts now lets the Node Keys screen list
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
            provider_key_account_registrations.push(ProviderKeyAccountEntry {
                account_type: key_wallet::account::AccountType::ProviderPlatformKeys,
                extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                    eddsa.ed25519_public_key.clone(),
                ),
                derived_platform_node_keys,
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

        // Persist the registration changeset, riding out a *transient*
        // backend blip (e.g. `SQLITE_BUSY`) with bounded exponential backoff
        // before giving up. On a transient `store` failure the persister
        // restores the buffered changeset (its documented contract), so the
        // retries re-drive that same write via `flush` — no re-merge, no
        // double-count: the first attempt hands the changeset over, later
        // attempts flush what the buffer preserved. A fatal error is not
        // retried and fails fast. Either way the typed `PersistenceError`
        // (and its transient/fatal classification) is preserved for the
        // caller instead of being flattened to a string.
        let mut changeset_slot = Some(registration_changeset);
        let store_result = retry_transient(|| match changeset_slot.take() {
            Some(cs) => self.persister.store(wallet_id, cs),
            None => self.persister.flush(wallet_id),
        })
        .await;
        if let Err(e) = store_result {
            tracing::error!(
                wallet_id = %hex::encode(wallet_id),
                transient = e.is_transient(),
                error = %e,
                "failed to persist wallet registration changeset after retries"
            );
            let mut wm = self.wallet_manager.write().await;
            if let Err(remove_err) = wm.remove_wallet(&wallet_id) {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %remove_err,
                    "rollback: remove_wallet failed while unwinding a failed wallet registration"
                );
            }
            return Err(PlatformWalletError::PersisterStore(e));
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
        // Retry a transient load blip the same way as the store above; a
        // load is an idempotent read, so re-reading after a lock blip is
        // safe. `load_persisted()` returns the typed `PersistenceError` this
        // rehydration boundary is built around, routed through the
        // dedicated `PersisterLoad` variant so its retry classification
        // survives to the caller.
        let load_result = retry_transient(|| platform_wallet.load_persisted()).await;
        let crate::changeset::ClientStartState {
            mut platform_addresses,
            wallets: _,
            #[cfg(feature = "shielded")]
                shielded: _,
        } = match load_result {
            Ok(state) => state,
            Err(e) => {
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    transient = e.is_transient(),
                    error = %e,
                    "failed to load persisted wallet state after retries"
                );
                let mut wm = self.wallet_manager.write().await;
                if let Err(remove_err) = wm.remove_wallet(&wallet_id) {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %remove_err,
                        "rollback: remove_wallet failed while unwinding a failed wallet setup"
                    );
                }
                return Err(PlatformWalletError::PersisterLoad(e));
            }
        };

        if let Some(persisted) = platform_addresses.remove(&wallet_id) {
            if let Err(e) = platform_wallet
                .platform()
                .initialize_from_persisted(persisted)
                .await
            {
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "failed to restore persisted platform-address state"
                );
                let mut wm = self.wallet_manager.write().await;
                if let Err(remove_err) = wm.remove_wallet(&wallet_id) {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %remove_err,
                        "rollback: remove_wallet failed while unwinding a failed wallet setup"
                    );
                }
                // `initialize_from_persisted` already returns a typed
                // `PlatformWalletError`; wrap (boxed) rather than stringify so
                // its concrete variant and source chain survive.
                return Err(PlatformWalletError::PersisterRestore(Box::new(e)));
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
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
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

        let removed = {
            let mut wallets = self.wallets.write().await;
            wallets
                .remove(wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?
        };

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

#[cfg(test)]
mod persist_retry_tests {
    //! Registration-path persistence: transient-error retry (QA-001) and
    //! typed error classification across the boundary (QA-002 / QA-005).

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    fn transient() -> PersistenceError {
        PersistenceError::backend_with_kind(
            PersistenceErrorKind::Transient,
            "simulated SQLITE_BUSY",
        )
    }

    fn fatal() -> PersistenceError {
        PersistenceError::backend_with_kind(PersistenceErrorKind::Fatal, "simulated corruption")
    }

    /// Persister whose `store` / `flush` / `load` outcomes are scripted so
    /// the registration retry path can be driven deterministically. Models
    /// the real contract: a transient `store` failure preserves the
    /// changeset in the buffer, so the retry re-drives the write through
    /// `flush`.
    #[derive(Default)]
    struct FaultyPersister {
        store_calls: AtomicUsize,
        flush_calls: AtomicUsize,
        load_calls: AtomicUsize,
        /// The first `store` fails transiently (buffer preserved for retry).
        store_transient_first: bool,
        /// Every `store` fails fatally (must NOT retry).
        store_fatal: bool,
        /// Number of leading `flush` calls that fail transiently before Ok.
        flush_transient_failures: usize,
        /// Number of leading `load` calls that fail transiently before Ok.
        load_transient_failures: usize,
        /// Every `load` fails fatally (must NOT retry).
        load_fatal: bool,
    }

    impl PlatformWalletPersistence for FaultyPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let n = self.store_calls.fetch_add(1, Ordering::SeqCst);
            if self.store_fatal {
                return Err(fatal());
            }
            if self.store_transient_first && n == 0 {
                return Err(transient());
            }
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let n = self.flush_calls.fetch_add(1, Ordering::SeqCst);
            if n < self.flush_transient_failures {
                Err(transient())
            } else {
                Ok(())
            }
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            let n = self.load_calls.fetch_add(1, Ordering::SeqCst);
            if self.load_fatal {
                return Err(fatal());
            }
            if n < self.load_transient_failures {
                return Err(transient());
            }
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    fn make_manager(
        persister: Arc<FaultyPersister>,
    ) -> Arc<PlatformWalletManager<FaultyPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        Arc::new(PlatformWalletManager::new(sdk, persister, event_handler))
    }

    fn seed_bytes() -> [u8; 64] {
        Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid test mnemonic")
            .to_seed("")
    }

    /// `Some(0)` skips the SPV-tip birth-height lookup so the test never
    /// consults SPV.
    async fn register(
        manager: &PlatformWalletManager<FaultyPersister>,
    ) -> Result<(), PlatformWalletError> {
        manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed_bytes(),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .map(|_| ())
    }

    /// QA-001: a transient `store` failure is ridden out — the persister
    /// buffers the changeset, the retry re-drives it via `flush`, and
    /// registration succeeds instead of aborting.
    #[tokio::test]
    async fn transient_store_failure_is_retried_and_succeeds() {
        let persister = Arc::new(FaultyPersister {
            store_transient_first: true,
            flush_transient_failures: 1, // one transient flush, then Ok
            ..Default::default()
        });
        let manager = make_manager(Arc::clone(&persister));

        register(&manager)
            .await
            .expect("registration must succeed after retrying the transient store");

        // store attempted once; flush retried twice (fail, then succeed).
        assert_eq!(persister.store_calls.load(Ordering::SeqCst), 1);
        assert_eq!(persister.flush_calls.load(Ordering::SeqCst), 2);
    }

    /// QA-001 / QA-002: a fatal `store` failure fails fast — no retry — and
    /// surfaces as the typed `PersisterStore` whose inner classification is
    /// non-transient.
    #[tokio::test]
    async fn fatal_store_failure_fails_fast_without_retry() {
        let persister = Arc::new(FaultyPersister {
            store_fatal: true,
            ..Default::default()
        });
        let manager = make_manager(Arc::clone(&persister));

        let err = register(&manager)
            .await
            .expect_err("a fatal store must abort registration");

        match err {
            PlatformWalletError::PersisterStore(pe) => assert!(
                !pe.is_transient(),
                "a fatal store must carry non-transient classification"
            ),
            other => panic!("expected PersisterStore, got {other:?}"),
        }
        assert_eq!(persister.store_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            persister.flush_calls.load(Ordering::SeqCst),
            0,
            "a fatal store must not be retried via flush"
        );
    }

    /// QA-001 bounds + QA-002: a store that stays transient exhausts the
    /// bounded retry budget and returns the typed `PersisterStore` still
    /// carrying transient classification (distinguishable from the fatal
    /// case above).
    #[tokio::test]
    async fn persistently_transient_store_exhausts_bounded_retries() {
        let persister = Arc::new(FaultyPersister {
            store_transient_first: true,
            flush_transient_failures: usize::MAX, // never recovers
            ..Default::default()
        });
        let manager = make_manager(Arc::clone(&persister));

        let err = register(&manager)
            .await
            .expect_err("registration must fail once the retry budget is spent");

        match err {
            PlatformWalletError::PersisterStore(pe) => assert!(
                pe.is_transient(),
                "an exhausted-but-transient store must stay classified transient"
            ),
            other => panic!("expected PersisterStore, got {other:?}"),
        }
        // 1 store + 3 flush retries == 4 total attempts (the budget).
        assert_eq!(persister.store_calls.load(Ordering::SeqCst), 1);
        assert_eq!(persister.flush_calls.load(Ordering::SeqCst), 3);
    }

    /// QA-001: a transient `load` blip during rehydration is retried (an
    /// idempotent read), so registration succeeds.
    #[tokio::test]
    async fn transient_load_failure_is_retried_and_succeeds() {
        let persister = Arc::new(FaultyPersister {
            load_transient_failures: 1,
            ..Default::default()
        });
        let manager = make_manager(Arc::clone(&persister));

        register(&manager)
            .await
            .expect("registration must succeed after retrying the transient load");

        assert_eq!(persister.store_calls.load(Ordering::SeqCst), 1);
        assert_eq!(persister.flush_calls.load(Ordering::SeqCst), 0);
        assert_eq!(persister.load_calls.load(Ordering::SeqCst), 2);
    }

    /// QA-002: a fatal `load` fails fast and surfaces as the typed
    /// `PersisterLoad` — never the flattened `WalletCreation(String)`.
    #[tokio::test]
    async fn fatal_load_failure_surfaces_as_persister_load() {
        let persister = Arc::new(FaultyPersister {
            load_fatal: true,
            ..Default::default()
        });
        let manager = make_manager(Arc::clone(&persister));

        let err = register(&manager)
            .await
            .expect_err("a fatal load must abort registration");

        match err {
            PlatformWalletError::PersisterLoad(pe) => assert!(!pe.is_transient()),
            other => panic!("expected PersisterLoad, got {other:?}"),
        }
        assert_eq!(
            persister.load_calls.load(Ordering::SeqCst),
            1,
            "a fatal load must not be retried"
        );
    }

    /// QA-002 / QA-005: the typed persister-phase variants preserve retry
    /// classification, enable structural matching, and keep the `#[source]`
    /// chain instead of flattening to a string.
    #[test]
    fn typed_variants_preserve_classification_matching_and_source() {
        use std::error::Error;

        let store_err = PlatformWalletError::PersisterStore(transient());
        match &store_err {
            PlatformWalletError::PersisterStore(pe) => assert!(pe.is_transient()),
            other => panic!("expected PersisterStore, got {other:?}"),
        }
        assert!(
            store_err.source().is_some(),
            "PersisterStore must expose its PersistenceError source"
        );

        let load_err = PlatformWalletError::PersisterLoad(fatal());
        match &load_err {
            PlatformWalletError::PersisterLoad(pe) => assert!(!pe.is_transient()),
            other => panic!("expected PersisterLoad, got {other:?}"),
        }
        assert!(load_err.source().is_some());

        // The restore variant wraps a typed inner error; structural matching
        // must recover the concrete inner variant, not an opaque string.
        let restore_err =
            PlatformWalletError::PersisterRestore(Box::new(PlatformWalletError::WalletLocked));
        assert!(restore_err.source().is_some());
        match restore_err {
            PlatformWalletError::PersisterRestore(inner) => {
                assert!(matches!(*inner, PlatformWalletError::WalletLocked));
            }
            other => panic!("expected PersisterRestore, got {other:?}"),
        }
    }
}
