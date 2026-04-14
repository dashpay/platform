//! Multi-wallet manager with SPV coordination.

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use key_wallet_manager::WalletManager;

use crate::changeset::{Merge, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::spv::SpvRuntime;
use crate::wallet::asset_lock::LockNotifyHandler;
use crate::wallet::core::{BalanceUpdateHandler, WalletBalance};
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Multi-wallet coordinator with SPV sync and event handling.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// [`PlatformEventHandler`]s by reference (no cloning).
pub struct PlatformWalletManager {
    sdk: Arc<dash_sdk::Sdk>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Map of registered wallets. Held in an `Arc` so the
    /// `BalanceUpdateHandler` can hold a clone and look up wallets to
    /// update their lock-free balance atomics from event-handler
    /// context, without touching the SPV-contended `wallet_manager`
    /// lock.
    wallets: Arc<RwLock<std::collections::BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    /// Notified on InstantLock / ChainLock events for `AssetLockManager` waiters.
    lock_notify: Arc<Notify>,
    spv: Arc<SpvRuntime>,
    persister: Arc<dyn PlatformWalletPersistence>,
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    ///
    /// `app_handler` receives all SPV and platform events by reference.
    /// Internally, a `LockNotifyHandler` is also registered to wake
    /// `AssetLockManager` async waiters on lock events.
    pub fn new(
        sdk: Arc<dash_sdk::Sdk>,
        persister: Arc<dyn PlatformWalletPersistence>,
        app_handler: Arc<dyn PlatformEventHandler>,
    ) -> Self {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(sdk.network)));
        let wallets = Arc::new(RwLock::new(std::collections::BTreeMap::new()));
        let lock_notify = Arc::new(Notify::new());

        // Build handler list: app handler + internal handlers.
        // BalanceUpdateHandler holds a clone of the wallets map (a
        // separate lock from wallet_manager) so it can look up
        // PlatformWallets and write to their lock-free balance
        // atomics from broadcast-handler context without contending
        // with SPV's write lock.
        let lock_handler = Arc::new(LockNotifyHandler::new(Arc::clone(&lock_notify)));
        let balance_handler = Arc::new(BalanceUpdateHandler::new(Arc::clone(&wallets)));
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            app_handler,
            lock_handler,
            balance_handler,
        ]));

        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        Self {
            sdk,
            wallet_manager,
            wallets,
            lock_notify,
            spv,
            persister,
        }
    }

    /// The SDK instance.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Access the SPV runtime for sync control.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv
    }

    // TODO: We can delete it and use core().broadcast() ? don't delete this todo
    /// Broadcast a transaction via SPV P2P peers.
    pub async fn broadcast_transaction(
        &self,
        tx: &dashcore::Transaction,
    ) -> Result<(), PlatformWalletError> {
        self.spv.broadcast_transaction(tx).await
    }

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

        // Build the PlatformWallet handle.
        let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
            &self.spv,
        )));

        let platform_wallet = PlatformWallet::new(
            Arc::clone(&self.sdk),
            wallet_id,
            Arc::clone(&self.wallet_manager),
            balance,
            Arc::clone(&self.lock_notify),
            Arc::clone(&self.persister),
            broadcaster,
        );

        // Load persisted state and apply it to the in-memory wallet.
        // `apply` is async and `must_use` — `.await` it and propagate
        // any error, otherwise the future is dropped and the wallet
        // boots from empty state instead of restoring.
        let changeset = platform_wallet.load_persisted().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted wallet state: {}",
                e
            ))
        })?;
        if !changeset.is_empty() {
            platform_wallet.apply(changeset).await.map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to apply persisted wallet state: {}",
                    e
                ))
            })?;
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

    /// Get a clone of a wallet by its ID.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        let wallets = self.wallets.read().await;
        wallets.get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    pub async fn wallet_ids(&self) -> Vec<WalletId> {
        let wallets = self.wallets.read().await;
        wallets.keys().copied().collect()
    }
}
