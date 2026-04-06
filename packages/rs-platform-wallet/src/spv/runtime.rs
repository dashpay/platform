//! SPV client runtime — manages the DashSpvClient lifecycle.
//!
//! Extracted from `PlatformWalletManager` so the same SPV coordination can be
//! used both with a multi-wallet manager and with a standalone `PlatformWallet`.
//!
//! Asset-lock finality tracking (IS/CL proof waiting) is handled by
//! `AssetLockManager` directly — it subscribes to the shared event channel.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::{ClientConfig, DashSpvClient};

use key_wallet_manager::WalletInterface;

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::event_forwarder::SpvEventForwarder;
use crate::spv::wallet_adapter::SpvWalletAdapter;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

type SpvClient =
    DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>;

/// SPV client runtime — owns the `DashSpvClient` and tracks sync height.
///
/// Holds references to the wallets collection and event channel at construction
/// time, so callers just need `start(config)` / `stop()`.
///
/// Asset-lock finality tracking (InstantLock / ChainLock waiting) is handled
/// directly by `AssetLockManager` via SPV event subscriptions — the runtime
/// only drives SPV sync and forwards events.
pub struct SpvRuntime {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    adapter: Arc<RwLock<SpvWalletAdapter>>,
    client: RwLock<Option<SpvClient>>,
}

impl SpvRuntime {
    /// Create a new SPV runtime bound to a wallets collection and event channel.
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        let adapter = Arc::new(RwLock::new(SpvWalletAdapter::new(wallets)));
        Self {
            event_tx,
            adapter,
            client: RwLock::new(None),
        }
    }

    /// Current synced height.
    pub fn synced_height(&self) -> u32 {
        self.adapter
            .try_read()
            .map(|a| a.synced_height())
            .unwrap_or(0)
    }

    /// Signal that the wallet set changed (added/removed).
    /// SPV will rebuild the bloom filter on the next tick.
    pub fn notify_wallets_changed(&self) {
        if let Ok(adapter) = self.adapter.try_read() {
            adapter.monitor_revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Start SPV sync.
    pub async fn start(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        {
            let running = self.client.read().await;
            if running.is_some() {
                return Err(PlatformWalletError::SpvAlreadyRunning);
            }
        }

        let forwarder = SpvEventForwarder::new(self.event_tx.clone());

        let network_manager = PeerNetworkManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        let storage_manager = DiskStorageManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let spv_client = DashSpvClient::new(
            config,
            network_manager,
            storage_manager,
            Arc::clone(&self.adapter),
            Arc::new(forwarder),
        )
        .await
        .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let mut client = self.client.write().await;
        *client = Some(spv_client);

        Ok(())
    }

    /// Stop SPV sync gracefully.
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        let mut client = self.client.write().await;
        if let Some(c) = client.take() {
            c.stop()
                .await
                .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        }
        Ok(())
    }

}

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("synced_height", &self.synced_height())
            .finish()
    }
}
