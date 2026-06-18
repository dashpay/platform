//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};

use dashcore::Network;

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::{BlockHeaderStorage, DiskStorageManager, StorageManager};
use dash_spv::sync::SyncProgress;
use dash_spv::{ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::spv::genesis::{resolve_devnet_genesis_header, DevnetGenesisOverride};
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
    /// Cancel token for the `run()` task when it was spawned via
    /// [`spawn_in_background`]. [`stop`] fires this token and joins
    /// on the client shutdown.
    background_cancel: StdMutex<Option<CancellationToken>>,
    /// JoinHandle for the background task spawned by [`spawn_in_background`].
    /// [`stop`] joins it with a 15s timeout and aborts if it stalls.
    task: StdMutex<Option<JoinHandle<()>>>,
    /// Per-field overrides for the devnet genesis header pre-seeded
    /// into SPV storage on [`start`]. Empty = use the `dashcore`
    /// built-in (the standard / porter devnet genesis). Only consulted
    /// when the client config's network is [`Network::Devnet`].
    devnet_genesis: StdMutex<DevnetGenesisOverride>,
    /// Optional terminal sync height. `None` (the default) syncs to
    /// chain tip exactly as before. `Some(h)` makes [`run`] halt once
    /// the confirmed filter height reaches `h`. See
    /// [`set_terminal_height`](Self::set_terminal_height).
    terminal_height: StdMutex<Option<u32>>,
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
            task: StdMutex::new(None),
            devnet_genesis: StdMutex::new(DevnetGenesisOverride::default()),
            terminal_height: StdMutex::new(None),
        }
    }

    /// Set an optional terminal sync height.
    ///
    /// `None` (the default) keeps the production behaviour: [`run`]
    /// syncs to chain tip and only returns when [`stop`] is called.
    /// `Some(h)` makes the next [`run`] halt the client once the
    /// confirmed filter height (the height up to which compact-filter
    /// batches have been fully committed to the wallet) reaches `h`,
    /// so a caller can sync a fixed historical window without racing
    /// the live tip. Must be set before [`run`] / [`spawn_in_background`]
    /// to take effect on that sync.
    pub fn set_terminal_height(&self, height: Option<u32>) {
        *self
            .terminal_height
            .lock()
            .expect("terminal_height poisoned") = height;
    }

    /// Override the devnet genesis header pre-seeded on [`start`].
    ///
    /// Useful only for a non-standard devnet whose block 0 differs from
    /// the `dashcore` built-in; the default (no override) already
    /// covers every standard Dash devnet. Has no effect once the client
    /// is running, and is ignored on non-devnet networks.
    pub fn set_devnet_genesis_override(&self, overrides: DevnetGenesisOverride) {
        *self.devnet_genesis.lock().expect("devnet_genesis poisoned") = overrides;
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

        if config.network == Network::Devnet {
            self.preseed_devnet_genesis(&storage_manager).await?;
        }

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

    /// Pre-seed the devnet genesis header into SPV storage when the
    /// store is empty.
    ///
    /// `dash-spv` has no built-in genesis for devnet, so its own
    /// `initialize_genesis_block` would fail with "No known genesis
    /// hash for network". That routine early-returns when storage
    /// already holds a tip, so seeding genesis at height 0 here lets
    /// the client start cleanly. No-ops when the store is non-empty
    /// (warm cache), so it stays idempotent across runs.
    async fn preseed_devnet_genesis(
        &self,
        storage: &DiskStorageManager,
    ) -> Result<(), PlatformWalletError> {
        let block_headers = StorageManager::block_headers(storage);
        let mut bh = block_headers.write().await;
        if BlockHeaderStorage::get_tip_height(&*bh).await.is_some() {
            tracing::debug!("SPV storage already has a tip; skipping devnet genesis pre-seed");
            return Ok(());
        }

        let overrides = self
            .devnet_genesis
            .lock()
            .expect("devnet_genesis poisoned")
            .clone();
        let header = resolve_devnet_genesis_header(&overrides)
            .map_err(|e| PlatformWalletError::SpvError(format!("devnet genesis pre-seed: {e}")))?;

        BlockHeaderStorage::store_headers(&mut *bh, &[header])
            .await
            .map_err(|e| {
                PlatformWalletError::SpvError(format!("failed to pre-seed devnet genesis: {e}"))
            })?;
        tracing::info!(
            genesis_hash = %header.block_hash(),
            "pre-seeded devnet genesis header into SPV storage"
        );
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
        let client = client_guard.as_ref().ok_or(PlatformWalletError::SpvError(
            "SPV Client not started".to_string(),
        ))?;

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
        let client = client_guard.as_ref().ok_or(PlatformWalletError::SpvError(
            "SPV Client not started".to_string(),
        ))?;

        let llmq_type = LLMQType::from(quorum_type as u8);
        let qh = QuorumHash::from_byte_array(quorum_hash).reverse();

        let quorum = client
            .get_quorum_at_height(height, llmq_type, qh)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        Ok(*quorum.quorum_entry.quorum_public_key.as_ref())
    }

    /// Drive the sync loop of an already-[`start`]ed client until [`stop`]
    /// is called
    async fn run(&self) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvError(
                "SPV Client not started".to_string(),
            ))?
            .clone();
        drop(client_guard);

        let terminal_height = *self
            .terminal_height
            .lock()
            .expect("terminal_height poisoned");

        let result = match terminal_height {
            // Production path: sync to tip, return only on `stop()`.
            None => client
                .run()
                .await
                .map_err(|e| PlatformWalletError::SpvError(e.to_string())),
            // Capped path: race the sync loop against a watcher that
            // stops the client once filters are committed up to `target`.
            // `client.stop()` flips the run-loop's running flag, so
            // `client.run()` then returns cleanly with the same result it
            // would on an external `stop()`.
            Some(target) => {
                let watcher_client = client.clone();
                let result = tokio::select! {
                    res = client.run() => {
                        res.map_err(|e| PlatformWalletError::SpvError(e.to_string()))
                    }
                    _ = Self::watch_terminal_height(&watcher_client, target) => {
                        if let Err(e) = watcher_client.stop().await {
                            tracing::warn!(
                                target,
                                error = %e,
                                "terminal-height stop returned error"
                            );
                        }
                        Ok(())
                    }
                };
                result
            }
        };

        let mut client = self.client.write().await;
        let _ = client.take();

        result
    }

    /// Poll the client's sync progress until the confirmed filter
    /// height reaches `target`. Resolves once the cap is met; the
    /// caller is responsible for stopping the client afterwards.
    ///
    /// Confirmed filter height = `FiltersProgress::committed_height`,
    /// the height up to which compact-filter batches have been fully
    /// committed to the wallet — the right gate for "all funds visible
    /// up to here". Polls every `TERMINAL_HEIGHT_POLL` so the cost is
    /// negligible against a multi-minute scan.
    async fn watch_terminal_height(client: &SpvClient, target: u32) {
        const TERMINAL_HEIGHT_POLL: std::time::Duration = std::time::Duration::from_millis(500);
        loop {
            let committed = client
                .sync_progress()
                .await
                .filters()
                .ok()
                .map(|f| f.committed_height())
                .unwrap_or(0);
            if committed >= target {
                tracing::info!(
                    target,
                    committed,
                    "terminal sync height reached; stopping SPV client"
                );
                return;
            }
            tokio::time::sleep(TERMINAL_HEIGHT_POLL).await;
        }
    }

    /// Synchronously fire the background `run()` task's cancellation
    /// token, if any. The actual storage/lockfile teardown still
    /// happens asynchronously inside the spawned task as it unwinds
    /// to its `self.stop().await` epilogue — this method just wakes
    /// it. Idempotent: subsequent calls (and a follow-up [`stop`])
    /// see `None` and return immediately.
    ///
    /// Designed for sync contexts where awaiting [`stop`] isn't
    /// possible — for example a `std::panic::set_hook` callback that
    /// needs to release the dash-spv data-dir lock before the next
    /// init attempt without blocking the panicking thread.
    pub fn cancel_background(&self) {
        if let Some(token) = self
            .background_cancel
            .lock()
            .expect("background_cancel poisoned")
            .take()
        {
            token.cancel();
        }
    }

    /// Stop SPV sync gracefully. Unlocks the data dir safely.
    ///
    /// If a `run()` task was spawned via [`spawn_in_background`], its
    /// cancel token is fired and the handle is joined with a 15s timeout.
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        if let Some(token) = self
            .background_cancel
            .lock()
            .expect("background_cancel poisoned")
            .take()
        {
            token.cancel();
        }

        let taken = {
            let mut client = self.client.write().await;
            client.take()
        };

        let stop_result = match taken {
            Some(c) => c
                .stop()
                .await
                .map_err(|e| PlatformWalletError::SpvError(e.to_string())),
            None => Ok(()),
        };

        let handle = self.task.lock().expect("spv task mutex poisoned").take();
        if let Some(handle) = handle {
            let abort = handle.abort_handle();
            if tokio::time::timeout(std::time::Duration::from_secs(15), handle)
                .await
                .is_err()
            {
                tracing::warn!(
                    "SPV stop: background run loop did not unwind within 15s; aborting it"
                );
                abort.abort();
            }
        }

        stop_result
    }

    /// Spawn a background task that **starts** the SPV client with
    /// `config` and then drives its sync loop, returning immediately.
    ///
    /// The cancel token is stashed internally; calling [`stop`] (or
    /// [`cancel_background`]) fires it so the spawned task observes
    /// shutdown. Replacing an already-running background task cancels
    /// the previous one first.
    ///
    /// Unlike [`spawn_run_loop`](Self::spawn_run_loop), this folds
    /// [`start`](Self::start) into the spawned task. Callers that need
    /// start errors surfaced synchronously should call
    /// [`start`](Self::start) themselves and use
    /// [`spawn_run_loop`](Self::spawn_run_loop) instead.
    pub fn spawn_in_background(self: &Arc<Self>, config: ClientConfig) {
        // Cancel any previous run.
        let mut cancel_guard = self.background_cancel.lock().expect("bg_cancel poisoned");
        if let Some(prev) = cancel_guard.take() {
            prev.cancel();
        }
        let cancel = CancellationToken::new();
        *cancel_guard = Some(cancel.clone());
        drop(cancel_guard);

        let this = Arc::clone(self);
        let run_this = Arc::clone(&this);
        let handle = tokio::spawn(async move {
            tokio::select! {
                res = async move {
                    run_this.start(config).await?;
                    run_this.run().await
                } => {
                    if let Err(e) = res {
                        tracing::warn!("SpvRuntime background run exited with error: {}", e);
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("SpvRuntime background cancel fired; stopping client");
                    if let Err(e) = this.stop().await {
                        tracing::warn!("SpvRuntime cancel stop error: {}", e);
                    }
                }
            }
        });

        *self.task.lock().expect("spv task mutex poisoned") = Some(handle);
    }

    /// Spawn the sync loop of an already-[`start`](Self::start)ed client
    /// on the current tokio runtime and return immediately.
    ///
    /// Ignores the call (with a warning) if a task is already running.
    /// Call [`stop`] to stop it.
    pub fn spawn_run_loop(self: &Arc<Self>) {
        {
            let existing = self.task.lock().expect("spv task mutex poisoned");
            if existing.is_some() {
                tracing::warn!("spawn_run_loop called while a task is already running; ignoring");
                return;
            }
        }

        let this = Arc::clone(self);
        let handle = tokio::spawn(async move {
            if let Err(e) = this.run().await {
                tracing::warn!("SpvRuntime background run loop exited with error: {}", e);
            }
        });

        *self.task.lock().expect("spv task mutex poisoned") = Some(handle);
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
        let client = client_guard.as_ref().ok_or(PlatformWalletError::SpvError(
            "SPV Client not started".to_string(),
        ))?;

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
        let client = client_guard.as_ref().ok_or(PlatformWalletError::SpvError(
            "SPV Client not started".to_string(),
        ))?;

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
