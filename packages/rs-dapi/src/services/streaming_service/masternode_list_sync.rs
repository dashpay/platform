use std::sync::Arc;

use ciborium::ser::into_writer;
use dashcore_rpc::dashcore::hashes::Hash as HashTrait;
use dashcore_rpc::dashcore::BlockHash;
use tokio::sync::{Mutex, Notify, RwLock};
use tracing::{debug, info, warn};

use crate::clients::CoreClient;
use crate::error::{DAPIResult, DapiError};
use crate::services::streaming_service::{FilterType, StreamingEvent, SubscriberManager};

#[derive(Default)]
struct MasternodeState {
    block_hash: Option<BlockHash>,
    block_height: Option<u32>,
    full_diff: Option<Vec<u8>>,
}

/// Manages masternode list synchronization and diff emission.
pub struct MasternodeListSync {
    core_client: CoreClient,
    subscriber_manager: Arc<SubscriberManager>,
    state: RwLock<MasternodeState>,
    update_lock: Mutex<()>,
    ready_notify: Notify,
}

impl MasternodeListSync {
    pub fn new(core_client: CoreClient, subscriber_manager: Arc<SubscriberManager>) -> Self {
        Self {
            core_client,
            subscriber_manager,
            state: RwLock::new(MasternodeState::default()),
            update_lock: Mutex::new(()),
            ready_notify: Notify::new(),
        }
    }

    pub fn spawn_initial_sync(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            match this.sync_best_chain_lock().await {
                Ok(true) => {
                    info!("Initial masternode list sync completed");
                }
                Ok(false) => {
                    debug!("No chain lock available yet for initial masternode list sync");
                }
                Err(err) => {
                    warn!("Failed to perform initial masternode list sync: {}", err);
                }
            }
        });
    }

    pub fn start_chain_lock_listener(self: &Arc<Self>, subscriber_manager: Arc<SubscriberManager>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let handle = subscriber_manager
                .add_subscription(FilterType::CoreChainLocks)
                .await;

            while let Some(event) = handle.recv().await {
                if let StreamingEvent::CoreChainLock { .. } = event {
                    this.handle_chain_lock_notification().await;
                }
            }
            debug!("Chain lock listener stopped");
        });
    }

    pub async fn ensure_ready(&self) -> DAPIResult<()> {
        if self.state.read().await.full_diff.is_some() {
            return Ok(());
        }

        if self.sync_best_chain_lock().await? {
            return Ok(());
        }

        self.ready_notify.notified().await;
        Ok(())
    }

    pub async fn current_full_diff(&self) -> Option<Vec<u8>> {
        self.state
            .read()
            .await
            .full_diff
            .as_ref()
            .map(|diff| diff.clone())
    }

    pub async fn handle_chain_lock_notification(&self) {
        match self.sync_best_chain_lock().await {
            Ok(true) => {}
            Ok(false) => {
                debug!("Chain lock notification received but no best chain lock available yet");
            }
            Err(err) => {
                warn!("Failed to sync masternode list on chain lock: {}", err);
            }
        }
    }

    async fn sync_best_chain_lock(&self) -> DAPIResult<bool> {
        match self.core_client.get_best_chain_lock().await? {
            Some(chain_lock) => {
                self.sync_to_chain_lock(chain_lock.block_hash, chain_lock.block_height)
                    .await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn sync_to_chain_lock(&self, block_hash: BlockHash, height: u32) -> DAPIResult<()> {
        let _guard = self.update_lock.lock().await;

        if self
            .state
            .read()
            .await
            .block_hash
            .as_ref()
            .filter(|current| *current == &block_hash)
            .is_some()
        {
            debug!("Masternode list already synced for block {}", block_hash);
            return Ok(());
        }

        let previous_state = self.state.read().await;
        let previous_hash = previous_state.block_hash.clone();
        drop(previous_state);

        let full_diff = self.fetch_diff(None, &block_hash).await?;

        let diff_bytes = if let Some(prev) = previous_hash.clone() {
            if prev == block_hash {
                None
            } else {
                Some(self.fetch_diff(Some(&prev), &block_hash).await?)
            }
        } else {
            None
        };

        {
            let mut state = self.state.write().await;
            state.block_hash = Some(block_hash);
            state.block_height = Some(height);
            state.full_diff = Some(full_diff.clone());
        }

        let payload = diff_bytes.unwrap_or_else(|| full_diff.clone());
        self.subscriber_manager
            .notify(StreamingEvent::CoreMasternodeListDiff { data: payload })
            .await;

        self.ready_notify.notify_waiters();

        info!(
            %block_hash,
            height,
            "Masternode list synchronized"
        );

        Ok(())
    }

    async fn fetch_diff(&self, base: Option<&BlockHash>, block: &BlockHash) -> DAPIResult<Vec<u8>> {
        let base_hash = base.cloned().unwrap_or_else(Self::null_block_hash);
        let diff = self.core_client.mn_list_diff(&base_hash, block).await?;

        let mut buffer = Vec::new();
        into_writer(&diff, &mut buffer)
            .map_err(|e| DapiError::internal(format!("failed to encode masternode diff: {}", e)))?;

        Ok(buffer)
    }

    fn null_block_hash() -> BlockHash {
        BlockHash::from_slice(&[0u8; 32]).expect("zero block hash")
    }
}
