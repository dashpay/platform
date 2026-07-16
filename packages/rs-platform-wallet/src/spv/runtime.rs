//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use dashcore::sml::llmq_entry_verification::LLMQEntryVerificationStatus;
use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::{DiskStorageManager, StorageManager};
use dash_spv::sync::{SyncProgress, SyncState};
use dash_spv::{ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::broadcaster::BroadcastError;
use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::spv::peers::{classify_peers, PeerTracker, SpvPeerInfo};
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient =
    DashSpvClient<WalletManager<PlatformWalletInfo>, PeerNetworkManager, DiskStorageManager>;

/// Build the masternode-engine lookup key from a proof-supplied quorum hash.
///
/// The SDK's proof verifier supplies the quorum hash in display (big-endian)
/// byte order — the same order the trusted HTTP provider matches against — but
/// the masternode engine keys its quorum map in internal (little-endian) order.
/// The bytes must therefore be reversed before building the [`QuorumHash`] key;
/// without this every real quorum misses. Verified against a synced testnet
/// node: the engine stores the reversed form of each requested hash (e.g.
/// requested `0000…7f`, stored `…000000`).
///
/// [`SpvRuntime::get_quorum_public_key`] calls this so the byte-order regression
/// test exercises the exact transform the production lookup uses.
fn quorum_lookup_key(display_order: [u8; 32]) -> QuorumHash {
    let mut internal_order = display_order;
    internal_order.reverse();
    QuorumHash::from_byte_array(internal_order)
}

fn verified_quorum_public_key(
    status: &LLMQEntryVerificationStatus,
    public_key: [u8; 48],
) -> Result<[u8; 48], PlatformWalletError> {
    if matches!(status, LLMQEntryVerificationStatus::Verified) {
        Ok(public_key)
    } else {
        Err(PlatformWalletError::SpvError(format!(
            "quorum entry is not cryptographically verified: {status:?}"
        )))
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
/// Classify a dash-spv broadcast failure per the
/// [`TransactionBroadcaster::broadcast`] contract.
///
/// dash-spv's `broadcast_transaction` raises
/// `NetworkError::NotConnected` from its zero-connected-peers check
/// *before* handing the transaction to any peer, so it is the only
/// error safe to classify [`BroadcastError::Rejected`]; anything else
/// may follow a partial peer send and must stay
/// [`BroadcastError::MaybeSent`]. Pinned by the tests below so a
/// dash-spv semantic change is caught at this crate's boundary.
///
/// [`TransactionBroadcaster::broadcast`]: crate::broadcaster::TransactionBroadcaster::broadcast
fn classify_spv_broadcast_error(error: dash_spv::error::SpvError) -> BroadcastError {
    use dash_spv::error::{NetworkError, SpvError};

    match error {
        SpvError::Network(NetworkError::NotConnected) => BroadcastError::Rejected {
            reason: "SPV broadcast failed: no connected peers".to_string(),
        },
        other => BroadcastError::MaybeSent {
            reason: format!("SPV broadcast failed: {}", other),
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

    /// Whether dash-spv's current progress reports both header and
    /// masternode-list synchronization complete.
    pub async fn is_ready(&self) -> bool {
        self.sync_progress().await.is_some_and(|progress| {
            progress
                .headers()
                .is_ok_and(|headers| headers.state() == SyncState::Synced)
                && progress
                    .masternodes()
                    .is_ok_and(|masternodes| masternodes.state() == SyncState::Synced)
        })
    }

    /// Broadcast a transaction to all connected SPV peers.
    ///
    /// Failures are classified per the [`TransactionBroadcaster::broadcast`]
    /// contract: an unstarted client and dash-spv's zero-peer
    /// `NetworkError::NotConnected` both fire before any bytes leave the
    /// process, so they are [`BroadcastError::Rejected`]; any later failure
    /// may follow a partial peer send and is [`BroadcastError::MaybeSent`].
    ///
    /// [`TransactionBroadcaster::broadcast`]: crate::broadcaster::TransactionBroadcaster::broadcast
    pub(crate) async fn broadcast_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<(), BroadcastError> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or(BroadcastError::Rejected {
            reason: "SPV client not started".to_string(),
        })?;

        client
            .broadcast_transaction(tx)
            .await
            .map_err(classify_spv_broadcast_error)?;

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
        let qh = quorum_lookup_key(quorum_hash);

        let quorum = client
            .get_quorum_at_height(height, llmq_type, qh)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        verified_quorum_public_key(
            &quorum.verified,
            *quorum.quorum_entry.quorum_public_key.as_ref(),
        )
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
    use dashcore::sml::llmq_entry_verification::{
        LLMQEntryVerificationSkipStatus, LLMQEntryVerificationStatus,
    };
    use dashcore::sml::quorum_validation_error::QuorumValidationError;
    use dashcore::Network;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use super::{
        classify_spv_broadcast_error, quorum_lookup_key, verified_quorum_public_key, SpvRuntime,
    };
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

    /// An unstarted SPV client fails before any bytes leave the process,
    /// so the failure must classify `Rejected` (safe to release the
    /// transaction's input reservation).
    #[tokio::test]
    async fn broadcast_on_unstarted_client_is_rejected() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        let runtime = SpvRuntime::new(wallet_manager, Arc::new(PlatformEventManager::new(vec![])));

        let result = runtime.broadcast_transaction(&dummy_tx()).await;
        assert!(
            matches!(result, Err(BroadcastError::Rejected { .. })),
            "unstarted client must classify Rejected, got {result:?}"
        );
    }

    /// dash-spv raises `NetworkError::NotConnected` from its
    /// zero-connected-peers check before handing the transaction to any
    /// peer, so it is the one client error safe to classify `Rejected`.
    /// If dash-spv ever starts raising `NotConnected` after a partial
    /// send, this pin must be revisited — releasing on a post-send
    /// failure reopens the double-spend-on-retry window.
    #[test]
    fn not_connected_classifies_rejected() {
        let result = classify_spv_broadcast_error(SpvError::Network(NetworkError::NotConnected));
        assert!(
            matches!(result, BroadcastError::Rejected { .. }),
            "NotConnected must classify Rejected, got {result:?}"
        );
    }

    /// Every other dash-spv error may follow a partial peer send and must
    /// stay `MaybeSent`, keeping the reservation for the TTL backstop.
    #[test]
    fn any_other_spv_error_classifies_maybe_sent() {
        for error in [
            SpvError::Network(NetworkError::Timeout),
            SpvError::Network(NetworkError::PeerDisconnected),
            SpvError::Network(NetworkError::ConnectionFailed("reset by peer".to_string())),
            SpvError::Config("bad config".to_string()),
        ] {
            let rendered = error.to_string();
            let result = classify_spv_broadcast_error(error);
            assert!(
                matches!(result, BroadcastError::MaybeSent { .. }),
                "{rendered} must classify MaybeSent, got {result:?}"
            );
        }
    }

    /// Regression guard for the quorum-hash byte order used by
    /// [`SpvRuntime::get_quorum_public_key`].
    ///
    /// This exercises the production transform directly — [`quorum_lookup_key`]
    /// is the exact function `get_quorum_public_key` calls to build its engine
    /// lookup key — so dropping (or re-introducing a spurious) reversal there
    /// fails this test, rather than the test asserting standalone `BTreeMap`
    /// semantics that can't detect a change in the real code.
    ///
    /// The masternode engine keys its quorum map — `quorum_entry_of_type_for_quorum_hash`,
    /// a `BTreeMap<QuorumHash, _>::get` — in internal byte order, but the SDK
    /// proof verifier supplies the hash in display (reversed) order. Verified
    /// end-to-end against a synced testnet node: the engine stores the reversed
    /// form of each requested hash (requested `0000…`, stored `…0000`), so a
    /// non-reversed lookup misses every real quorum and falls through to
    /// fail-closed rejection (previously masked by the trusted-quorum fallback).
    #[test]
    fn quorum_hash_reversed_to_internal_order_before_lookup() {
        use dashcore::hashes::Hash;
        use dashcore::QuorumHash;
        use std::collections::BTreeMap;

        // The hash as the proof verifier supplies it (display order).
        let display_bytes: [u8; 32] = std::array::from_fn(|i| (i as u8) + 1);
        // The engine keys the quorum under the internal (reversed) order.
        let mut internal_bytes = display_bytes;
        internal_bytes.reverse();

        let pubkey = [0xABu8; 48];
        let mut quorums: BTreeMap<QuorumHash, [u8; 48]> = BTreeMap::new();
        quorums.insert(QuorumHash::from_byte_array(internal_bytes), pubkey);

        // The production key builder must land on the internally-keyed quorum.
        assert_eq!(
            quorums.get(&quorum_lookup_key(display_bytes)),
            Some(&pubkey),
            "quorum_lookup_key must reverse display order to the engine's internal key"
        );

        // Sanity: the un-reversed display-order key is absent — this is exactly
        // the miss that `quorum_lookup_key`'s reversal exists to prevent, so if
        // the reversal is removed the assertion above fails.
        assert_eq!(
            quorums.get(&QuorumHash::from_byte_array(display_bytes)),
            None,
            "using the hash without reversal must miss — this was the regression"
        );
    }

    #[test]
    fn only_verified_quorums_can_supply_a_public_key() {
        let public_key = [0xAB; 48];
        assert_eq!(
            verified_quorum_public_key(&LLMQEntryVerificationStatus::Verified, public_key).unwrap(),
            public_key
        );

        for status in [
            LLMQEntryVerificationStatus::Unknown,
            LLMQEntryVerificationStatus::Skipped(
                LLMQEntryVerificationSkipStatus::NotMarkedForVerification,
            ),
            LLMQEntryVerificationStatus::Invalid(QuorumValidationError::InvalidQuorumPublicKey),
        ] {
            let error = verified_quorum_public_key(&status, public_key)
                .unwrap_err()
                .to_string();
            assert!(error.contains("not cryptographically verified"));
            assert!(error.contains(&format!("{status:?}")));
        }
    }
}
