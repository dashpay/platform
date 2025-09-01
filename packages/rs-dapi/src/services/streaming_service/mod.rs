// Streaming service modular implementation
// This module handles real-time streaming of blockchain data from ZMQ to gRPC clients

mod block_header_stream;
mod masternode_list_stream;
mod subscriber_manager;
mod transaction_filter;
mod transaction_stream;
mod zmq_listener;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Instant;
use tracing::{error, info, trace};

use crate::clients::traits::TenderdashClientTrait;
use crate::config::Config;

pub(crate) use subscriber_manager::{
    FilterType, StreamingMessage, SubscriberManager, SubscriptionType,
};
pub(crate) use zmq_listener::{ZmqEvent, ZmqListener, ZmqListenerTrait};

/// Cache expiration time for streaming responses
const CACHE_EXPIRATION_DURATION: std::time::Duration = std::time::Duration::from_secs(1);

/// Type alias for cache data: (data, timestamp)
type CacheData = (Vec<u8>, Instant);
/// Type alias for the cache store
type CacheStore = Arc<RwLock<HashMap<String, CacheData>>>;

/// Streaming service implementation with ZMQ integration
#[derive(Clone)]
pub struct StreamingServiceImpl {
    pub drive_client: crate::clients::drive_client::DriveClient,
    pub tenderdash_client: Arc<dyn TenderdashClientTrait>,
    pub config: Arc<Config>,
    pub zmq_listener: Arc<dyn ZmqListenerTrait>,
    pub subscriber_manager: Arc<SubscriberManager>,
    pub cache: CacheStore,
    pub block_notify: broadcast::Sender<()>,
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
        let service = Self {
            drive_client,
            tenderdash_client,
            config,
            zmq_listener,
            subscriber_manager,
            cache: Arc::new(RwLock::new(HashMap::new())),
            block_notify,
        };

        info!("Starting streaming service background tasks");
        service.start_background_tasks();

        Ok(service)
    }

    /// Start the streaming service background tasks
    fn start_background_tasks(&self) {
        trace!("Starting ZMQ listener and event processing tasks");
        // Start ZMQ listener
        let zmq_listener = self.zmq_listener.clone();

        // Start event processing task
        let subscriber_manager = self.subscriber_manager.clone();
        let block_notify = self.block_notify.clone();
        tokio::spawn(async move {
            let zmq_events = match zmq_listener.subscribe().await {
                Ok(zmq) => zmq,
                Err(e) => {
                    error!("ZMQ listener error: {}", e);
                    panic!("Failed to start ZMQ listener: {}", e);
                }
            };

            trace!("ZMQ listener started successfully, processing events");
            Self::process_zmq_events(zmq_events, subscriber_manager, block_notify).await;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
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

    /// Get a cached response if it exists and is still fresh
    pub async fn get_cached_response(&self, cache_key: &str) -> Option<Vec<u8>> {
        if let Some((cached_response, cached_time)) =
            self.cache.read().await.get(cache_key).cloned()
        {
            if cached_time.elapsed() < CACHE_EXPIRATION_DURATION {
                trace!("Cache hit for key: {}", cache_key);
                return Some(cached_response);
            }
        }
        trace!("Cache miss for key: {}", cache_key);
        None
    }

    /// Set a response in the cache with current timestamp
    pub async fn set_cached_response(&self, cache_key: String, response: Vec<u8>) {
        trace!("Caching response for key: {}", cache_key);
        let cache_entry = (response, Instant::now());
        self.cache.write().await.insert(cache_key, cache_entry);
    }

    /// Clear expired entries from the cache
    pub async fn clear_expired_cache_entries(&self) {
        trace!("Clearing expired cache entries");
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        cache.retain(|_, (_, cached_time)| cached_time.elapsed() < CACHE_EXPIRATION_DURATION);
        let cleared_count = initial_size - cache.len();
        if cleared_count > 0 {
            trace!("Cleared {} expired cache entries", cleared_count);
        }
    }
}
