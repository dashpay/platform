// Streaming service modular implementation
// This module handles real-time streaming of blockchain data from ZMQ to gRPC clients

mod block_header_stream;
mod masternode_list_stream;
mod subscriber_manager;
mod transaction_filter;
mod transaction_stream;
mod zmq_listener;

use crate::clients::traits::TenderdashClientTrait;
use crate::config::Config;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tracing::{error, info, trace, warn};

pub(crate) use subscriber_manager::{
    FilterType, StreamingMessage, SubscriberManager, SubscriptionType,
};
pub(crate) use zmq_listener::{ZmqEvent, ZmqListener, ZmqListenerTrait};

/// Streaming service implementation with ZMQ integration
#[derive(Clone)]
pub struct StreamingServiceImpl {
    pub drive_client: crate::clients::drive_client::DriveClient,
    pub tenderdash_client: Arc<dyn TenderdashClientTrait>,
    pub config: Arc<Config>,
    pub zmq_listener: Arc<dyn ZmqListenerTrait>,
    pub subscriber_manager: Arc<SubscriberManager>,
    pub block_notify: broadcast::Sender<()>,
    /// Background workers; aborted when the last reference is dropped
    pub workers: Arc<JoinSet<()>>,
}

impl StreamingServiceImpl {
    pub fn new(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        config: Arc<Config>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!("Creating streaming service with ZMQ listener");
        let zmq_listener: Arc<dyn ZmqListenerTrait> =
            Arc::new(ZmqListener::new(&config.dapi.core.zmq_url)?);

        Self::create_with_common_setup(drive_client, tenderdash_client, config, zmq_listener)
    }

    /// Create a new streaming service with a custom ZMQ listener (useful for testing)
    fn create_with_common_setup(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        config: Arc<Config>,
        zmq_listener: Arc<dyn ZmqListenerTrait>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        trace!("Creating streaming service with custom ZMQ listener");
        let subscriber_manager = Arc::new(SubscriberManager::new());

        let (block_notify, _) = broadcast::channel(32);

        // Prepare background workers set
        let mut workers = JoinSet::new();

        // Spawn ZMQ subscribe + process loop
        workers.spawn(Self::zmq_subscribe_and_process_worker(
            zmq_listener.clone(),
            subscriber_manager.clone(),
            block_notify.clone(),
        ));

        info!("Starting streaming service background tasks");

        Ok(Self {
            drive_client,
            tenderdash_client,
            config,
            zmq_listener,
            subscriber_manager,
            block_notify,
            workers: Arc::new(workers),
        })
    }

    /// Background worker: subscribe to ZMQ and process events, with retry/backoff
    async fn zmq_subscribe_and_process_worker(
        zmq_listener: Arc<dyn ZmqListenerTrait>,
        subscriber_manager: Arc<SubscriberManager>,
        block_notify: broadcast::Sender<()>,
    ) {
        trace!("Starting ZMQ subscribe/process loop");
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);
        loop {
            match zmq_listener.subscribe().await {
                Ok(zmq_events) => {
                    trace!("ZMQ listener started successfully, processing events");
                    Self::process_zmq_events(
                        zmq_events,
                        subscriber_manager.clone(),
                        block_notify.clone(),
                    )
                    .await;
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
        block_notify: broadcast::Sender<()>,
    ) {
        trace!("Starting ZMQ event processing loop");
        while let Ok(event) = zmq_events.recv().await {
            match event {
                ZmqEvent::RawTransaction { data } => {
                    trace!("Processing raw transaction event");
                    subscriber_manager
                        .notify_transaction_subscribers(&data)
                        .await;
                }
                ZmqEvent::RawBlock { data } => {
                    trace!("Processing raw block event");
                    subscriber_manager.notify_block_subscribers(&data).await;
                    let _ = block_notify.send(());
                }
                ZmqEvent::RawTransactionLock { data } => {
                    trace!("Processing transaction lock event");
                    subscriber_manager
                        .notify_instant_lock_subscribers(&data)
                        .await;
                }
                ZmqEvent::RawChainLock { data } => {
                    trace!("Processing chain lock event");
                    subscriber_manager
                        .notify_chain_lock_subscribers(&data)
                        .await;
                }
                ZmqEvent::HashBlock { hash } => {
                    trace!("Processing new block hash event");
                    subscriber_manager.notify_new_block_subscribers(&hash).await;
                    let _ = block_notify.send(());
                }
            }
        }
        trace!("ZMQ event processing loop ended");
    }

    pub fn subscribe_blocks(&self) -> broadcast::Receiver<()> {
        self.block_notify.subscribe()
    }

    /// Returns current health of the ZMQ streaming pipeline
    pub fn is_healthy(&self) -> bool {
        self.zmq_listener.is_connected()
    }
}
