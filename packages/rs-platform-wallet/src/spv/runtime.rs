//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::future::Future;
use std::pin::pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::{JoinError, JoinHandle};

use dashcore::sml::llmq_type::LLMQType;
use dashcore::sml::masternode_list::MasternodeList;
use dashcore::{PubkeyHash, QuorumHash, Transaction};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::{DiskStorageManager, StorageManager};
use dash_spv::sync::SyncProgress;
use dash_spv::{BroadcastResult, ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::broadcaster::BroadcastError;
use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::masternode::list::MasternodeListSummary;
use crate::spv::peers::{classify_peers, PeerTracker, SpvPeerInfo};
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient =
    DashSpvClient<WalletManager<PlatformWalletInfo>, PeerNetworkManager, DiskStorageManager>;

/// Maximum wait per stop call; the owned teardown continues after this deadline.
const SPV_STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// How often readiness is rechecked for a client with connected peers.
const SPV_READINESS_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Default)]
enum Shutdown {
    #[default]
    Idle,
    Running(JoinHandle<Result<(), JoinError>>),
    Failed(String),
}

/// SPV client runtime — owns the `DashSpvClient` and drives sync.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// handlers by reference (no cloning).
///
/// # Lifecycle
///
/// [`start`](Self::start) works only on a stopped runtime. [`stop`](Self::stop)
/// is idempotent; once it returns `Ok`, no SPV task runs and the storage
/// directory is released. A stop that does not complete says which way:
/// - [`ShutdownIncomplete`](PlatformWalletError::ShutdownIncomplete): teardown
///   outlived the wait and continues. Stop again; start and storage clear
///   return the same error until a stop completes.
/// - [`SpvProcessRestartRequired`](PlatformWalletError::SpvProcessRestartRequired):
///   startup or teardown panicked. This runtime refuses every later start,
///   stop and storage clear, and nothing is promised about SPV in this
///   process until it restarts.
pub struct SpvRuntime {
    event_manager: Arc<PlatformEventManager>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    client: RwLock<Option<SpvClient>>,
    last_config: RwLock<Option<ClientConfig>>,
    task: Mutex<Option<JoinHandle<()>>>,
    /// Teardown state. Held across client construction, run-loop spawning and
    /// each stop or storage clear, so none of them interleave.
    shutdown: tokio::sync::Mutex<Shutdown>,
    peer_tracker: Arc<PeerTracker>,
}

/// Classify a failure from the SPV acceptance-check path
/// ([`SpvRuntime::broadcast_transaction_and_wait`]).
///
/// dash-spv raises `NetworkError::NotConnected` from its zero-connected-peers
/// check *before* the transaction enters the send pipeline (no local dispatch,
/// no deferred rebroadcast), so it is a provably-never-sent failure and is
/// surfaced as [`BroadcastError::Rejected`] per the `SpvChannel` error
/// contract. Anything else may follow a partial send and must stay
/// [`BroadcastError::MaybeSent`]. Pinned by the tests below so a dash-spv
/// semantic change is caught at this crate's boundary.
fn classify_spv_send_error(error: dash_spv::error::SpvError) -> BroadcastError {
    use dash_spv::error::{NetworkError, SpvError};

    match error {
        SpvError::Network(NetworkError::NotConnected) => BroadcastError::Rejected {
            reason: "SPV broadcast not sent: no connected peers".to_string(),
        },
        other => BroadcastError::MaybeSent {
            reason: format!("SPV acceptance check failed: {other}"),
        },
    }
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
            last_config: RwLock::new(None),
            task: Mutex::new(None),
            shutdown: tokio::sync::Mutex::new(Shutdown::Idle),
            peer_tracker: Arc::new(PeerTracker::default()),
        }
    }

    /// Start SPV sync.
    pub async fn start(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        if self.client.read().await.is_some() {
            return Err(PlatformWalletError::SpvAlreadyRunning);
        }
        // Fail fast instead of queueing behind a stop that is still joining.
        self.ensure_no_live_run_loop()?;
        let shutdown = self.shutdown.lock().await;
        {
            let running = self.client.read().await;
            if running.is_some() {
                return Err(PlatformWalletError::SpvAlreadyRunning);
            }
        }
        self.ensure_joined(&shutdown)?;

        let network_manager = PeerNetworkManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        let mut storage_manager = DiskStorageManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        // Upstream starts the storage writer here and again in `run`, but
        // stops it only for a client that ran.
        StorageManager::stop(&mut storage_manager).await;

        // PlatformEventManager implements `EventHandler`; the peer tracker
        // rides alongside it so `connected_peers` can answer from the latest
        // `PeersUpdated` snapshot without a dash-spv query API.
        let event_handlers: Vec<Arc<dyn EventHandler>> = vec![
            Arc::clone(&self.event_manager) as Arc<dyn EventHandler>,
            Arc::clone(&self.peer_tracker) as Arc<dyn EventHandler>,
        ];

        let retained_config = config.clone();

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
        *self.last_config.write().await = Some(retained_config);

        Ok(())
    }

    /// Refuse restart until all previous startup and shutdown work is joined.
    fn ensure_no_live_run_loop(&self) -> Result<(), PlatformWalletError> {
        let shutdown = self.shutdown.try_lock().map_err(|_| {
            PlatformWalletError::SpvError(
                "an SPV start or stop is still in progress; wait for it before starting SPV"
                    .to_string(),
            )
        })?;
        self.ensure_joined(&shutdown)
    }

    /// Check that nothing is left to join. Takes the locked `shutdown` state:
    /// `task` only changes under that lock, so the answer cannot go stale.
    fn ensure_joined(&self, shutdown: &Shutdown) -> Result<(), PlatformWalletError> {
        match shutdown {
            Shutdown::Idle => {}
            Shutdown::Running(_) => {
                return Err(PlatformWalletError::ShutdownIncomplete(
                    "SPV teardown is still running; stop SPV again before starting it".to_string(),
                ))
            }
            Shutdown::Failed(error) => {
                return Err(PlatformWalletError::SpvProcessRestartRequired(
                    error.clone(),
                ))
            }
        }
        let task = self.task_slot();
        if task.is_some() {
            return Err(PlatformWalletError::ShutdownIncomplete(
                "SPV startup has not been joined; stop SPV before restarting it".to_string(),
            ));
        }
        Ok(())
    }

    /// Lock the startup-task slot, recovering from poisoning: the slot is only
    /// read, taken or replaced whole, so a panicking holder cannot tear it.
    fn task_slot(&self) -> MutexGuard<'_, Option<JoinHandle<()>>> {
        self.task.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Check whether the SPV client has been started.
    pub fn is_started(&self) -> bool {
        self.client.try_read().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Whether SPV sync is running or starting. `false` once background sync
    /// has failed, although the client stays started until [`Self::stop`].
    /// Never waits, so an event handler may call it.
    pub fn is_running(&self) -> bool {
        let Ok(client) = self.client.try_read() else {
            return false;
        };
        let Some(client) = client.as_ref() else {
            return false;
        };
        let starting = self
            .task_slot()
            .as_ref()
            .is_some_and(|task| !task.is_finished());
        if starting {
            return true;
        }
        let mut running = pin!(tokio::task::unconstrained(client.is_running()));
        match running
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(running) => running,
            // Upstream holds this lock while it cleans up after a failed sync
            // and, briefly, while it recovers from a fork.
            Poll::Pending => false,
        }
    }

    /// Whether a broadcast issued right now could reach the network: the
    /// client is started *and* at least one peer is connected.
    ///
    /// Both halves are required because both are pre-send rejections in
    /// [`broadcast_transaction_and_wait`](Self::broadcast_transaction_and_wait)
    /// — an unstarted client, and dash-spv's zero-connected-peers check
    /// classified by [`classify_spv_send_error`].
    async fn is_broadcast_ready(&self) -> bool {
        self.client.read().await.is_some() && !self.peer_tracker.snapshot().is_empty()
    }

    /// Resolve once a broadcast could actually reach the network, or when
    /// `timeout` elapses. Returns whether readiness was reached.
    ///
    /// This closes the launch race where work resumed at app start (the
    /// asset-lock catch-up in particular) broadcasts into a client that has
    /// not finished starting, takes the definitive `Rejected`
    /// ("client not started") verdict, and — having no retry — stays
    /// un-broadcast for the whole session.
    ///
    /// Readiness is polled rather than pushed: "started" is a `client`
    /// transition and "has peers" arrives as a dash-spv `PeersUpdated`
    /// event, with no combined signal to subscribe to. The poll interval is
    /// irrelevant next to the network latency being waited on.
    ///
    /// The bound is a plain `Duration` and is applied with
    /// [`tokio::time::timeout`], which saturates an unrepresentable deadline
    /// instead of panicking the way `Instant::now() + timeout` does. That
    /// matters because callers reach here through `extern "C"` entry points
    /// whose timeout arrives as an unrestricted `u64`, and a panic in an
    /// FFI frame aborts the host process.
    pub async fn wait_until_ready(&self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, async {
            while !self.is_broadcast_ready().await {
                tokio::time::sleep(SPV_READINESS_POLL_INTERVAL).await;
            }
        })
        .await
        .is_ok()
    }

    /// Broadcast a transaction through SPV peers and wait for dash-spv's
    /// network-acceptance verdict.
    ///
    /// dash-spv withholds the transaction from a subset of connected peers;
    /// an `inv` announcement of the txid from a withheld peer, an InstantSend
    /// lock, or a confirmation proves the transaction propagated and resolves
    /// [`BroadcastResult::Accepted`]. No signal within `timeout` resolves
    /// [`BroadcastResult::Uncertain`] (the p2p network has no negative
    /// signal — modern Dash Core removed the BIP61 `reject` message).
    /// dash-spv also injects the transaction into its local mempool pipeline
    /// as part of the broadcast, so no separate relay step is needed.
    ///
    /// Error contract (consumed by `SpvChannel`): failures that provably
    /// precede any send — an unstarted client, or dash-spv's
    /// zero-connected-peers check — surface as [`BroadcastError::Rejected`]
    /// ("never sent"); failures that may follow a partial send surface as
    /// [`BroadcastError::MaybeSent`].
    pub(crate) async fn broadcast_transaction_and_wait(
        &self,
        tx: &Transaction,
        timeout: Option<Duration>,
    ) -> Result<BroadcastResult, BroadcastError> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or(BroadcastError::Rejected {
            reason: "SPV broadcast not sent: client not started".to_string(),
        })?;

        client
            .broadcast_transaction_and_wait(tx, timeout)
            .await
            .map_err(classify_spv_send_error)
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

    /// Start upstream's background sync while retaining its client for queries and stop.
    async fn run(&self) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvError(
                "SPV Client not started".to_string(),
            ))?
            .clone();
        drop(client_guard);

        let result = client
            .run()
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()));

        self.finish_startup(result).await
    }

    async fn finish_startup(
        &self,
        result: Result<(), PlatformWalletError>,
    ) -> Result<(), PlatformWalletError> {
        if result.is_err() {
            let client = self.client.write().await.take();
            if let Some(client) = client {
                client.stop().await;
            }
            self.peer_tracker.clear();
        }
        result
    }

    /// Stop SPV sync, waiting up to 15 seconds for startup and teardown.
    ///
    /// A timeout or cancelled caller leaves teardown tracked for the next stop.
    /// Restart stays blocked until a stop confirms completion. Client queries
    /// already in flight finish before teardown starts; this wait is unbounded.
    ///
    /// # Errors
    ///
    /// See the lifecycle on [`SpvRuntime`].
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        self.stop_with(|client| async move { client.stop().await })
            .await
    }

    async fn stop_with<F, Fut>(&self, stop_client: F) -> Result<(), PlatformWalletError>
    where
        F: FnOnce(SpvClient) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut shutdown = self.shutdown.lock().await;
        self.stop_locked(&mut shutdown, stop_client).await
    }

    /// Run or rejoin teardown. The caller holds the `shutdown` lock.
    async fn stop_locked<F, Fut>(
        &self,
        shutdown: &mut Shutdown,
        stop_client: F,
    ) -> Result<(), PlatformWalletError>
    where
        F: FnOnce(SpvClient) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        if matches!(*shutdown, Shutdown::Idle) {
            let client = self.client.write().await.take();
            let startup = self.task_slot().take();
            let peers = Arc::clone(&self.peer_tracker);
            // Never cancel upstream run/stop: each can own callback-capable tasks
            // that cancellation would detach before their handles are restored.
            *shutdown = Shutdown::Running(tokio::spawn(async move {
                let result = match startup {
                    Some(task) => task.await,
                    None => Ok(()),
                };
                if let Some(client) = client {
                    stop_client(client).await;
                }
                peers.clear();
                result
            }));
        }

        if let Shutdown::Running(task) = shutdown {
            match tokio::time::timeout(SPV_STOP_TIMEOUT, task).await {
                Ok(Ok(Ok(()))) => *shutdown = Shutdown::Idle,
                Ok(result) => {
                    *shutdown = Shutdown::Failed(format!("SPV teardown task failed: {result:?}"))
                }
                Err(_) => {
                    return Err(PlatformWalletError::ShutdownIncomplete(
                        "SPV teardown timed out; it is still tracked for a retry".to_string(),
                    ))
                }
            }
        }
        match shutdown {
            Shutdown::Failed(error) => Err(PlatformWalletError::SpvProcessRestartRequired(
                error.clone(),
            )),
            _ => Ok(()),
        }
    }

    /// Start upstream background sync on the current Tokio runtime.
    ///
    /// Skipped with a warning while a start or stop is in progress, or while
    /// earlier startup or teardown is still unjoined. Call [`Self::stop`] to
    /// stop it.
    pub fn spawn_run_loop(self: &Arc<Self>) {
        let Ok(shutdown) = self.shutdown.try_lock() else {
            tracing::warn!("SPV background sync not spawned: an SPV start or stop is in progress");
            return;
        };
        if let Err(error) = self.ensure_joined(&shutdown) {
            tracing::warn!(%error, "SPV background sync not spawned: earlier SPV work is unjoined");
            return;
        }
        let this = Arc::clone(self);
        let mut task = self.task_slot();
        *task = Some(tokio::spawn(async move {
            if let Err(e) = this.run().await {
                tracing::warn!("SpvRuntime background startup failed: {}", e);
            }
        }));
    }

    /// The peers the SPV client is currently connected to, each classified
    /// against the masternode list (Evonode / Masternode / Normal, or
    /// Unknown while the masternode list hasn't synced yet).
    ///
    /// Returns an empty vec when the client isn't running or no peers are
    /// connected.
    pub async fn connected_peers(&self) -> Vec<SpvPeerInfo> {
        // Resolve the client before copying the snapshot: a concurrent
        // `stop()` removes the client under the write lock and clears the
        // tracker afterwards, so snapshotting first could return peers
        // that no longer exist.
        let client_guard = self.client.read().await;
        let Some(client) = client_guard.as_ref() else {
            return Vec::new();
        };

        let addresses = self.peer_tracker.snapshot();
        if addresses.is_empty() {
            return Vec::new();
        }

        let engine = client.masternode_list_engine().ok();
        drop(client_guard);

        match engine {
            Some(engine) => {
                let engine_guard = engine.read().await;
                classify_peers(&addresses, engine_guard.latest_masternode_list())
            }
            None => classify_peers(&addresses, None),
        }
    }

    /// Snapshot of the current deterministic masternode list (DML) keyed
    /// by proTxHash in internal/wire byte order (matching a registration
    /// txid), mapping to each entry's `is_valid` flag — the authoritative
    /// input for masternode status (Active / Inactive / Retired).
    ///
    /// Returns `None` when the DML isn't available (the SPV client isn't
    /// running, its masternode-list engine isn't initialized, or the list
    /// hasn't synced yet), so the caller renders "Unknown" and keeps any
    /// previously persisted status. Blocking: acquires the client + engine
    /// `tokio::RwLock`s via `blocking_read`, so it must run off the async
    /// runtime (FFI blocking thread), mirroring the other `*_blocking`
    /// accessors.
    pub fn masternode_validity_snapshot_blocking(
        &self,
    ) -> Option<std::collections::HashMap<[u8; 32], bool>> {
        // Clone the engine `Arc` out while holding the client lock, then
        // drop it before reading the engine — same ordering as
        // `connected_peers`.
        let engine = {
            let client_guard = self.client.blocking_read();
            let client = client_guard.as_ref()?;
            client.masternode_list_engine().ok()?
        };

        let engine_guard = engine.blocking_read();
        let list = engine_guard.latest_masternode_list()?;

        let mut map = std::collections::HashMap::with_capacity(list.masternodes.len());
        for qualified in list.masternodes.values() {
            let entry = &qualified.masternode_list_entry;
            // `pro_reg_tx_hash` is internal order (the DML map itself keys
            // by the reversed/display form, so read it off the entry).
            let mut pro_tx = [0u8; 32];
            pro_tx.copy_from_slice(entry.pro_reg_tx_hash.as_ref());
            map.insert(pro_tx, entry.is_valid);
        }
        Some(map)
    }

    /// The proTxHashes of every masternode in the current-tip deterministic
    /// masternode list whose voting key hash matches `voting_key_id` (the
    /// 20-byte hash160 of a voting public key).
    ///
    /// Replaces dashj's
    /// `MasternodeListManager.getMasternodesByVotingKey(votingKeyId)`, the
    /// lookup contested-username voting uses to find which masternode(s) a
    /// voting key can cast a vote for. The current tip is the highest
    /// `CoreBlockHeight` held by the engine (`latest_masternode_list`).
    ///
    /// Each proTxHash is returned in internal byte order — the same
    /// `pro_reg_tx_hash.as_ref()` convention as
    /// [`Self::masternode_validity_snapshot_blocking`]. Returns an empty vec
    /// when the DML isn't available (SPV client not running, engine not
    /// initialized, or the masternode list hasn't synced yet). Blocking:
    /// acquires the client + engine `tokio::RwLock`s via `blocking_read`, so
    /// it must run off the async runtime (FFI blocking thread), mirroring the
    /// other `*_blocking` accessors.
    pub fn masternodes_by_voting_key_blocking(&self, voting_key_id: &PubkeyHash) -> Vec<[u8; 32]> {
        // Clone the engine `Arc` out while holding the client lock, then drop
        // it before reading the engine — same ordering as `connected_peers`.
        let engine = {
            let client_guard = self.client.blocking_read();
            let Some(client) = client_guard.as_ref() else {
                return Vec::new();
            };
            match client.masternode_list_engine().ok() {
                Some(engine) => engine,
                None => return Vec::new(),
            }
        };

        let engine_guard = engine.blocking_read();
        let Some(list) = engine_guard.latest_masternode_list() else {
            return Vec::new();
        };

        masternodes_by_voting_key(list, voting_key_id)
    }

    /// Snapshot of the current-tip deterministic masternode list as typed
    /// summaries. `None` when the list isn't available (SPV client not
    /// running, engine not initialized, or masternode sync not complete).
    /// Clones the engine `Arc` out under the client lock and reads the
    /// engine without it — the two never nest, same as
    /// [`Self::masternode_validity_snapshot_blocking`].
    pub async fn masternode_list_summaries(&self) -> Option<Vec<MasternodeListSummary>> {
        let engine = {
            let client_guard = self.client.read().await;
            let client = client_guard.as_ref()?;
            client.masternode_list_engine().ok()?
        };
        let engine_guard = engine.read().await;
        let list = engine_guard.latest_masternode_list()?;
        Some(MasternodeListSummary::all_from_list(list))
    }

    /// Blocking twin of [`Self::masternode_list_summaries`] for FFI threads
    /// (`blocking_read`; never call from the async runtime).
    pub fn masternode_list_summaries_blocking(&self) -> Option<Vec<MasternodeListSummary>> {
        let engine = {
            let client_guard = self.client.blocking_read();
            let client = client_guard.as_ref()?;
            client.masternode_list_engine().ok()?
        };
        let engine_guard = engine.blocking_read();
        let list = engine_guard.latest_masternode_list()?;
        Some(MasternodeListSummary::all_from_list(list))
    }

    /// Get the current sync progress.
    ///
    /// Returns `None` if the SPV client is not running.
    pub async fn sync_progress(&self) -> Option<SyncProgress> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref()?;
        Some(client.progress().await)
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

    /// Hash of the stored header at `height`.
    ///
    /// Returns `None` if the SPV client isn't running or the header store
    /// does not hold that height (below the sync start, above the tip, or
    /// unreadable).
    pub(crate) async fn header_hash_at(
        &self,
        height: u32,
    ) -> crate::wallet::asset_lock::sync::locate::HeaderLookup {
        use crate::wallet::asset_lock::sync::locate::HeaderLookup;
        use dash_spv::storage::{BlockHeaderStorage, StorageManager};

        let client_guard = self.client.read().await;
        let Some(client) = client_guard.as_ref() else {
            return HeaderLookup::Unreadable("SPV client not running".to_string());
        };
        let storage_arc = client.storage();
        let storage = storage_arc.lock().await;
        let block_headers = StorageManager::block_headers(&*storage);
        drop(storage);
        let bh = block_headers.read().await;
        match BlockHeaderStorage::get_header(&*bh, height).await {
            Ok(Some(header)) => HeaderLookup::Found(*header.hash()),
            // The store is readable and the chain has nothing there.
            Ok(None) => HeaderLookup::Absent,
            Err(e) => HeaderLookup::Unreadable(e.to_string()),
        }
    }

    /// Clear all persisted SPV storage (headers, filters, state).
    ///
    /// A running client is stopped first, as by [`Self::stop`], and stays
    /// stopped: start SPV again to resync. Nothing is cleared unless that
    /// stop completes.
    pub async fn clear_storage(&self) -> Result<(), PlatformWalletError> {
        self.clear_storage_with(|client| async move { client.stop().await })
            .await
    }

    async fn clear_storage_with<F, Fut>(&self, stop_client: F) -> Result<(), PlatformWalletError>
    where
        F: FnOnce(SpvClient) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Held to the end so no start can reopen the storage mid-clear.
        let mut shutdown = self.shutdown.lock().await;
        self.stop_locked(&mut shutdown, stop_client).await?;

        let config = self.last_config.read().await.clone().ok_or_else(|| {
            PlatformWalletError::SpvError(
                "SPV storage location unknown; start the client at least once before clearing"
                    .to_string(),
            )
        })?;

        let mut storage = DiskStorageManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        StorageManager::clear(&mut storage)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        StorageManager::stop(&mut storage).await;

        Ok(())
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

/// The proTxHashes (internal byte order) of every entry in `list` whose
/// `key_id_voting` equals `voting_key_id`.
///
/// A single voting key can back more than one masternode, so this is a
/// filter-and-collect rather than a point lookup; the result is empty when
/// nothing matches.
///
/// # Why this lives here instead of in rust-dashcore
///
/// This duplicates `MasternodeList::masternodes_by_voting_key`, which is not
/// present on the Dash-owned rust-dashcore revision this workspace pins. The
/// upstream helper is still in flight as dashpay/rust-dashcore#916 and that PR
/// is blocked on being split, so pinning to a revision carrying it would mean
/// depending on a personal fork for an indefinite period. The filter is small
/// and reads only long-standing public SML fields, so keeping a local copy is
/// cheaper than the fork pin.
///
/// Delete this function and call `list.masternodes_by_voting_key(voting_key_id)`
/// once #916 lands and the workspace pin moves past it — tracked by
/// dashpay/platform#4262.
fn masternodes_by_voting_key(list: &MasternodeList, voting_key_id: &PubkeyHash) -> Vec<[u8; 32]> {
    list.masternodes
        .values()
        .filter(|qualified| qualified.masternode_list_entry.key_id_voting == *voting_key_id)
        .map(|qualified| {
            // Internal byte order, matching `masternode_validity_snapshot_blocking`:
            // the DML map keys by the reversed/display form, so read the hash off
            // the entry rather than the map key.
            let mut out = [0u8; 32];
            out.copy_from_slice(qualified.masternode_list_entry.pro_reg_tx_hash.as_ref());
            out
        })
        .collect()
}

#[cfg(test)]
mod masternodes_by_voting_key_tests {
    use dashcore::bls_sig_utils::BLSPublicKey;
    use dashcore::hashes::Hash;
    use dashcore::sml::masternode_list_entry::{
        EntryMasternodeType, MasternodeListEntry, MasternodeNetInfo,
    };
    use dashcore::{BlockHash, ProTxHash, PubkeyHash};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use super::{masternodes_by_voting_key, MasternodeList};

    /// Build a list from `(proTxHash-seed, voting-key-id)` pairs so each entry
    /// gets a distinct proTxHash and a caller-chosen voting key.
    fn list_from(entries: Vec<(u8, [u8; 20])>) -> MasternodeList {
        let masternodes: dashcore::sml::masternode_list::MasternodeMap = entries
            .into_iter()
            .map(|(seed, voting_key_id)| {
                let mut hash_bytes = [0u8; 32];
                hash_bytes[0] = seed;
                let pro_tx_hash = ProTxHash::from_byte_array(hash_bytes);
                let entry = MasternodeListEntry {
                    version: 1,
                    pro_reg_tx_hash: pro_tx_hash,
                    confirmed_hash: None,
                    service_address: MasternodeNetInfo::Legacy(SocketAddr::V4(SocketAddrV4::new(
                        Ipv4Addr::new(10, 0, 0, seed),
                        9999,
                    ))),
                    operator_public_key: BLSPublicKey::from([0u8; 48]),
                    key_id_voting: PubkeyHash::from_byte_array(voting_key_id),
                    is_valid: true,
                    mn_type: EntryMasternodeType::Regular,
                };
                (pro_tx_hash, std::sync::Arc::new(entry.into()))
            })
            .collect();
        MasternodeList::build(
            masternodes,
            std::collections::BTreeMap::new(),
            BlockHash::from_byte_array([0u8; 32]),
            0,
        )
        .build()
    }

    #[test]
    fn collects_every_masternode_sharing_a_voting_key() {
        let key_a = [0xAAu8; 20];
        let key_b = [0xBBu8; 20];
        // Two masternodes share voting key A, one uses key B.
        let list = list_from(vec![(1, key_a), (2, key_b), (3, key_a)]);

        let mut matched = masternodes_by_voting_key(&list, &PubkeyHash::from_byte_array(key_a));
        // Iteration is BTreeMap (proTxHash) order; sort on the seed byte so the
        // assert does not depend on it.
        matched.sort_by_key(|hash| hash[0]);
        assert_eq!(matched.len(), 2, "both key-A masternodes must be returned");
        assert_eq!(matched[0][0], 1);
        assert_eq!(matched[1][0], 3);
    }

    #[test]
    fn returns_the_single_masternode_for_an_unshared_voting_key() {
        let key_a = [0xAAu8; 20];
        let key_b = [0xBBu8; 20];
        let list = list_from(vec![(1, key_a), (2, key_b), (3, key_a)]);

        let matched = masternodes_by_voting_key(&list, &PubkeyHash::from_byte_array(key_b));
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0][0], 2);
    }

    #[test]
    fn returns_empty_when_no_masternode_uses_the_voting_key() {
        let list = list_from(vec![(1, [0xAAu8; 20]), (2, [0xBBu8; 20])]);

        let matched = masternodes_by_voting_key(&list, &PubkeyHash::from_byte_array([0xCCu8; 20]));
        assert!(
            matched.is_empty(),
            "an unused voting key must match nothing"
        );
    }

    #[test]
    fn returns_empty_for_an_empty_masternode_list() {
        let list = list_from(vec![]);

        let matched = masternodes_by_voting_key(&list, &PubkeyHash::from_byte_array([0xAAu8; 20]));
        assert!(matched.is_empty());
    }
}

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("is_started", &self.is_started())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use dash_spv::error::{NetworkError, SpvError};
    use dashcore::Network;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use super::{classify_spv_send_error, SpvRuntime};
    use crate::broadcaster::BroadcastError;
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler, PlatformEventManager};
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use dash_spv::ClientConfig;

    fn unstarted_runtime() -> SpvRuntime {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])))
    }

    /// Poll until `ready` holds, failing the test instead of hanging it.
    async fn wait_until(what: &str, ready: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !ready() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
    }

    /// An outstanding startup must remain owned until stop joins it.
    #[tokio::test]
    async fn should_refuse_a_start_while_a_parked_run_loop_is_live() {
        let runtime = unstarted_runtime();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let parked = tokio::spawn(async move {
            let _ = release_rx.await;
        });
        *runtime.task.lock().expect("spv task mutex poisoned") = Some(parked);

        let result = runtime.start(ClientConfig::default()).await;

        assert!(
            is_retryable(&result),
            "a start over a live parked run loop must ask for a stop, got {result:?}"
        );
        assert!(!runtime.is_started(), "no client may be started");
        assert!(
            runtime
                .task
                .lock()
                .expect("spv task mutex poisoned")
                .is_some(),
            "the live run loop must stay parked for a later stop to join"
        );
        let _ = release_tx.send(());
    }

    /// Startup remains owned by teardown while its original slot is empty.
    #[tokio::test]
    async fn should_refuse_a_start_while_a_stop_is_joining_the_run_loop() {
        let runtime = Arc::new(unstarted_runtime());
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let run_loop = tokio::spawn(async move {
            let _ = release_rx.await;
        });
        *runtime.task.lock().expect("spv task mutex poisoned") = Some(run_loop);

        let stopping = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { runtime.stop().await })
        };
        // The stop takes the handle out of `task` before it joins it.
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime
                .task
                .lock()
                .expect("spv task mutex poisoned")
                .is_some()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the stop should take the run-loop handle promptly");

        let result = runtime.start(ClientConfig::default()).await;

        assert!(
            matches!(result, Err(PlatformWalletError::SpvError(_))),
            "a start during a stop must fail, got {result:?}"
        );
        assert!(!runtime.is_started(), "no client may be started");

        release_tx
            .send(())
            .expect("the run loop is still waiting to be released");
        stopping
            .await
            .expect("the stop task completes")
            .expect("the released run loop exits cleanly");
        assert!(
            runtime.ensure_no_live_run_loop().is_ok(),
            "a finished stop must not block the next start"
        );
    }

    /// A second stop issued while the first is still joining the run loop
    /// must not report success before that loop has exited.
    #[tokio::test]
    async fn should_not_finish_a_second_stop_while_the_first_is_joining() {
        let runtime = Arc::new(unstarted_runtime());
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let run_loop = tokio::spawn(async move {
            let _ = release_rx.await;
        });
        *runtime.task.lock().expect("spv task mutex poisoned") = Some(run_loop);

        let first = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { runtime.stop().await })
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime
                .task
                .lock()
                .expect("spv task mutex poisoned")
                .is_some()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the first stop should take the run-loop handle promptly");

        let second = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { runtime.stop().await })
        };
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert!(
            !second.is_finished(),
            "a second stop must wait while the first is still joining the run loop"
        );

        release_tx
            .send(())
            .expect("the run loop is still waiting to be released");
        first
            .await
            .expect("the first stop task completes")
            .expect("the released run loop exits cleanly");
        second
            .await
            .expect("the second stop task completes")
            .expect("the second stop finds nothing left to stop");
    }

    /// Completion alone cannot prove success: stop must check the join result.
    #[tokio::test]
    async fn should_join_a_completed_startup_before_allowing_restart() {
        let runtime = unstarted_runtime();
        let exited = tokio::spawn(async {});
        tokio::time::timeout(Duration::from_secs(5), async {
            while !exited.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("an empty task should finish promptly");
        *runtime.task.lock().expect("spv task mutex poisoned") = Some(exited);

        assert!(runtime.ensure_no_live_run_loop().is_err());
        runtime.stop().await.unwrap();
        assert!(runtime.ensure_no_live_run_loop().is_ok());
        assert!(
            runtime
                .task
                .lock()
                .expect("spv task mutex poisoned")
                .is_none(),
            "an exited run loop must be dropped"
        );
    }

    #[tokio::test]
    async fn should_retain_client_after_successful_startup() {
        let (runtime, _storage) = offline_runtime().await;
        runtime.finish_startup(Ok(())).await.unwrap();
        *runtime.task.lock().unwrap() = Some(tokio::spawn(async {}));
        assert!(matches!(
            runtime.start(ClientConfig::default()).await,
            Err(PlatformWalletError::SpvAlreadyRunning)
        ));
        let retained = runtime.is_started();
        let progress = runtime.sync_progress().await;
        let broadcast = runtime
            .broadcast_transaction_and_wait(&dummy_tx(), None)
            .await;
        assert!(
            matches!(broadcast, Err(BroadcastError::Rejected { reason }) if reason.contains("no connected peers")),
            "broadcast must reach the retained client"
        );
        runtime.stop().await.unwrap();
        assert!(
            retained,
            "successful startup must retain access to the background client"
        );
        assert!(
            progress.is_some(),
            "queries must remain available after startup"
        );
    }

    #[tokio::test]
    async fn should_track_shutdown_after_stop_caller_is_cancelled() {
        let runtime = Arc::new(unstarted_runtime());
        let (release, pending) = tokio::sync::oneshot::channel();
        *runtime.task.lock().unwrap() = Some(tokio::spawn(async move {
            let _ = pending.await;
        }));
        let stopping = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { runtime.stop().await })
        };
        wait_until("stop to take the startup task", || {
            runtime.task.lock().unwrap().is_none()
        })
        .await;
        stopping.abort();
        let _ = stopping.await;
        let blocked = runtime.ensure_no_live_run_loop().is_err();
        let retry = tokio::time::timeout(Duration::from_millis(10), runtime.stop()).await;
        let _ = release.send(());
        runtime.stop().await.unwrap();
        assert!(
            blocked,
            "cancelling stop must not permit a restart over live work"
        );
        assert!(
            retry.is_err(),
            "retry must wait for the original work to finish"
        );
    }

    /// Teardown is still tracked: the host should stop again.
    fn is_retryable(result: &Result<(), PlatformWalletError>) -> bool {
        matches!(result, Err(PlatformWalletError::ShutdownIncomplete(_)))
    }

    /// Startup or teardown panicked: only a process restart recovers.
    fn needs_restart(result: &Result<(), PlatformWalletError>) -> bool {
        matches!(
            result,
            Err(PlatformWalletError::SpvProcessRestartRequired(_))
        )
    }

    async fn offline_runtime() -> (Arc<SpvRuntime>, tempfile::TempDir) {
        let storage = tempfile::tempdir().unwrap();
        let runtime = Arc::new(unstarted_runtime());
        runtime
            .start(
                ClientConfig::testnet()
                    .with_storage_path(storage.path())
                    .with_restrict_to_configured_peers(true),
            )
            .await
            .unwrap();
        (runtime, storage)
    }

    /// Wait out two periods of upstream's five-second storage writer and
    /// check that nothing re-created the removed directory.
    async fn assert_no_storage_writer_survives(storage: &tempfile::TempDir) {
        std::fs::remove_dir_all(storage.path()).unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert!(
            !storage.path().exists(),
            "a storage writer outlived its client"
        );
    }

    /// Upstream starts a storage writer at construction and stops it only for
    /// a client whose sync loop ran. Left behind, it keeps writing into a
    /// directory that was cleared or handed to the next client.
    #[tokio::test(start_paused = true)]
    async fn should_leave_no_storage_writer_after_clearing_a_client_that_never_ran() {
        let (runtime, storage) = offline_runtime().await;
        runtime.clear_storage().await.unwrap();
        assert_no_storage_writer_survives(&storage).await;
    }

    #[tokio::test(start_paused = true)]
    async fn should_leave_no_storage_writer_after_client_construction_fails() {
        let storage = tempfile::tempdir().unwrap();
        let mut config = ClientConfig::testnet()
            .with_storage_path(storage.path())
            .with_restrict_to_configured_peers(true);
        config.max_peers = 0;
        assert!(unstarted_runtime().start(config).await.is_err());
        assert_no_storage_writer_survives(&storage).await;
    }

    /// Drive the production startup path and wait for upstream `run` to return.
    async fn run_offline(runtime: &Arc<SpvRuntime>) {
        runtime.spawn_run_loop();
        wait_until("the startup task to finish", || {
            runtime
                .task
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|task| task.is_finished())
        })
        .await;
    }

    fn offline_config(storage: &tempfile::TempDir) -> ClientConfig {
        ClientConfig::testnet()
            .with_storage_path(storage.path())
            .with_restrict_to_configured_peers(true)
    }

    /// The production startup path: `spawn_run_loop` drives upstream `run`,
    /// the client stays available for queries, and stop tears down a client
    /// whose sync loop is live.
    #[tokio::test]
    async fn should_run_the_real_startup_and_stop_a_running_client() {
        let (runtime, storage) = offline_runtime().await;
        assert!(
            !runtime.is_running(),
            "a client that never ran is not running"
        );

        run_offline(&runtime).await;
        assert!(runtime.is_started(), "startup must retain the client");
        assert!(runtime.is_running());
        assert!(
            runtime.sync_progress().await.is_some(),
            "queries must reach the retained client"
        );

        runtime.stop().await.unwrap();
        assert!(!runtime.is_started());
        assert!(!runtime.is_running());
        runtime
            .start(offline_config(&storage))
            .await
            .expect("restart after stopping a running client");
        runtime.stop().await.unwrap();
    }

    /// Upstream stops itself when its sync loop fails and reports that only
    /// through `on_error`, so the retained client must stop counting as
    /// running.
    #[tokio::test]
    async fn should_not_report_running_after_upstream_background_sync_fails() {
        /// Reports the error and what a host callback sees when it queries
        /// the running state from inside the callback.
        struct ErrorObserver {
            runtime: std::sync::Weak<SpvRuntime>,
            report: Mutex<Option<tokio::sync::oneshot::Sender<(String, bool)>>>,
        }

        impl EventHandler for ErrorObserver {
            fn on_error(&self, error: &str) {
                if let Some(report) = self.report.lock().unwrap().take() {
                    let running = self.runtime.upgrade().is_some_and(|spv| spv.is_running());
                    let _ = report.send((error.to_owned(), running));
                }
            }
        }

        impl PlatformEventHandler for ErrorObserver {}

        let (runtime, storage) = offline_runtime().await;
        let (sender, error) = tokio::sync::oneshot::channel();
        runtime.event_manager.add_handler(Arc::new(ErrorObserver {
            runtime: Arc::downgrade(&runtime),
            report: Mutex::new(Some(sender)),
        }));
        run_offline(&runtime).await;
        assert!(runtime.is_running());

        // Dropping the empty fixture's wallet closes its event channel. The
        // real upstream monitor reports the failure and stops the sync loop.
        *runtime.wallet_manager.write().await = WalletManager::new(Network::Testnet);
        let (error, running_in_callback) = tokio::time::timeout(Duration::from_secs(5), error)
            .await
            .expect("upstream must report the background failure")
            .expect("the error observer must remain registered");
        assert!(
            !running_in_callback,
            "a callback querying the running state must get the stopped answer"
        );
        assert!(
            error.contains("WalletEvent monitor channel closed unexpectedly"),
            "expected a wallet event monitor failure, got {error}"
        );

        assert!(runtime.is_started(), "the client stays owned until stop");
        // Upstream cleans up after the failure in a task of its own.
        for _ in 0..1000 {
            assert!(
                !runtime.is_running(),
                "a failed sync must not report running while upstream cleans up"
            );
            tokio::task::yield_now().await;
        }
        runtime.stop().await.unwrap();
        assert!(!runtime.is_started());
        assert!(!runtime.is_running());
        runtime
            .start(offline_config(&storage))
            .await
            .expect("restart after releasing the stopped client");
        run_offline(&runtime).await;
        assert!(runtime.is_running());
        runtime.stop().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_timed_out_upstream_stop_alive_until_retry_joins_it() {
        let (runtime, storage) = offline_runtime().await;
        let (release, pending) = tokio::sync::oneshot::channel();
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let completed = Arc::clone(&stopped);
        let result = runtime
            .stop_with(move |client| async move {
                let _ = pending.await;
                client.stop().await;
                completed.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .await;
        assert!(
            is_retryable(&result),
            "an incomplete upstream stop must time out as retryable, got {result:?}"
        );
        assert!(is_retryable(&runtime.start(ClientConfig::default()).await));
        assert!(
            is_retryable(&runtime.clear_storage().await),
            "storage must not be cleared under a pending upstream stop"
        );
        assert!(
            is_retryable(&runtime.stop().await),
            "retry must not report clean while upstream stop is pending"
        );
        release
            .send(())
            .expect("timed out stop must still own its future");
        runtime.stop().await.unwrap();
        assert!(stopped.load(std::sync::atomic::Ordering::SeqCst));
        runtime
            .start(
                ClientConfig::testnet()
                    .with_storage_path(storage.path())
                    .with_restrict_to_configured_peers(true),
            )
            .await
            .expect("restart after confirmed teardown");
        runtime.stop().await.unwrap();
        runtime.stop().await.unwrap();
    }

    #[tokio::test]
    async fn should_keep_upstream_stop_alive_when_its_caller_is_cancelled() {
        let (runtime, _storage) = offline_runtime().await;
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, pending) = tokio::sync::oneshot::channel();
        let stopping = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move {
                runtime
                    .stop_with(move |client| async move {
                        entered.send(()).unwrap();
                        let _ = pending.await;
                        client.stop().await;
                    })
                    .await
            })
        };
        started.await.unwrap();
        stopping.abort();
        let _ = stopping.await;
        assert!(runtime.start(ClientConfig::default()).await.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(10), runtime.stop())
                .await
                .is_err()
        );
        release
            .send(())
            .expect("cancelled caller must leave upstream stop owned");
        runtime.stop().await.unwrap();
    }

    /// Clearing stops the client, so the runtime must not keep reporting it
    /// as started or holding its startup task.
    #[tokio::test]
    async fn should_leave_spv_stopped_and_restartable_after_clearing_a_running_client() {
        let (runtime, storage) = offline_runtime().await;
        runtime.finish_startup(Ok(())).await.unwrap();
        *runtime.task.lock().unwrap() = Some(tokio::spawn(async {}));

        runtime.clear_storage().await.unwrap();

        assert!(
            !runtime.is_started(),
            "a cleared client is stopped and must not be reported as started"
        );
        runtime
            .start(
                ClientConfig::testnet()
                    .with_storage_path(storage.path())
                    .with_restrict_to_configured_peers(true),
            )
            .await
            .expect("restart after clearing a running client");
        runtime.stop().await.unwrap();
    }

    #[tokio::test]
    async fn should_keep_teardown_tracked_when_a_clear_storage_caller_is_cancelled() {
        let (runtime, storage) = offline_runtime().await;
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, pending) = tokio::sync::oneshot::channel();
        let clearing = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move {
                runtime
                    .clear_storage_with(move |client| async move {
                        entered.send(()).unwrap();
                        let _ = pending.await;
                        client.stop().await;
                    })
                    .await
            })
        };
        started.await.unwrap();
        clearing.abort();
        let _ = clearing.await;
        assert!(runtime.start(ClientConfig::default()).await.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(10), runtime.clear_storage())
                .await
                .is_err(),
            "a retried clear must wait for the teardown it left running"
        );
        release
            .send(())
            .expect("cancelled caller must leave upstream stop owned");
        runtime.clear_storage().await.unwrap();
        runtime
            .start(
                ClientConfig::testnet()
                    .with_storage_path(storage.path())
                    .with_restrict_to_configured_peers(true),
            )
            .await
            .expect("restart after the retried clear joins teardown");
        runtime.stop().await.unwrap();
    }

    #[tokio::test]
    async fn should_wait_for_startup_before_stopping_its_client() {
        let (runtime, _storage) = offline_runtime().await;
        let (release, pending) = tokio::sync::oneshot::channel();
        let startup_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let completed = Arc::clone(&startup_done);
        *runtime.task.lock().unwrap() = Some(tokio::spawn(async move {
            let _ = pending.await;
            completed.store(true, std::sync::atomic::Ordering::SeqCst);
        }));
        let stopping = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move {
                runtime
                    .stop_with(move |client| async move {
                        assert!(startup_done.load(std::sync::atomic::Ordering::SeqCst));
                        client.stop().await;
                    })
                    .await
            })
        };
        wait_until("stop to take the startup task", || {
            runtime.task.lock().unwrap().is_none()
        })
        .await;
        assert!(!stopping.is_finished());
        runtime.spawn_run_loop();
        assert!(
            runtime.task.lock().unwrap().is_none(),
            "cannot spawn during teardown"
        );
        release.send(()).unwrap();
        stopping.await.unwrap().unwrap();
    }

    /// Failed-startup cleanup runs inside the startup task, whose finished
    /// handle stays owned until stop joins it.
    #[tokio::test]
    async fn should_clean_up_failed_startup_and_allow_restart_after_stop() {
        let (runtime, storage) = offline_runtime().await;
        let config = || {
            ClientConfig::testnet()
                .with_storage_path(storage.path())
                .with_restrict_to_configured_peers(true)
        };
        let startup = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move {
                let failure = PlatformWalletError::SpvError("mock startup failure".into());
                assert!(runtime.finish_startup(Err(failure)).await.is_err());
            })
        };
        *runtime.task.lock().unwrap() = Some(startup);
        wait_until("the failed startup to finish its cleanup", || {
            runtime
                .task
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|task| task.is_finished())
        })
        .await;

        assert!(!runtime.is_started(), "failed startup must drop its client");
        assert!(
            is_retryable(&runtime.start(config()).await),
            "an unjoined startup must block restart until a stop joins it"
        );
        runtime.stop().await.unwrap();
        runtime
            .start(config())
            .await
            .expect("restart after stop joins the failed startup");
        runtime.stop().await.unwrap();
    }

    #[tokio::test]
    async fn should_never_report_clean_after_a_startup_task_panics() {
        let runtime = Arc::new(unstarted_runtime());
        *runtime.task.lock().unwrap() = Some(tokio::spawn(async { panic!("mock startup panic") }));
        assert!(needs_restart(&runtime.stop().await));
        assert!(
            needs_restart(&runtime.stop().await),
            "a join failure cannot be forgotten by a retry"
        );
        assert!(needs_restart(&runtime.start(ClientConfig::default()).await));
        assert!(needs_restart(&runtime.clear_storage().await));
        runtime.spawn_run_loop();
        assert!(
            runtime.task.lock().unwrap().is_none(),
            "no run loop may be spawned over a panicked startup"
        );
    }

    #[tokio::test]
    async fn should_preserve_ownership_when_cancelled_while_waiting_for_client_access() {
        let (runtime, _storage) = offline_runtime().await;
        let guard = runtime.client.read().await;
        let stopping = {
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { runtime.stop().await })
        };
        // Stop takes `shutdown` and then parks on the client write lock.
        wait_until("stop to take the shutdown lock", || {
            runtime.shutdown.try_lock().is_err()
        })
        .await;
        stopping.abort();
        let _ = stopping.await;
        assert!(
            guard.is_some(),
            "a cancelled lock wait must not take the client"
        );
        drop(guard);
        runtime.stop().await.unwrap();
        assert!(!runtime.is_started());
    }

    #[tokio::test]
    async fn should_keep_teardown_panics_non_clean_on_every_retry() {
        let (runtime, _storage) = offline_runtime().await;
        let failed = runtime
            .stop_with(|_client| async { panic!("mock teardown panic") })
            .await;
        assert!(needs_restart(&failed));
        assert!(needs_restart(&runtime.stop().await));
        assert!(needs_restart(&runtime.start(ClientConfig::default()).await));
        assert!(needs_restart(&runtime.clear_storage().await));
    }

    /// A panic under the `task` mutex must not make every later lifecycle
    /// call panic too: teardown has to stay reachable.
    #[tokio::test]
    async fn should_keep_lifecycle_usable_after_the_task_mutex_is_poisoned() {
        let runtime = Arc::new(unstarted_runtime());
        let poisoner = Arc::clone(&runtime);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.task.lock().unwrap();
            panic!("mock panic while holding the task mutex");
        })
        .join();
        assert!(runtime.task.is_poisoned());

        runtime.spawn_run_loop();
        runtime.stop().await.unwrap();
        assert!(runtime.ensure_no_live_run_loop().is_ok());
    }

    /// A minimal valid transaction — the unstarted-client arm never
    /// inspects it.
    fn dummy_tx() -> dashcore::Transaction {
        dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        }
    }

    /// An unstarted client fails the acceptance-check path before any bytes
    /// leave the process, so it must classify `Rejected` ("never sent") per
    /// the `SpvChannel` error contract — this is what lets a
    /// DAPI-unreachable + SPV-down send release its UTXO reservation.
    #[tokio::test]
    async fn broadcast_and_wait_on_unstarted_client_is_never_sent() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        let runtime = SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])));

        let result = runtime
            .broadcast_transaction_and_wait(&dummy_tx(), None)
            .await;
        assert!(
            matches!(result, Err(BroadcastError::Rejected { .. })),
            "unstarted client must classify never-sent on the acceptance path, got {result:?}"
        );
    }

    /// dash-spv raises `NotConnected` from its zero-peer check before the
    /// transaction enters the send pipeline, so it is the one error the
    /// acceptance path may classify as never-sent. If dash-spv ever starts
    /// raising `NotConnected` after a partial send, this pin must be
    /// revisited — releasing on a post-send failure reopens the
    /// double-spend-on-retry window.
    #[test]
    fn not_connected_classifies_never_sent_on_acceptance_path() {
        let result = classify_spv_send_error(SpvError::Network(NetworkError::NotConnected));
        assert!(
            matches!(result, BroadcastError::Rejected { .. }),
            "NotConnected must classify never-sent on the acceptance path, got {result:?}"
        );
    }

    /// The readiness predicate must fail closed on an unstarted client:
    /// the acceptance path rejects that state before any send, so reporting
    /// it ready hands the caller straight back into the never-sent verdict
    /// the gate exists to avoid.
    #[tokio::test(start_paused = true)]
    async fn readiness_is_not_reached_while_the_client_is_unstarted() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        let runtime = SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])));

        assert!(
            !runtime.wait_until_ready(Duration::from_secs(30)).await,
            "an unstarted client must never report broadcast-ready"
        );
    }

    /// An `extern "C"` caller supplies the readiness budget as an
    /// unrestricted `u64` of seconds. Building the deadline with
    /// `Instant::now() + timeout` panics once that instant is not
    /// representable, and a panic inside an FFI frame aborts the host
    /// process instead of returning a result code — so the wait has to
    /// survive an extreme budget rather than take the host down with it.
    #[tokio::test(start_paused = true)]
    async fn an_extreme_readiness_budget_does_not_panic() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        let runtime = SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])));

        // Never resolves (the client is unstarted), so cut it short: the
        // assertion here is that constructing the wait survives, not that
        // it finishes.
        let outcome = tokio::time::timeout(
            Duration::from_secs(1),
            runtime.wait_until_ready(Duration::MAX),
        )
        .await;

        assert!(outcome.is_err(), "an extreme budget must park, not resolve");
    }

    /// A started client with no peers is the OTHER pre-send rejection, and
    /// readiness has to observe both halves and then actually resolve.
    ///
    /// The launch race this gate exists for ends the moment dash-spv reports
    /// its first connection, so the predicate must go from false to true on
    /// that event alone — with no restart, and without the caller polling
    /// anything itself. A predicate that only ever reported false would keep
    /// every recovery test green (they all assert around an expired wait)
    /// while turning the gate into a fixed 15s delay before the same
    /// never-sent broadcast, which is strictly worse than not waiting.
    ///
    /// This starts a real client — offline, restricted to a configured peer
    /// list that is empty, so it opens its storage and connects to nothing —
    /// because "started" is exactly the half a double cannot stand in for.
    #[tokio::test(start_paused = true)]
    async fn readiness_arrives_when_a_started_client_reports_its_first_peer() {
        use dash_spv::network::NetworkEvent;
        use dash_spv::{ClientConfig, EventHandler};
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        // `DiskStorageManager` locks the directory it opens, so the client
        // gets one of its own and the stop below releases it.
        let storage = std::env::temp_dir().join(format!(
            "platform-wallet-spv-readiness-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&storage).expect("private storage dir");
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        let runtime = SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])));
        runtime
            .start(
                ClientConfig::testnet()
                    .with_storage_path(&storage)
                    .with_restrict_to_configured_peers(true),
            )
            .await
            .expect("an offline client with no configured peers still starts");
        assert!(runtime.is_started(), "the client must be started");

        assert!(
            !runtime.wait_until_ready(Duration::from_secs(30)).await,
            "a started client with no connected peers must not report ready — \
             dash-spv's zero-peer check rejects the send before it dispatches, \
             exactly like an unstarted client"
        );

        // The event dash-spv pushes to its handlers on the first connection.
        runtime
            .peer_tracker
            .on_network_event(&NetworkEvent::PeersUpdated {
                connected_count: 1,
                addresses: vec![SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
                    19999,
                )],
                best_height: Some(1_100_000),
            });

        assert!(
            runtime.wait_until_ready(Duration::from_secs(30)).await,
            "a started client that has just reported its first peer must \
             report ready — this transition is the whole point of the wait"
        );

        runtime
            .stop()
            .await
            .expect("clean stop releases the data dir");
        let _ = std::fs::remove_dir_all(&storage);
    }

    /// Every other error on the acceptance path may follow a partial send
    /// and must stay `MaybeSent`.
    #[test]
    fn other_acceptance_path_errors_classify_maybe_sent() {
        for error in [
            SpvError::Network(NetworkError::Timeout),
            SpvError::Network(NetworkError::PeerDisconnected),
            SpvError::Config("bad config".to_string()),
        ] {
            let result = classify_spv_send_error(error);
            assert!(
                matches!(result, BroadcastError::MaybeSent { .. }),
                "non-NotConnected errors must stay MaybeSent, got {result:?}"
            );
        }
    }
}
