//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{PubkeyHash, QuorumHash, Transaction};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::{DiskStorageManager, StorageManager};
use dash_spv::sync::SyncProgress;
use dash_spv::{BroadcastResult, ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::broadcaster::BroadcastError;
use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::spv::peers::{classify_peers, PeerTracker, SpvPeerInfo};
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient =
    DashSpvClient<WalletManager<PlatformWalletInfo>, PeerNetworkManager, DiskStorageManager>;

const SPV_STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// Join a stopped SPV runner, escalating to cancellation after `timeout` but
/// never returning until Tokio confirms that the task has terminated.
async fn join_spv_task(mut handle: JoinHandle<()>, timeout: Duration) {
    match tokio::time::timeout(timeout, &mut handle).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            tracing::warn!(?error, "SPV background run loop join error");
        }
        Err(_) => {
            tracing::warn!("SPV stop: background run loop did not unwind in time; aborting it");
            handle.abort();
            if let Err(error) = handle.await {
                if !error.is_cancelled() {
                    tracing::warn!(?error, "SPV background run loop abort join error");
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

    /// Stop SPV sync gracefully. Unlocks the data dir safely
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
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
        self.peer_tracker.clear();

        let handle = self.task.lock().expect("spv task mutex poisoned").take();
        if let Some(handle) = handle {
            join_spv_task(handle, SPV_STOP_TIMEOUT).await;
        }

        stop_result
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

        list.masternodes_by_voting_key(voting_key_id)
            .into_iter()
            .map(|pro_tx| {
                let mut out = [0u8; 32];
                out.copy_from_slice(pro_tx.as_ref());
                out
            })
            .collect()
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

        join_spv_task(handle, SPV_STOP_TIMEOUT).await;

        assert!(
            dropped.load(Ordering::SeqCst),
            "abort must be joined so task-owned callback state is dropped"
        );
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
