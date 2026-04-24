//! Read-only accessors on [`PlatformWalletManager`].

use std::sync::Arc;

use crate::changeset::PlatformWalletPersistence;
use crate::platform_address_sync::PlatformAddressSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// The SDK instance.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Access the SPV runtime for sync control.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv
    }

    /// Clone the `Arc<SpvRuntime>` so callers (e.g. FFI) can invoke
    /// [`SpvRuntime::spawn_in_background`] which takes `&Arc<Self>`.
    pub fn spv_arc(&self) -> Arc<SpvRuntime> {
        Arc::clone(&self.spv)
    }

    /// Access the platform-address sync coordinator.
    pub fn platform_address_sync(&self) -> &PlatformAddressSyncManager {
        &self.platform_address_sync
    }

    /// Clone the `Arc<PlatformAddressSyncManager>` so callers (e.g. FFI)
    /// can invoke [`PlatformAddressSyncManager::start`] which takes
    /// `&Arc<Self>`.
    pub fn platform_address_sync_arc(&self) -> Arc<PlatformAddressSyncManager> {
        Arc::clone(&self.platform_address_sync)
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
