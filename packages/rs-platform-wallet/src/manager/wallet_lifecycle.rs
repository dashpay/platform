//! Wallet creation, registration, and removal on [`PlatformWalletManager`].

use std::sync::Arc;

use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletBalance;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Create a PlatformWallet from a BIP39 mnemonic phrase.
    ///
    /// The mnemonic is parsed as English. For other languages or passphrases,
    /// derive the seed externally and use [`create_wallet_from_seed_bytes`].
    pub async fn create_wallet_from_mnemonic(
        &self,
        mnemonic_phrase: &str,
        network: Network,
        accounts: WalletAccountCreationOptions,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let mnemonic = Mnemonic::from_phrase(mnemonic_phrase, Language::English)
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
    async fn register_wallet(
        &self,
        wallet: Wallet,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);

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
            .map(|a| (a.account_type.clone(), a.account_xpub))
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
                let account_type = managed.account_type.to_account_type();
                let pools = managed
                    .account_type
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
            token_watched: std::collections::BTreeMap::new(),
            token_balances: std::collections::BTreeMap::new(),
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

        // Emit metadata + per-account xpubs to the persister so the
        // watch-only restore path has everything it needs on next
        // launch. Failures are logged but don't abort wallet
        // registration — the persister is a best-effort channel, not
        // a source of truth in steady state.

        // Birth height = SPV's confirmed header tip if SPV is running,
        // otherwise 0 (caller can bump it later when SPV catches up).
        // 0 means "scan from genesis", which is safe-correct for
        // fresh wallets.
        let birth_height: u32 = self
            .spv
            .sync_progress()
            .await
            .and_then(|p| p.headers().ok().map(|h| h.tip_height()))
            .unwrap_or(0);
        if let Err(e) =
            self.persister
                .store_wallet_metadata(wallet_id, self.sdk.network, birth_height)
        {
            tracing::error!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "failed to persist wallet metadata"
            );
        }

        for (account_type, account_xpub) in &account_specs {
            if let Err(e) = self
                .persister
                .store_account(wallet_id, account_type, account_xpub)
            {
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    account_type = ?account_type,
                    error = %e,
                    "failed to persist account xpub"
                );
            }
        }

        // Emit the initial address pool contents per account. Every
        // account type contributes at least one pool (external, or a
        // single `Absent` pool for degenerate types); Standard
        // accounts contribute two. Ordering within a pool is by
        // derivation index via `BTreeMap::values`.
        for (account_type, pools) in &address_snapshots {
            for (pool_type, infos) in pools {
                if infos.is_empty() {
                    continue;
                }
                if let Err(e) = self.persister.store_account_addresses(
                    wallet_id,
                    account_type,
                    *pool_type,
                    infos,
                ) {
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        account_type = ?account_type,
                        pool_type = ?pool_type,
                        error = %e,
                        "failed to persist account addresses"
                    );
                }
            }
        }

        // Build the PlatformWallet handle.
        let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
            &self.spv,
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
        let crate::changeset::ClientStartState {
            mut platform_addresses,
            wallets: _,
        } = platform_wallet.load_persisted().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted wallet state: {}",
                e
            ))
        })?;

        if let Some(persisted) = platform_addresses.remove(&wallet_id) {
            platform_wallet
                .platform()
                .initialize_from_persisted(persisted)
                .await
                .map_err(|e| {
                    PlatformWalletError::WalletCreation(format!(
                        "Failed to restore persisted platform address state: {}",
                        e
                    ))
                })?;
        } else {
            platform_wallet.platform().initialize().await;
        }

        let platform_wallet = Arc::new(platform_wallet);

        // Register the PlatformWallet handle.
        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(wallet_id, Arc::clone(&platform_wallet));
        }

        Ok(platform_wallet)
    }

    /// Remove a wallet from the manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let removed = {
            let mut wallets = self.wallets.write().await;
            wallets
                .remove(wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?
        };
        {
            let mut wm = self.wallet_manager.write().await;
            let _ = wm.remove_wallet(wallet_id);
        }
        Ok(removed)
    }
}
