//! Multi-wallet manager with SPV coordination.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;

use crate::changeset::{Merge, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Multi-wallet coordinator with SPV sync and event broadcasting.
///
/// Mirrors the role of `key-wallet-manager`'s `WalletManager` for the Core
/// layer, but at the Platform level: manages multiple [`PlatformWallet`]
/// instances, coordinates SPV block/filter sync via [`SpvRuntime`], and
/// broadcasts unified [`PlatformWalletEvent`]s (sync progress, network
/// changes, wallet updates, finality proofs) to subscribers.
///
/// Each managed [`PlatformWallet`] shares its underlying `Wallet` and
/// `ManagedWalletInfo` with the SPV adapter through `Arc<RwLock<…>>`,
/// so balance and UTXO updates from SPV are immediately visible to all
/// wallet operations.
pub struct PlatformWalletManager {
    sdk: Arc<dash_sdk::Sdk>,
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    spv: SpvRuntime,
    persister: Arc<dyn PlatformWalletPersistence>,
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    pub fn new(sdk: Arc<dash_sdk::Sdk>, persister: Arc<dyn PlatformWalletPersistence>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let wallets = Arc::new(RwLock::new(BTreeMap::new()));
        let spv = SpvRuntime::new(Arc::clone(&wallets), event_tx.clone());
        Self {
            sdk,
            wallets,
            event_tx,
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

    /// Subscribe to platform wallet events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent> {
        self.event_tx.subscribe()
    }

    /// Create a PlatformWallet from raw seed bytes, initialize persisted
    /// state, register it with the manager and return an `Arc` handle.
    ///
    /// The wallet is created with the manager's shared event channel so
    /// SPV events (InstantLock / ChainLock) reach the `AssetLockManager`.
    /// Persisted state (transactions, UTXOs, balances, identities) is loaded
    /// from the shared persister and applied before the wallet is registered,
    /// so the returned wallet is fully configured and ready for use.
    pub async fn create_wallet_from_seed_bytes(
        &self,
        network: Network,
        seed_bytes: [u8; 64],
        options: WalletAccountCreationOptions,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::wallet::Wallet;

        let wallet = Wallet::from_seed_bytes(seed_bytes, network, options).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from seed bytes: {}",
                e
            ))
        })?;
        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        let wallet_id = wallet_info.wallet_id;

        let platform_wallet = PlatformWallet::new(
            Arc::clone(&self.sdk),
            wallet,
            wallet_info,
            self.event_tx.clone(),
            Arc::clone(&self.persister),
        );

        // Load persisted state and apply it to the in-memory wallet.
        let changeset = self.persister.load(wallet_id).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted wallet state: {}",
                e
            ))
        })?;
        if !changeset.is_empty() {
            platform_wallet.apply(&changeset);
        }

        let platform_wallet = Arc::new(platform_wallet);

        // Register with the manager so SPV processes this wallet.
        let mut wallets = self.wallets.write().await;
        wallets.insert(wallet_id, Arc::clone(&platform_wallet));
        drop(wallets);
        self.spv.notify_wallets_changed();

        Ok(platform_wallet)
    }

    /// Remove a wallet from the manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let mut wallets = self.wallets.write().await;
        let removed = wallets
            .remove(wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
        self.spv.notify_wallets_changed();
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
