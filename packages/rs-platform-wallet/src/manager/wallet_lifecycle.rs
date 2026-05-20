//! Wallet creation, registration, and removal on [`PlatformWalletManager`].

use std::sync::Arc;

use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, PlatformWalletChangeSet,
    PlatformWalletPersistence, WalletMetadataEntry,
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
    pub async fn create_wallet_from_mnemonic(
        &self,
        mnemonic_phrase: &str,
        network: Network,
        accounts: WalletAccountCreationOptions,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let mnemonic = parse_mnemonic_any_language(mnemonic_phrase)
            .map_err(|e| PlatformWalletError::WalletCreation(format!("Invalid mnemonic: {}", e)))?;
        let wallet = Wallet::from_mnemonic(mnemonic, network, accounts).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from mnemonic: {}",
                e
            ))
        })?;
        self.register_wallet(wallet).await
    }

    /// Create a PlatformWallet from raw seed bytes, initialize persisted
    /// state, register it with the manager and return an `Arc` handle.
    pub async fn create_wallet_from_seed_bytes(
        &self,
        network: Network,
        seed_bytes: [u8; 64],
        accounts: WalletAccountCreationOptions,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let wallet = Wallet::from_seed_bytes(seed_bytes, network, accounts).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from seed bytes: {}",
                e
            ))
        })?;
        self.register_wallet(wallet).await
    }

    /// Register a pre-built `Wallet` with the manager: insert into the
    /// `WalletManager`, build a `PlatformWallet` handle, load persisted
    /// state, and return an `Arc` to the managed wallet.
    #[allow(clippy::type_complexity)]
    async fn register_wallet(
        &self,
        wallet: Wallet,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);

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

        let platform_info = PlatformWalletInfo {
            core_wallet: wallet_info,
            balance: Arc::clone(&balance),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
        };

        // Insert into WalletManager.
        let wallet_id = {
            let mut wm = self.wallet_manager.write().await;
            wm.insert_wallet(wallet, platform_info).map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to register wallet in WalletManager: {}",
                    e
                ))
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
        // Failures are logged but don't abort wallet registration —
        // the persister is a best-effort channel, not a source of
        // truth in steady state.

        // Birth height = SPV's confirmed header tip if SPV is running,
        // otherwise 0 (caller can bump it later when SPV catches up).
        // 0 means "scan from genesis", which is safe-correct for
        // fresh wallets.
        let birth_height: u32 = self
            .spv_manager
            .sync_progress()
            .await
            .and_then(|p| p.headers().ok().map(|h| h.tip_height()))
            .unwrap_or(0);

        let mut registration_changeset = PlatformWalletChangeSet {
            wallet_metadata: Some(WalletMetadataEntry {
                network: self.sdk.network,
                birth_height,
            }),
            account_registrations: account_specs
                .iter()
                .map(|(account_type, account_xpub)| AccountRegistrationEntry {
                    account_type: *account_type,
                    account_xpub: *account_xpub,
                })
                .collect(),
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
                let _ = wm.remove_wallet(&wallet_id);
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
                let _ = wm.remove_wallet(&wallet_id);
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
            let _ = wm.remove_wallet(wallet_id);
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
