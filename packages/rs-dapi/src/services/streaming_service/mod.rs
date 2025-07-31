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

use crate::clients::traits::{DriveClientTrait, TenderdashClientTrait};
use crate::config::Config;

pub(crate) use subscriber_manager::{
    FilterType, StreamingMessage, SubscriberManager, SubscriptionType,
};
pub(crate) use transaction_filter::TransactionFilter;
pub(crate) use zmq_listener::{ZmqEvent, ZmqListener};

/// Cache expiration time for streaming responses
const CACHE_EXPIRATION_DURATION: std::time::Duration = std::time::Duration::from_secs(1);

/// Streaming service implementation with ZMQ integration
#[derive(Clone)]
pub struct StreamingServiceImpl {
    pub drive_client: Arc<dyn DriveClientTrait>,
    pub tenderdash_client: Arc<dyn TenderdashClientTrait>,
    pub config: Arc<Config>,
    pub zmq_listener: Arc<ZmqListener>,
    pub subscriber_manager: Arc<SubscriberManager>,
    pub cache: Arc<RwLock<HashMap<String, (Vec<u8>, Instant)>>>,
}

impl StreamingServiceImpl {
    pub fn new(
        drive_client: Arc<dyn DriveClientTrait>,
        tenderdash_client: Arc<dyn TenderdashClientTrait>,
        config: Arc<Config>,
    ) -> Self {
        let zmq_listener = Arc::new(ZmqListener::new(&config.dapi.core.zmq_url));
        let subscriber_manager = Arc::new(SubscriberManager::new());

        Self {
            drive_client,
            tenderdash_client,
            config,
            zmq_listener,
            subscriber_manager,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the streaming service background tasks
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Start ZMQ listener
        let zmq_events = self.zmq_listener.start().await?;

        // Start event processing task
        let subscriber_manager = self.subscriber_manager.clone();
        tokio::spawn(async move {
            Self::process_zmq_events(zmq_events, subscriber_manager).await;
        });

        Ok(())
    }

    /// Process ZMQ events and forward to matching subscribers
    async fn process_zmq_events(
        mut zmq_events: broadcast::Receiver<ZmqEvent>,
        subscriber_manager: Arc<SubscriberManager>,
    ) {
        while let Ok(event) = zmq_events.recv().await {
            match event {
                ZmqEvent::RawTransaction { data } => {
                    subscriber_manager
                        .notify_transaction_subscribers(&data)
                        .await;
                }
                ZmqEvent::RawBlock { data } => {
                    subscriber_manager.notify_block_subscribers(&data).await;
                }
                ZmqEvent::RawTransactionLock { data } => {
                    subscriber_manager
                        .notify_instant_lock_subscribers(&data)
                        .await;
                }
                ZmqEvent::RawChainLock { data } => {
                    subscriber_manager
                        .notify_chain_lock_subscribers(&data)
                        .await;
                }
                ZmqEvent::HashBlock { hash } => {
                    subscriber_manager.notify_new_block_subscribers(&hash).await;
                }
            }
        }
    }

    /// Get a cached response if it exists and is still fresh
    pub async fn get_cached_response(&self, cache_key: &str) -> Option<Vec<u8>> {
        if let Some((cached_response, cached_time)) =
            self.cache.read().await.get(cache_key).cloned()
        {
            if cached_time.elapsed() < CACHE_EXPIRATION_DURATION {
                return Some(cached_response);
            }
        }
        None
    }

    /// Set a response in the cache with current timestamp
    pub async fn set_cached_response(&self, cache_key: String, response: Vec<u8>) {
        let cache_entry = (response, Instant::now());
        self.cache.write().await.insert(cache_key, cache_entry);
    }

    /// Clear expired entries from the cache
    pub async fn clear_expired_cache_entries(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, (_, cached_time)| cached_time.elapsed() < CACHE_EXPIRATION_DURATION);
    }
}
