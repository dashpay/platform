use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::error::DAPIResult;
use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use zeromq::prelude::*;

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
            socket: Arc::new(tokio::sync::Mutex::new(zeromq::SubSocket::new())),
        }
    }
}

#[async_trait]
impl ZmqListenerTrait for ZmqListener {
    /// Start the ZMQ listener and return a receiver for events
    async fn start(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>> {
        let receiver = self.event_sender.subscribe();

        // Start the ZMQ listener in a background task
        let zmq_uri = self.zmq_uri.clone();
        let topics = self.topics.clone();
        let sender = self.event_sender.clone();

        let socket = self.socket.clone();
        let connected = self.connected.clone();

        tokio::task::spawn(async move {
            if let Err(e) =
                Self::zmq_listener_task(zmq_uri, topics, sender, socket, connected).await
            {
                error!("ZMQ listener task error: {}", e);
            }
        });

        // Give the ZMQ connection a moment to establish
        sleep(Duration::from_millis(100)).await;

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
        topics: ZmqTopics,
        sender: broadcast::Sender<ZmqEvent>,
        socket_store: Arc<tokio::sync::Mutex<zeromq::SubSocket>>,
        connected: Arc<AtomicBool>,
    ) -> DAPIResult<()> {
        info!("Starting ZMQ listener on {}", zmq_uri);

        let socket_arc = socket_store.clone();
        let mut socket = socket_arc.lock().await;

        // Subscribe to all topics
        socket.subscribe(&topics.rawtx).await?;
        socket.subscribe(&topics.rawblock).await?;
        socket.subscribe(&topics.rawtxlocksig).await?;
        socket.subscribe(&topics.rawchainlocksig).await?;
        socket.subscribe(&topics.hashblock).await?;

        // Connect to Dash Core ZMQ
        socket.connect(&zmq_uri).await?;
        info!("Connected to ZMQ at {}", zmq_uri);
        drop(socket); // release the lock before starting the monitor

        // Start the ZMQ monitor task
        let monitor_socket = socket_store.clone();
        let connected_clone = connected.clone();
        tokio::spawn(async move {
            Self::zmq_monitor_task(monitor_socket, connected_clone).await;
            tracing::info!("ZMQ monitor task terminated");
        });

        let mut backoff = Duration::from_millis(100);
        loop {
            match Self::receive_zmq_message(socket_store.clone(), &topics).await {
                Ok(Some(event)) => {
                    debug!("Received ZMQ event: {:?}", event);
                    if let Err(e) = sender.send(event) {
                        warn!("Failed to send ZMQ event to subscribers: {}", e);
                    }

                    backoff = Duration::from_millis(100); // Reset backoff on successful receive
                }
                Ok(None) => {
                    // No message or unknown topic, continue
                    backoff = Duration::from_millis(100); // Reset backoff on successful receive
                }
                Err(e) => {
                    error!("Error receiving ZMQ message: {}", e);
                    // sleep with backoff to avoid busy loop
                    sleep(backoff).await;

                    if backoff < Duration::from_secs(5) {
                        backoff *= 2; // Exponential backoff
                    } else {
                        backoff = Duration::from_secs(5); // Cap backoff at 5 seconds
                    }
                }
            }
        }
    }

    /// Receive and parse a ZMQ message
    async fn receive_zmq_message(
        socket: Arc<Mutex<zeromq::SubSocket>>,
        topics: &ZmqTopics,
    ) -> DAPIResult<Option<ZmqEvent>> {
        // Receive message
        let mut socket_guard = socket.lock().await;
        let message = socket_guard.recv().await?;
        drop(socket_guard); // Release the lock before processing

        // Convert ZmqMessage to multipart frames
        let frames = message.into_vec();

        // ZeroMQ messages are multipart: [topic, data]
        if frames.len() < 2 {
            return Ok(None);
        }

        let topic = String::from_utf8_lossy(&frames[0]);
        let data = frames[1].to_vec(); // Convert to Vec<u8>

        let event = match topic.as_ref() {
            topic if topic == topics.rawtx => Some(ZmqEvent::RawTransaction { data }),
            topic if topic == topics.rawblock => Some(ZmqEvent::RawBlock { data }),
            topic if topic == topics.rawtxlocksig => Some(ZmqEvent::RawTransactionLock { data }),
            topic if topic == topics.rawchainlocksig => Some(ZmqEvent::RawChainLock { data }),
            topic if topic == topics.hashblock => Some(ZmqEvent::HashBlock { hash: data }),
            _ => {
                debug!("Unknown ZMQ topic: {}", topic);
                None
            }
        };

        Ok(event)
    }

    /// ZMQ monitor task that runs in the background and updates the connection status
    async fn zmq_monitor_task(
        socket_store: Arc<tokio::sync::Mutex<zeromq::SubSocket>>,
        connected: Arc<AtomicBool>,
    ) {
        info!("Starting ZMQ monitor task");
        let mut socket = socket_store.lock().await;
        let mut monitor = socket.monitor();
        drop(socket);

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
                }
                _ => {
                    debug!("ZMQ socket event: {:?}", event);
                }
            }
        }

        info!("ZMQ monitor channel closed");
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
