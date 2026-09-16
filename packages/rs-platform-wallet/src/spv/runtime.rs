//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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

/// Graceful join budget for the SPV run loop before escalating to `abort`.
const SPV_STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// Budget for `DashSpvClient::stop()` itself.
///
/// dash-spv's stop joins its internal monitors, and those monitors dispatch
/// host event callbacks **synchronously** — a callback blocked in host code
/// (an FFI persister `store`, say) makes the stop unbounded. Since
/// [`SpvRuntime::stop`] sits on the path the FFI `destroy` must return
/// through, an unbounded stop hangs teardown outright instead of surfacing
/// `ErrorShutdownIncomplete`. On timeout the partially-stopped client is
/// dropped and the stop is reported as an error, so the caller treats SPV
/// as non-clean.
const SPV_CLIENT_STOP_BUDGET: Duration = Duration::from_secs(15);

/// Post-`abort` confirmation grace for the SPV run loop.
///
/// An abort only lands at the task's next await point, so a task parked in
/// synchronous host-callback code cannot be interrupted at all. Without this
/// bound the post-abort `handle.await` waits forever — the same hang the
/// graceful timeout above was meant to escape.
const SPV_ABORT_GRACE: Duration = Duration::from_secs(2);

/// How often [`SpvRuntime::wait_until_ready`] re-checks for a started client
/// with connected peers.
const SPV_READINESS_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Join a stopped SPV runner, escalating to cancellation after `timeout`.
///
/// Returns `None` once Tokio has confirmed the task terminated. Returns
/// `Some(handle)` when it is *still live* after the post-abort grace: the
/// caller must re-park that handle (so a teardown retry re-joins it rather
/// than silently detaching a callback-capable task) and report SPV as
/// non-clean.
#[must_use = "a returned handle is a still-live task that must be re-parked and reported non-clean"]
async fn join_spv_task(handle: JoinHandle<()>, timeout: Duration) -> Option<JoinHandle<()>> {
    join_spv_task_within(handle, timeout, SPV_ABORT_GRACE).await
}

/// [`join_spv_task`] with an explicit post-abort grace, so tests can drive
/// the survived-the-abort path without waiting out [`SPV_ABORT_GRACE`].
#[must_use = "a returned handle is a still-live task that must be re-parked and reported non-clean"]
async fn join_spv_task_within(
    mut handle: JoinHandle<()>,
    timeout: Duration,
    abort_grace: Duration,
) -> Option<JoinHandle<()>> {
    match tokio::time::timeout(timeout, &mut handle).await {
        Ok(Ok(())) => None,
        Ok(Err(error)) => {
            tracing::warn!(?error, "SPV background run loop join error");
            None
        }
        Err(_) => {
            tracing::warn!("SPV stop: background run loop did not unwind in time; aborting it");
            handle.abort();
            match tokio::time::timeout(abort_grace, &mut handle).await {
                Ok(Err(error)) if !error.is_cancelled() => {
                    tracing::warn!(?error, "SPV background run loop abort join error");
                    None
                }
                Ok(_) => None,
                Err(_) => {
                    tracing::warn!(
                        "SPV stop: background run loop survived abort for {abort_grace:?}; \
                         keeping the handle for a teardown retry"
                    );
                    Some(handle)
                }
            }
        }
    }
}

/// SPV client runtime — owns the `DashSpvClient` and drives sync.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// handlers by reference (no cloning).
pub struct SpvRuntime {
    event_manager: Arc<PlatformEventManager>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    client: RwLock<Option<SpvClient>>,
    last_config: RwLock<Option<ClientConfig>>,
    task: Mutex<Option<JoinHandle<()>>>,
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
            peer_tracker: Arc::new(PeerTracker::default()),
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

    /// Check whether the SPV client has been started.
    pub fn is_started(&self) -> bool {
        self.client.try_read().map(|c| c.is_some()).unwrap_or(false)
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

        let result = client
            .run()
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()));

        let mut client = self.client.write().await;
        let _ = client.take();
        self.peer_tracker.clear();

        result
    }

    /// Stop SPV sync gracefully. Unlocks the data dir safely.
    ///
    /// **Every phase is bounded** — `stop` runs on the path
    /// [`PlatformWalletManager::shutdown`](crate::manager::PlatformWalletManager::shutdown)
    /// and therefore the FFI's `destroy` must return through, so a wedged
    /// host callback has to surface as an error rather than hang teardown:
    /// the client stop is capped at [`SPV_CLIENT_STOP_BUDGET`], the run-loop
    /// join at [`SPV_STOP_TIMEOUT`] with an [`SPV_ABORT_GRACE`] post-abort
    /// confirmation. A run loop that outlives all of that is **re-parked**,
    /// not detached, so a teardown retry re-joins it — and the error return
    /// keeps it out of a clean shutdown verdict.
    ///
    /// Idempotent: a second call finds no client and re-joins whatever the
    /// first call re-parked.
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        let taken = {
            let mut client = self.client.write().await;
            client.take()
        };

        let stop_result = match taken {
            Some(c) => match tokio::time::timeout(SPV_CLIENT_STOP_BUDGET, c.stop()).await {
                Ok(result) => result.map_err(|e| PlatformWalletError::SpvError(e.to_string())),
                Err(_) => {
                    // The client is dropped with the timed-out future. The
                    // data-dir lock may outlive this call, which is strictly
                    // better than never returning from `destroy`.
                    tracing::warn!(
                        "SPV client stop did not complete within {:?}; abandoning it",
                        SPV_CLIENT_STOP_BUDGET
                    );
                    Err(PlatformWalletError::SpvError(format!(
                        "SPV client stop did not complete within {SPV_CLIENT_STOP_BUDGET:?}"
                    )))
                }
            },
            None => Ok(()),
        };
        self.peer_tracker.clear();

        let handle = self.task.lock().expect("spv task mutex poisoned").take();
        let join_result = match handle {
            None => Ok(()),
            Some(handle) => match join_spv_task(handle, SPV_STOP_TIMEOUT).await {
                None => Ok(()),
                Some(live) => {
                    // Re-park rather than drop: dropping a `JoinHandle`
                    // detaches the task, and this one can still reach host
                    // callbacks. Keeping it lets a teardown retry re-join.
                    *self.task.lock().expect("spv task mutex poisoned") = Some(live);
                    Err(PlatformWalletError::SpvError(
                        "SPV background run loop did not terminate after abort; \
                         it is still tracked for a retry"
                            .to_string(),
                    ))
                }
            },
        };

        // A failed client stop is the more informative diagnosis, so it wins;
        // either one makes the caller's shutdown verdict non-clean.
        stop_result.and(join_result)
    }

    /// Spawn the sync loop of an already-[`start`]ed client on the current
    /// tokio runtime and return immediately.
    ///
    /// Call [`stop`] to stop it
    pub fn spawn_run_loop(self: &Arc<Self>) {
        {
            let existing = self.task.lock().expect("spv task mutex poisoned");
            if existing.is_some() {
                tracing::warn!(
                    "spawn_in_background called while a task is already running; ignoring"
                );
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
    /// If the SPVClient is running it will be stopped and the
    /// storage will be cleaned. If it is not running a tmp
    /// Storage Manager built from the cached config will be used.
    pub async fn clear_storage(&self) -> Result<(), PlatformWalletError> {
        // Fast path: a live client holds the storage lock; clear through it.
        {
            let client_guard = self.client.read().await;
            if let Some(client) = client_guard.as_ref() {
                return client
                    .clear_storage()
                    .await
                    .map_err(|e| PlatformWalletError::SpvError(e.to_string()));
            }
        }

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
        StorageManager::shutdown(&mut storage).await;

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
        let masternodes = entries
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
                (pro_tx_hash, entry.into())
            })
            .collect();
        MasternodeList::build(
            masternodes,
            Default::default(),
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

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_spv_task_is_aborted_and_joined_before_return() {
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_in_task = Arc::clone(&dropped);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _flag = DropFlag(dropped_in_task);
            let _ = started_tx.send(());
            std::future::pending::<()>().await;
        });
        started_rx.await.expect("SPV task should start");

        assert!(
            join_spv_task(handle, SPV_STOP_TIMEOUT).await.is_none(),
            "an abortable task must be confirmed terminated, not returned as live"
        );

        assert!(
            dropped.load(Ordering::SeqCst),
            "abort must be joined so task-owned callback state is dropped"
        );
    }

    /// A run loop parked in **synchronous** code cannot be interrupted by
    /// `abort` — the cancellation only lands at the next await point, which
    /// never comes. The post-abort confirmation must therefore be bounded
    /// and hand the still-live handle back, so `stop` can re-park it (a
    /// dropped `JoinHandle` detaches the task, and this one can still reach
    /// host callbacks) and report SPV non-clean.
    ///
    /// Without the post-abort deadline this test hangs: that is exactly the
    /// hang that would reach the FFI's `destroy`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spv_task_surviving_abort_is_returned_for_reparking() {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _ = started_tx.send(());
            // Stands in for a monitor blocked inside a synchronous host
            // callback: no await point for the abort to land on.
            std::thread::sleep(Duration::from_millis(500));
        });
        started_rx.await.expect("SPV task should start");

        let live =
            join_spv_task_within(handle, Duration::from_millis(10), Duration::from_millis(20))
                .await
                .expect("an un-abortable task must be handed back, never silently detached");

        // Cleanup: the blocking section does end eventually.
        let _ = live.await;
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
    use std::sync::Arc;
    use std::time::Duration;

    use dash_spv::error::{NetworkError, SpvError};
    use dashcore::Network;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use super::{classify_spv_send_error, SpvRuntime};
    use crate::broadcaster::BroadcastError;
    use crate::events::PlatformEventManager;
    use crate::wallet::platform_wallet::PlatformWalletInfo;

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
