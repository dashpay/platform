//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::Arc;

use tokio::sync::RwLock;

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::sync::SyncProgress;
use dash_spv::{ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient =
    DashSpvClient<WalletManager<PlatformWalletInfo>, PeerNetworkManager, DiskStorageManager>;

/// SPV client runtime — owns the `DashSpvClient` and drives sync.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// handlers by reference (no cloning).
pub struct SpvRuntime {
    event_manager: Arc<PlatformEventManager>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    client: RwLock<Option<SpvClient>>,
}
// TODO: We want it better
impl SpvRuntime {
    /// Create a new SPV runtime.
    pub fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        event_manager: Arc<PlatformEventManager>,
    ) -> Self {
        Self {
            event_manager,
            wallet_manager,
            client: RwLock::new(None),
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

        let network_manager = PeerNetworkManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        let storage_manager = DiskStorageManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        // PlatformEventManager implements `EventHandler`; pass it as the
        // sole entry in the SPV client's handler vec. Additional dyn
        // handlers can be added here if other components need to observe
        // raw SPV events directly (today everything routes through the
        // platform event manager's own handler list).
        let event_handlers: Vec<Arc<dyn EventHandler>> =
            vec![Arc::clone(&self.event_manager) as Arc<dyn EventHandler>];
        let spv_client = DashSpvClient::new(
            config,
            network_manager,
            storage_manager,
            Arc::clone(&self.wallet_manager),
            event_handlers,
        )
        .await
        .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let mut client = self.client.write().await;
        *client = Some(spv_client);

        Ok(())
    }

    /// Check whether the SPV client has been started.
    pub fn is_started(&self) -> bool {
        self.client.try_read().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Broadcast a transaction to all connected SPV peers.
    pub(crate) async fn broadcast_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;

        client
            .broadcast_transaction(tx)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        Ok(())
    }

    /// Look up a quorum public key via the SPV masternode state.
    pub async fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<[u8; 48], PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;

        let llmq_type = LLMQType::from(quorum_type as u8);
        let qh = QuorumHash::from_byte_array(quorum_hash).reverse();

        let quorum = client
            .get_quorum_at_height(height, llmq_type, qh)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        Ok(*quorum.quorum_entry.quorum_public_key.as_ref())
    }

    /// Run the SPV sync loop until calling [`stop`]. This blocks the current thread.
    pub async fn run(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        tracing::info!("SpvRuntime::run() starting client...");
        self.start(config).await?;
        tracing::info!("SpvRuntime::run() client started, entering sync loop");

        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;

        let result = client
            .run()
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()));

        drop(client_guard);
        let mut client = self.client.write().await;
        let _ = client.take();

        result
    }

    /// Best-effort: fire the background `run()` task's cancel token if one
    /// is registered. Teardown of the dash-spv client and its data-dir
    /// lockfile still happens asynchronously inside the spawned task as it
    /// unwinds to its `self.stop().await` epilogue — this method only wakes
    /// the task. Idempotent: subsequent calls (and a follow-up [`stop`])
    /// see `None` and return immediately.
    ///
    /// Designed for sync contexts where awaiting [`stop`] isn't possible —
    /// for example a `std::panic::set_hook` callback that wants to nudge the
    /// SPV task toward shutdown without blocking the panicking thread.
    ///
    /// This method does **not** guarantee the dash-spv data-dir lock has
    /// been released by the time it returns. Callers that need that
    /// guarantee (e.g. before reinitializing on the same data directory)
    /// must `await stop()` from an async context instead.
    ///
    /// Tolerates a poisoned `background_cancel` mutex — the panic-hook use
    /// case is precisely when the lock may already be poisoned, so the
    /// guard is recovered via `PoisonError::into_inner` rather than
    /// panicking again.
    pub fn cancel_background(&self) {
        if let Some(token) = self
            .background_cancel
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            token.cancel();
        }
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

    /// Spawn `run()` on the current tokio runtime and return immediately.
    ///
    /// Call [`stop`] to stop it
    pub fn spawn_in_background(self: &Arc<Self>, config: ClientConfig) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = this.run(config).await {
                tracing::warn!("SpvRuntime background run exited with error: {}", e);
            }
        });
    }

    /// Get the current sync progress.
    ///
    /// Returns `None` if the SPV client is not running.
    pub async fn sync_progress(&self) -> Option<SyncProgress> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref()?;
        Some(client.sync_progress().await)
    }

    /// Read the unix-seconds block time of the SPV header storage's
    /// current tip.
    ///
    /// Useful as a "is core producing blocks?" indicator: if this
    /// stamp stays put across multiple polls, the chain has stalled
    /// even though the local SPV client is healthy.
    ///
    /// Returns `None` if the SPV client isn't running, no headers
    /// have been stored yet, or the tip header isn't readable for
    /// any reason.
    pub async fn tip_block_time(&self) -> Option<u32> {
        use dash_spv::storage::{BlockHeaderStorage, StorageManager};

        let client_guard = self.client.read().await;
        let client = client_guard.as_ref()?;
        let storage_arc = client.storage();
        let storage = storage_arc.lock().await;
        let block_headers = StorageManager::block_headers(&*storage);
        drop(storage);
        let bh = block_headers.read().await;
        let tip = BlockHeaderStorage::get_tip(&*bh).await?;
        Some(tip.header().time)
    }

    /// Clear all persisted SPV storage (headers, filters, state).
    ///
    /// The SPV client must be running to perform this operation.
    pub async fn clear_storage(&self) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;
        client
            .clear_storage()
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))
    }

    /// Update the running SPV client's configuration.
    ///
    /// The network cannot be changed on a running client.
    pub async fn update_config(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;
        client
            .update_config(config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))
    }
}

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("is_started", &self.is_started())
            .finish()
    }
}
