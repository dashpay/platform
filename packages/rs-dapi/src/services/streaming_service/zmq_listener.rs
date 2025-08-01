use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::error::{DAPIResult, DapiError};
use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use zeromq::prelude::*;
use zeromq::SubSocket;

/// ZMQ topics that we subscribe to from Dash Core

#[derive(Debug, Clone)]
pub struct ZmqTopics {
    pub hashtx: String,
    pub hashtxlock: String,
    pub hashblock: String,
    pub rawblock: String,
    pub rawtx: String,
    pub rawtxlock: String,
    pub rawtxlocksig: String,
    pub rawchainlock: String,
    pub rawchainlocksig: String,
}

impl Default for ZmqTopics {
    fn default() -> Self {
        Self {
            hashtx: "hashtx".to_string(),
            hashtxlock: "hashtxlock".to_string(),
            hashblock: "hashblock".to_string(),
            rawblock: "rawblock".to_string(),
            rawtx: "rawtx".to_string(),
            rawtxlock: "rawtxlock".to_string(),
            rawtxlocksig: "rawtxlocksig".to_string(),
            rawchainlock: "rawchainlock".to_string(),
            rawchainlocksig: "rawchainlocksig".to_string(),
        }
    }
}

impl ZmqTopics {
    /// Convert to a vector of topic strings
    pub fn to_vec(&self) -> Vec<String> {
        vec![
            self.rawtx.clone(),
            self.rawblock.clone(),
            self.rawtxlocksig.clone(),
            self.rawchainlocksig.clone(),
            self.hashblock.clone(),
        ]
    }
}

/// Events emitted by the ZMQ listener
#[derive(Debug, Clone)]
pub enum ZmqEvent {
    /// Raw transaction data from Dash Core
    RawTransaction { data: Vec<u8> },
    /// Raw block data from Dash Core
    RawBlock { data: Vec<u8> },
    /// Raw transaction lock (InstantSend) data
    RawTransactionLock { data: Vec<u8> },
    /// Raw chain lock data
    RawChainLock { data: Vec<u8> },
    /// New block hash notification
    HashBlock { hash: Vec<u8> },
}

/// Trait for ZMQ listeners that can start streaming events asynchronously
#[async_trait]
pub trait ZmqListenerTrait: Send + Sync {
    /// Start the ZMQ listener and return a receiver for events
    async fn start(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>>;

    /// Check if the ZMQ listener is connected
    fn is_connected(&self) -> bool;
}

/// ZMQ listener that connects to Dash Core and streams events
pub struct ZmqListener {
    zmq_uri: String,
    topics: ZmqTopics,
    event_sender: broadcast::Sender<ZmqEvent>,
    _event_receiver: broadcast::Receiver<ZmqEvent>,
    socket: Arc<tokio::sync::Mutex<zeromq::SubSocket>>,
    connected: Arc<AtomicBool>,
    max_retry_count: usize,
    connection_timeout: Duration,
}

impl ZmqListener {
    pub fn new(zmq_uri: &str) -> Self {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        Self {
            zmq_uri: zmq_uri.to_string(),
            topics: ZmqTopics::default(),
            event_sender,
            _event_receiver: event_receiver,
            connected: Arc::new(AtomicBool::new(false)),
            socket: Arc::new(tokio::sync::Mutex::new(SubSocket::new())),
            max_retry_count: 20,
            connection_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_retry_config(zmq_uri: &str, max_retries: usize, timeout: Duration) -> Self {
        Self {
            max_retry_count: max_retries,
            connection_timeout: timeout,
            ..Self::new(zmq_uri)
        }
    }
}

#[async_trait]
impl ZmqListenerTrait for ZmqListener {
    /// Start the ZMQ listener and return a receiver for events
    async fn start(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>> {
        let receiver = self.event_sender.subscribe();

        // Start the ZMQ monitor task to track connection status
        let monitor_socket = self.socket.clone();
        let monitor_connected = self.connected.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::zmq_monitor_task(monitor_socket, monitor_connected).await {
                error!("ZMQ monitor task error: {}", e);
            }
        });

        // Start the ZMQ listener in a background task
        let zmq_uri = self.zmq_uri.clone();
        let topics = self.topics.to_vec();
        let sender = self.event_sender.clone();
        let socket = self.socket.clone();
        let max_retry_count = self.max_retry_count;
        let connection_timeout = self.connection_timeout;

        tokio::task::spawn(async move {
            if let Err(e) = Self::zmq_listener_task(
                zmq_uri,
                topics,
                sender,
                socket,
                max_retry_count,
                connection_timeout,
            )
            .await
            {
                error!("ZMQ listener task error: {}", e);
            }
        });

        // Wait for initial connection attempt with timeout
        let start_time = tokio::time::Instant::now();
        while !self.is_connected() && start_time.elapsed() < self.connection_timeout {
            sleep(Duration::from_millis(100)).await;
        }

        if !self.is_connected() {
            warn!("ZMQ connection not established within timeout, but continuing with background retries");
        }

        Ok(receiver)
    }

    /// Check if the ZMQ listener is connected (placeholder)
    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ZmqListener {
    /// ZMQ listener task that runs asynchronously
    async fn zmq_listener_task(
        zmq_uri: String,
        topics: Vec<String>,
        sender: broadcast::Sender<ZmqEvent>,
        socket_store: Arc<tokio::sync::Mutex<zeromq::SubSocket>>,
        max_retry_count: usize,
        connection_timeout: Duration,
    ) -> DAPIResult<()> {
        let mut retry_count = 0;
        let mut delay = Duration::from_millis(1000); // Start with 1 second delay

        loop {
            // Try to establish connection
            match Self::connect_zmq(&zmq_uri, &topics, &socket_store, connection_timeout).await {
                Ok(_) => {
                    retry_count = 0; // Reset retry count on successful connection
                    delay = Duration::from_millis(1000); // Reset delay
                    info!("ZMQ connected to {}", zmq_uri);

                    // Listen for messages
                    if let Err(e) = Self::listen_for_messages(&socket_store, &sender).await {
                        error!("Error listening for ZMQ messages: {}", e);
                    }
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= max_retry_count {
                        error!(
                            "Failed to connect to ZMQ after {} attempts: {}",
                            max_retry_count, e
                        );
                        return Err(e);
                    }

                    warn!(
                        "ZMQ connection attempt {} failed: {}. Retrying in {:?}",
                        retry_count, e, delay
                    );
                    sleep(delay).await;

                    // Exponential backoff with jitter, capped at 30 seconds
                    delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                }
            }
        }
    }

    /// Helper method to establish ZMQ connection
    async fn connect_zmq(
        zmq_uri: &str,
        topics: &[String],
        socket_store: &Arc<Mutex<zeromq::SubSocket>>,
        connection_timeout: Duration,
    ) -> DAPIResult<()> {
        let mut socket_guard = socket_store.lock().await;

        // Set connection timeout
        tokio::time::timeout(connection_timeout, async {
            socket_guard.connect(zmq_uri).await
        })
        .await
        .map_err(|_| DapiError::Configuration("Connection timeout".to_string()))?
        .map_err(|e| DapiError::ZmqConnection(e))?;

        // Subscribe to topics
        for topic in topics {
            socket_guard
                .subscribe(topic)
                .await
                .map_err(|e| DapiError::ZmqConnection(e))?;
        }

        Ok(())
    }

    /// Helper method to listen for ZMQ messages
    async fn listen_for_messages(
        socket_store: &Arc<Mutex<zeromq::SubSocket>>,
        sender: &broadcast::Sender<ZmqEvent>,
    ) -> DAPIResult<()> {
        loop {
            let message = {
                let mut socket_guard = socket_store.lock().await;
                socket_guard.recv().await
            };

            match message {
                Ok(msg) => {
                    let frames: Vec<Vec<u8>> = msg
                        .into_vec()
                        .into_iter()
                        .map(|bytes| bytes.to_vec())
                        .collect();
                    if let Some(event) = Self::parse_zmq_message(frames) {
                        if let Err(e) = sender.send(event) {
                            warn!("Failed to send ZMQ event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving ZMQ message: {}", e);

                    return Err(DapiError::ZmqConnection(e));
                }
            }
        }
    }

    /// Parse ZMQ message frames into events
    fn parse_zmq_message(frames: Vec<Vec<u8>>) -> Option<ZmqEvent> {
        if frames.len() < 2 {
            return None;
        }

        let topic = String::from_utf8_lossy(&frames[0]);
        let data = frames[1].clone();

        match topic.as_ref() {
            "rawtx" => Some(ZmqEvent::RawTransaction { data }),
            "rawblock" => Some(ZmqEvent::RawBlock { data }),
            "rawtxlocksig" => Some(ZmqEvent::RawTransactionLock { data }),
            "rawchainlocksig" => Some(ZmqEvent::RawChainLock { data }),
            "hashblock" => Some(ZmqEvent::HashBlock { hash: data }),
            _ => {
                warn!("Unknown ZMQ topic: {}", topic);
                None
            }
        }
    }

    /// ZMQ monitor task that tracks connection status changes
    async fn zmq_monitor_task(
        socket_store: Arc<Mutex<zeromq::SubSocket>>,
        connected: Arc<AtomicBool>,
    ) -> DAPIResult<()> {
        info!("Starting ZMQ monitor task");

        // Get a monitor from the socket
        let mut monitor = {
            let mut socket_guard = socket_store.lock().await;
            socket_guard.monitor()
        };

        // Monitor socket events
        use tokio_stream::StreamExt;
        while let Some(event) = monitor.next().await {
            match event {
                zeromq::SocketEvent::Connected(endpoint, peer) => {
                    info!(?endpoint, ?peer, "ZMQ socket connected");
                    connected.store(true, Ordering::SeqCst);
                }
                zeromq::SocketEvent::Disconnected(peer) => {
                    warn!(?peer, "ZMQ socket disconnected");
                    connected.store(false, Ordering::SeqCst);
                }
                zeromq::SocketEvent::Closed => {
                    error!("ZMQ socket closed");
                    connected.store(false, Ordering::SeqCst);
                    break; // Exit monitor loop when socket is closed
                }
                _ => {
                    // Log other events for debugging
                    tracing::trace!("Unsupported ZMQ socket event: {:?}", event);
                }
            }
        }

        info!("ZMQ monitor task terminated");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zmq_topics_default() {
        let topics = ZmqTopics::default();
        assert_eq!(topics.rawtx, "rawtx");
        assert_eq!(topics.rawblock, "rawblock");
    }

    #[test]
    fn test_zmq_listener_creation() {
        let listener = ZmqListener::new("tcp://127.0.0.1:28332");
        assert_eq!(listener.zmq_uri, "tcp://127.0.0.1:28332");
    }
}
