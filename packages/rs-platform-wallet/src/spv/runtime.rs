//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::RwLock;

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};
use tokio_util::sync::CancellationToken;

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::sync::SyncProgress;
use dash_spv::{ClientConfig, DashSpvClient, Hash};

use key_wallet_manager::WalletManager;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient = DashSpvClient<
    WalletManager<PlatformWalletInfo>,
    PeerNetworkManager,
    DiskStorageManager,
    PlatformEventManager,
>;

/// SPV client runtime — owns the `DashSpvClient` and drives sync.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// handlers by reference (no cloning).
pub struct SpvRuntime {
    event_manager: Arc<PlatformEventManager>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    client: RwLock<Option<SpvClient>>,
    /// Cancel token for the `run()` task when it was spawned via
    /// [`spawn_in_background`]. [`stop`] fires this token and joins
    /// on the client shutdown.
    background_cancel: StdMutex<Option<CancellationToken>>,
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
            background_cancel: StdMutex::new(None),
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

        let spv_client = DashSpvClient::new(
            config,
            network_manager,
            storage_manager,
            Arc::clone(&self.wallet_manager),
            Arc::clone(&self.event_manager),
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

    /// Run the SPV sync loop until the cancellation token fires.
    pub async fn run(
        &self,
        config: ClientConfig,
        cancel_token: CancellationToken,
    ) -> Result<(), PlatformWalletError> {
        tracing::info!("SpvRuntime::run() starting client...");
        self.start(config).await?;
        tracing::info!("SpvRuntime::run() client started, entering sync loop");

        let result = {
            let client_guard = self.client.read().await;
            let client = client_guard
                .as_ref()
                .ok_or(PlatformWalletError::SpvNotRunning)?;

            let run_cancel = CancellationToken::new();
            let run_future = client.run(run_cancel.clone());
            tokio::pin!(run_future);

            tokio::select! {
                res = &mut run_future => {
                    tracing::info!("SpvRuntime::run() client.run() completed: {:?}", res.is_ok());
                    res.map_err(|e| PlatformWalletError::SpvError(e.to_string()))
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("SpvRuntime::run() cancel_token fired, cancelling client");
                    run_cancel.cancel();
                    Ok(())
                }
            }
        };

        tracing::info!(
            "SpvRuntime::run() exiting sync loop, result ok={}",
            result.is_ok()
        );
        if let Err(e) = self.stop().await {
            tracing::warn!("SPV stop error during cleanup: {}", e);
        }
        tracing::info!("SpvRuntime::run() done");
        result
    }

    /// Stop SPV sync gracefully.
    ///
    /// If a `run()` task was spawned via [`spawn_in_background`], its
    /// cancel token is fired here too so the background task exits.
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        if let Some(token) = self
            .background_cancel
            .lock()
            .expect("background_cancel poisoned")
            .take()
        {
            token.cancel();
        }
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
    /// The returned cancel token is stashed internally; calling [`stop`]
    /// fires it and awaits client shutdown. Replacing an already-running
    /// background task cancels the previous one first.
    pub fn spawn_in_background(self: &Arc<Self>, config: ClientConfig) {
        // Cancel any previous run.
        let mut guard = self.background_cancel.lock().expect("bg_cancel poisoned");
        if let Some(prev) = guard.take() {
            prev.cancel();
        }
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());
        drop(guard);

        let this = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = this.run(config, cancel).await {
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
