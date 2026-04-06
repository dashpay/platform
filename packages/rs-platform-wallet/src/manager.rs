//! Multi-wallet manager with SPV coordination.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

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
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    pub fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let wallets = Arc::new(RwLock::new(BTreeMap::new()));
        let spv = SpvRuntime::new(Arc::clone(&wallets), event_tx.clone());
        Self {
            sdk,
            wallets,
            event_tx,
            spv,
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

    /// Get a clone of the event broadcast sender.
    ///
    /// Pass this to [`PlatformWallet::from_wallet_and_info_with_event_tx`]
    /// when creating wallets that should share the manager's event channel,
    /// so their `AssetLockManager` can subscribe to SPV events.
    pub fn event_tx(&self) -> broadcast::Sender<PlatformWalletEvent> {
        self.event_tx.clone()
    }

    /// Add a wallet to the manager. Returns a clone for the caller.
    pub async fn add_wallet(
        &self,
        wallet: PlatformWallet,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        let wallet = Arc::new(wallet);
        let wallet_id = wallet.wallet_id();
        let mut wallets = self.wallets.write().await;
        if wallets.contains_key(&wallet_id) {
            return Err(PlatformWalletError::WalletAlreadyExists(hex::encode(
                wallet_id,
            )));
        }
        let cloned = wallet.clone();
        wallets.insert(wallet_id, wallet);
        self.spv.notify_wallets_changed();
        Ok(cloned)
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
