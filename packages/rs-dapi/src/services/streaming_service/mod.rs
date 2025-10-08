// Streaming service modular implementation
// This module handles real-time streaming of blockchain data from ZMQ to gRPC clients

mod block_header_stream;
mod bloom;
mod masternode_list_stream;
mod masternode_list_sync;
mod subscriber_manager;
mod transaction_stream;
mod zmq_listener;

use crate::DapiError;
use crate::clients::{CoreClient, TenderdashClient};
use crate::config::Config;
use crate::sync::Workers;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};
use tracing::{debug, trace};

pub(crate) use masternode_list_sync::MasternodeListSync;
pub(crate) use subscriber_manager::{
    FilterType, StreamingEvent, SubscriberManager, SubscriptionHandle,
};
pub(crate) use zmq_listener::{ZmqEvent, ZmqListener};

/// Streaming service implementation with ZMQ integration.
///
/// Cheap cloning is supported, and will create references to the same background workers.
/// Doesn't store any state itself; all state is in the background workers.
#[derive(Clone)]
pub struct StreamingServiceImpl {
    pub drive_client: crate::clients::drive_client::DriveClient,
    pub tenderdash_client: Arc<TenderdashClient>,
    pub core_client: CoreClient,
    pub config: Arc<Config>,
    pub zmq_listener: Arc<ZmqListener>,
    pub subscriber_manager: Arc<SubscriberManager>,
    pub masternode_list_sync: Arc<MasternodeListSync>,
    /// Background workers; aborted when the last reference is dropped
    pub workers: Workers,
}

impl StreamingServiceImpl {
    // --- Small helpers for concise logging across submodules ---
    /// Attempt to decode transaction bytes and return the txid as hex.
    pub(crate) fn txid_hex_from_bytes(bytes: &[u8]) -> Option<String> {
        use dashcore_rpc::dashcore::Transaction as CoreTx;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;
        deserialize::<CoreTx>(bytes)
            .ok()
            .map(|tx| tx.txid().to_string())
    }

    /// Decode transaction bytes and return the txid in raw byte form.
    pub(crate) fn txid_bytes_from_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
        use dashcore_rpc::dashcore::Transaction as CoreTx;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;
        use dashcore_rpc::dashcore::hashes::Hash as DashHash;

        deserialize::<CoreTx>(bytes)
            .ok()
            .map(|tx| tx.txid().to_byte_array().to_vec())
    }

    /// Decode block bytes and return the block hash in hex.
    pub(crate) fn block_hash_hex_from_block_bytes(bytes: &[u8]) -> Option<String> {
        use dashcore_rpc::dashcore::Block as CoreBlock;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;
        deserialize::<CoreBlock>(bytes)
            .ok()
            .map(|b| b.block_hash().to_string())
    }

    /// Return a short hexadecimal prefix of the provided bytes for logging.
    pub(crate) fn short_hex(bytes: &[u8], take: usize) -> String {
        let len = bytes.len().min(take);
        let mut s = hex::encode(&bytes[..len]);
        if bytes.len() > take {
            s.push('…');
        }
        s
    }

    /// Format a human-readable description of a streaming event for logs.
    pub(crate) fn summarize_streaming_event(event: &StreamingEvent) -> String {
        match event {
            StreamingEvent::CoreRawTransaction { data } => {
                if let Some(txid) = Self::txid_hex_from_bytes(data) {
                    format!("CoreRawTransaction txid={} size={}", txid, data.len())
                } else {
                    format!(
                        "CoreRawTransaction size={} bytes prefix={}",
                        data.len(),
                        Self::short_hex(data, 12)
                    )
                }
            }
            StreamingEvent::CoreRawBlock { data } => {
                if let Some(hash) = Self::block_hash_hex_from_block_bytes(data) {
                    format!("CoreRawBlock hash={} size={}", hash, data.len())
                } else {
                    format!(
                        "CoreRawBlock size={} bytes prefix={}",
                        data.len(),
                        Self::short_hex(data, 12)
                    )
                }
            }
            StreamingEvent::CoreInstantLock { data } => {
                format!("CoreInstantLock size={} bytes", data.len())
            }
            StreamingEvent::CoreChainLock { data } => {
                format!("CoreChainLock size={} bytes", data.len())
            }
            StreamingEvent::CoreNewBlockHash { hash } => {
                format!("CoreNewBlockHash {}", Self::short_hex(hash, 12))
            }
            StreamingEvent::PlatformTx { event } => {
                // `hash` is already a string on TD events
                format!("PlatformTx hash={} height={}", event.hash, event.height)
            }
            StreamingEvent::PlatformBlock { .. } => "PlatformBlock".to_string(),
            StreamingEvent::CoreMasternodeListDiff { data } => {
                format!("CoreMasternodeListDiff size={} bytes", data.len())
            }
        }
    }

    /// Describe a ZMQ event in a concise logging-friendly string.
    pub(crate) fn summarize_zmq_event(event: &ZmqEvent) -> String {
        match event {
            ZmqEvent::RawTransaction { data } => {
                if let Some(txid) = Self::txid_hex_from_bytes(data) {
                    format!("RawTransaction txid={} size={}", txid, data.len())
                } else {
                    format!(
                        "RawTransaction size={} bytes prefix={}",
                        data.len(),
                        Self::short_hex(data, 12)
                    )
                }
            }
            ZmqEvent::RawBlock { data } => {
                if let Some(hash) = Self::block_hash_hex_from_block_bytes(data) {
                    format!("RawBlock hash={} size={}", hash, data.len())
                } else {
                    format!(
                        "RawBlock size={} bytes prefix={}",
                        data.len(),
                        Self::short_hex(data, 12)
                    )
                }
            }
            ZmqEvent::RawTransactionLock { data } => {
                format!("RawTransactionLock size={} bytes", data.len())
            }
            ZmqEvent::RawChainLock { data } => {
                format!("RawChainLock size={} bytes", data.len())
            }
            ZmqEvent::HashBlock { hash } => {
                format!("HashBlock {}", Self::short_hex(hash, 12))
            }
        }
    }
    /// Construct the streaming service with default ZMQ listener and background workers.
    pub fn new(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<TenderdashClient>,
        core_client: CoreClient,
        config: Arc<Config>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!(
            zmq_url = %config.dapi.core.zmq_url,
            "Creating streaming service with default ZMQ listener"
        );
        let zmq_listener = Arc::new(ZmqListener::new(&config.dapi.core.zmq_url)?);

        Self::create_with_common_setup(
            drive_client,
            tenderdash_client,
            core_client,
            config,
            zmq_listener,
        )
    }

    /// Create a new streaming service with a custom ZMQ listener (useful for testing)
    fn create_with_common_setup(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<TenderdashClient>,
        core_client: CoreClient,
        config: Arc<Config>,
        zmq_listener: Arc<ZmqListener>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!(
            zmq_url = %config.dapi.core.zmq_url,
            "Creating streaming service with provided ZMQ listener"
        );
        let subscriber_manager = Arc::new(SubscriberManager::new());
        let masternode_list_sync = Arc::new(MasternodeListSync::new(
            core_client.clone(),
            subscriber_manager.clone(),
        ));
        masternode_list_sync.spawn_initial_sync();
        masternode_list_sync.start_chain_lock_listener(subscriber_manager.clone());

        // Prepare background workers set
        let workers = Workers::new();

        // Spawn Core ZMQ subscribe + process loop
        let zmq_listener_clone = zmq_listener.clone();
        let subscriber_manager_clone = subscriber_manager.clone();
        workers.spawn(async move {
            Self::core_zmq_subscription_worker(zmq_listener_clone, subscriber_manager_clone).await;
            Ok::<(), DapiError>(())
        });

        // Spawn Tenderdash transaction forwarder worker
        let td_client = tenderdash_client.clone();
        let sub_mgr = subscriber_manager.clone();
        workers.spawn(async move {
            Self::tenderdash_transactions_subscription_worker(td_client, sub_mgr).await;
            Ok::<(), DapiError>(())
        });
        let td_client = tenderdash_client.clone();
        let sub_mgr = subscriber_manager.clone();
        workers.spawn(async move {
            Self::tenderdash_block_subscription_worker(td_client, sub_mgr).await;
            Ok::<(), DapiError>(())
        });

        trace!(
            zmq_url = %config.dapi.core.zmq_url,
            drive = %config.dapi.drive.uri,
            tenderdash_http = %config.dapi.tenderdash.uri,
            tenderdash_ws = %config.dapi.tenderdash.websocket_uri,
            "Started streaming service background tasks"
        );

        Ok(Self {
            drive_client,
            tenderdash_client,
            core_client,
            config,
            zmq_listener,
            subscriber_manager,
            masternode_list_sync,
            workers,
        })
    }

    /// Background worker: subscribe to Tenderdash transactions and forward to subscribers
    async fn tenderdash_transactions_subscription_worker(
        tenderdash_client: Arc<TenderdashClient>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting Tenderdash tx forwarder loop");
        let mut transaction_rx = tenderdash_client.subscribe_to_transactions();
        let mut forwarded_events: u64 = 0;
        loop {
            match transaction_rx.recv().await {
                Ok(event) => {
                    debug!(
                        hash = %event.hash,
                        height = event.height,
                        forwarded = forwarded_events,
                        "Forwarding Tenderdash transaction event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::PlatformTx { event })
                        .await;
                    forwarded_events = forwarded_events.saturating_add(1);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!(
                        "Tenderdash event receiver lagged, skipped {} events",
                        skipped
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!(
                        forwarded = forwarded_events,
                        "Tenderdash transaction event receiver closed"
                    );
                    break;
                }
            }
        }
        trace!(
            forwarded = forwarded_events,
            "Tenderdash tx forwarder loop exited"
        );
    }

    /// Background worker: subscribe to Tenderdash transactions and forward to subscribers
    async fn tenderdash_block_subscription_worker(
        tenderdash_client: Arc<TenderdashClient>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting Tenderdash block forwarder loop");
        let mut block_rx = tenderdash_client.subscribe_to_blocks();
        let mut forwarded_events: u64 = 0;
        loop {
            match block_rx.recv().await {
                Ok(event) => {
                    debug!(
                        forwarded = forwarded_events,
                        "Forwarding Tenderdash block event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::PlatformBlock { event })
                        .await;
                    forwarded_events = forwarded_events.saturating_add(1);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!(
                        "Tenderdash block event receiver lagged, skipped {} events",
                        skipped
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!(
                        forwarded = forwarded_events,
                        "Tenderdash block event receiver closed"
                    );
                    break;
                }
            }
        }
        trace!(
            forwarded = forwarded_events,
            "Tenderdash block forwarder loop exited"
        );
    }

    /// Background worker: subscribe to ZMQ and process events, with retry/backoff
    async fn core_zmq_subscription_worker(
        zmq_listener: Arc<ZmqListener>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting ZMQ subscribe/process loop");
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);
        loop {
            match zmq_listener.subscribe().await {
                Ok(zmq_events) => {
                    trace!("ZMQ listener started successfully, processing events");
                    Self::process_zmq_events(zmq_events, subscriber_manager.clone()).await;
                    // processing ended; mark unhealthy and retry after short delay
                    debug!("ZMQ event processing ended; restarting after {:?}", backoff);
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
                Err(e) => {
                    debug!("ZMQ subscribe failed: {}", e);
                    debug!("Retrying ZMQ subscribe in {:?}", backoff);
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
        }
    }

    /// Process ZMQ events and forward to matching subscribers
    async fn process_zmq_events(
        mut zmq_events: broadcast::Receiver<ZmqEvent>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting ZMQ event processing loop");
        let mut processed_events: u64 = 0;
        while let Ok(event) = zmq_events.recv().await {
            processed_events = processed_events.saturating_add(1);
            match event {
                ZmqEvent::RawTransaction { data } => {
                    let txid =
                        Self::txid_hex_from_bytes(&data).unwrap_or_else(|| "n/a".to_string());
                    trace!(
                        txid = %txid,
                        size = data.len(),
                        processed = processed_events,
                        "Processing raw transaction event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::CoreRawTransaction { data })
                        .await;
                }
                ZmqEvent::RawBlock { data } => {
                    let block_hash = Self::block_hash_hex_from_block_bytes(&data)
                        .unwrap_or_else(|| "n/a".to_string());
                    trace!(
                        block_hash = %block_hash,
                        size = data.len(),
                        processed = processed_events,
                        "Processing raw block event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::CoreRawBlock { data })
                        .await;
                }
                ZmqEvent::RawTransactionLock { data } => {
                    trace!(
                        size = data.len(),
                        processed = processed_events,
                        "Processing transaction lock event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::CoreInstantLock { data })
                        .await;
                }
                ZmqEvent::RawChainLock { data } => {
                    trace!(
                        size = data.len(),
                        processed = processed_events,
                        "Processing chain lock event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::CoreChainLock { data })
                        .await;
                }
                ZmqEvent::HashBlock { hash } => {
                    trace!(
                        size = hash.len(),
                        processed = processed_events,
                        "Processing new block hash event"
                    );
                    subscriber_manager
                        .notify(StreamingEvent::CoreNewBlockHash { hash })
                        .await;
                }
            }
        }
        trace!(
            processed = processed_events,
            "ZMQ event processing loop ended"
        );
    }

    /// Returns current health of the ZMQ streaming pipeline
    pub fn is_healthy(&self) -> bool {
        self.zmq_listener.is_running()
    }
}
