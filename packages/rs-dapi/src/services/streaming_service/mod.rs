// Streaming service modular implementation
// This module handles real-time streaming of blockchain data from ZMQ to gRPC clients

mod block_header_stream;
mod bloom;
mod masternode_list_stream;
mod masternode_list_sync;
mod subscriber_manager;
mod transaction_stream;
mod zmq_listener;

use crate::clients::CoreClient;
use crate::clients::traits::TenderdashClientTrait;
use crate::config::Config;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep};
use tracing::{error, info, trace, warn};

pub(crate) use masternode_list_sync::MasternodeListSync;
pub(crate) use subscriber_manager::{
    FilterType, StreamingEvent, SubscriberManager, SubscriptionHandle,
};
pub(crate) use zmq_listener::{ZmqEvent, ZmqListener, ZmqListenerTrait};

/// Streaming service implementation with ZMQ integration
#[derive(Clone)]
pub struct StreamingServiceImpl {
    pub drive_client: crate::clients::drive_client::DriveClient,
    pub tenderdash_client: Arc<dyn TenderdashClientTrait>,
    pub core_client: CoreClient,
    pub config: Arc<Config>,
    pub zmq_listener: Arc<dyn ZmqListenerTrait>,
    pub subscriber_manager: Arc<SubscriberManager>,
    pub masternode_list_sync: Arc<MasternodeListSync>,
    /// Background workers; aborted when the last reference is dropped
    pub workers: Arc<JoinSet<()>>,
}

impl StreamingServiceImpl {
    pub fn new(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        core_client: CoreClient,
        config: Arc<Config>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!("Creating streaming service with ZMQ listener");
        let zmq_listener: Arc<dyn ZmqListenerTrait> =
            Arc::new(ZmqListener::new(&config.dapi.core.zmq_url)?);

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
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        core_client: CoreClient,
        config: Arc<Config>,
        zmq_listener: Arc<dyn ZmqListenerTrait>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!("Creating streaming service with custom ZMQ listener");
        let subscriber_manager = Arc::new(SubscriberManager::new());
        let masternode_list_sync = Arc::new(MasternodeListSync::new(
            core_client.clone(),
            subscriber_manager.clone(),
        ));
        masternode_list_sync.spawn_initial_sync();
        masternode_list_sync.start_chain_lock_listener(subscriber_manager.clone());

        // Prepare background workers set
        let mut workers = JoinSet::new();

        // Spawn Core ZMQ subscribe + process loop
        workers.spawn(Self::core_zmq_subscription_worker(
            zmq_listener.clone(),
            subscriber_manager.clone(),
        ));

        // Spawn Tenderdash transaction forwarder worker
        let td_client = tenderdash_client.clone();
        let sub_mgr = subscriber_manager.clone();
        workers.spawn(Self::tenderdash_transactions_subscription_worker(
            td_client, sub_mgr,
        ));
        let td_client = tenderdash_client.clone();
        let sub_mgr = subscriber_manager.clone();
        workers.spawn(Self::tenderdash_block_subscription_worker(
            td_client, sub_mgr,
        ));

        info!("Started streaming service background tasks");

        Ok(Self {
            drive_client,
            tenderdash_client,
            core_client,
            config,
            zmq_listener,
            subscriber_manager,
            masternode_list_sync,
            workers: Arc::new(workers),
        })
    }

    /// Background worker: subscribe to Tenderdash transactions and forward to subscribers
    async fn tenderdash_transactions_subscription_worker(
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting Tenderdash tx forwarder loop");
        let mut transaction_rx = tenderdash_client.subscribe_to_transactions();
        loop {
            match transaction_rx.recv().await {
                Ok(event) => {
                    subscriber_manager
                        .notify(StreamingEvent::PlatformTx { event })
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "Tenderdash event receiver lagged, skipped {} events",
                        skipped
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("Tenderdash event receiver closed");
                    break;
                }
            }
        }
    }

    /// Background worker: subscribe to Tenderdash transactions and forward to subscribers
    async fn tenderdash_block_subscription_worker(
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        trace!("Starting Tenderdash block forwarder loop");
        let mut block_rx = tenderdash_client.subscribe_to_blocks();
        loop {
            match block_rx.recv().await {
                Ok(event) => {
                    subscriber_manager
                        .notify(StreamingEvent::PlatformBlock { event })
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "Tenderdash block event receiver lagged, skipped {} events",
                        skipped
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("Tenderdash block event receiver closed");
                    break;
                }
            }
        }
    }

    /// Background worker: subscribe to ZMQ and process events, with retry/backoff
    async fn core_zmq_subscription_worker(
        zmq_listener: Arc<dyn ZmqListenerTrait>,
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
                    warn!("ZMQ event processing ended; restarting after {:?}", backoff);
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
                Err(e) => {
                    error!("ZMQ subscribe failed: {}", e);
                    warn!("Retrying ZMQ subscribe in {:?}", backoff);
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
        while let Ok(event) = zmq_events.recv().await {
            match event {
                ZmqEvent::RawTransaction { data } => {
                    trace!("Processing raw transaction event");
                    subscriber_manager
                        .notify(StreamingEvent::CoreRawTransaction { data })
                        .await;
                }
                ZmqEvent::RawBlock { data } => {
                    trace!("Processing raw block event");
                    subscriber_manager
                        .notify(StreamingEvent::CoreRawBlock { data })
                        .await;
                }
                ZmqEvent::RawTransactionLock { data } => {
                    trace!("Processing transaction lock event");
                    subscriber_manager
                        .notify(StreamingEvent::CoreInstantLock { data })
                        .await;
                }
                ZmqEvent::RawChainLock { data } => {
                    trace!("Processing chain lock event");
                    subscriber_manager
                        .notify(StreamingEvent::CoreChainLock { data })
                        .await;
                }
                ZmqEvent::HashBlock { hash } => {
                    trace!("Processing new block hash event");
                    subscriber_manager
                        .notify(StreamingEvent::CoreNewBlockHash { hash })
                        .await;
                }
            }
        }
        trace!("ZMQ event processing loop ended");
    }

    /// Returns current health of the ZMQ streaming pipeline
    pub fn is_healthy(&self) -> bool {
        self.zmq_listener.is_connected()
    }
}
