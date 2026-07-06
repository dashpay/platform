//! SPV client runtime — manages the DashSpvClient lifecycle.

use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::sync::SyncProgress;
use dash_spv::{ClientConfig, DashSpvClient, EventHandler, Hash};

use key_wallet_manager::WalletManager;

use crate::broadcaster::BroadcastError;
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
    task: Mutex<Option<JoinHandle<()>>>,
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
            task: Mutex::new(None),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dash_spv::error::{NetworkError, SpvError};
    use dashcore::Network;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use super::{classify_spv_broadcast_error, SpvRuntime};
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
}
