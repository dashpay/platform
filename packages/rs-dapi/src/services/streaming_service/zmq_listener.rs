use std::ops::DerefMut;
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

/// Number of threads to start that will receive and process ZMQ messages
const ZMQ_WORKER_THREADS: usize = 2;

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
    /// Subscribe to ZMQ events and return a receiver for them
    async fn subscribe(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>>;

    /// Check if the ZMQ listener is connected
    fn is_connected(&self) -> bool;
}

/// ZMQ listener that connects to Dash Core and streams events
pub struct ZmqListener {
    zmq_uri: String,
    topics: ZmqTopics,
    event_sender: broadcast::Sender<ZmqEvent>,
    _event_receiver: broadcast::Receiver<ZmqEvent>,
    socket: Arc<tokio::sync::Mutex<Option<zeromq::SubSocket>>>,
    connected: Arc<AtomicBool>,
    max_retry_count: usize,
    connection_timeout: Duration,
}

impl ZmqListener {
    pub fn new(zmq_uri: &str) -> DAPIResult<Self> {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        Self {
            zmq_uri: zmq_uri.to_string(),
            topics: ZmqTopics::default(),
            event_sender,
            _event_receiver: event_receiver,
            connected: Arc::new(AtomicBool::new(false)),
            socket: Arc::new(tokio::sync::Mutex::new(Some(SubSocket::new()))),
            max_retry_count: 20,
            connection_timeout: Duration::from_secs(30),
        }
        .start()
    }

    fn start(self) -> DAPIResult<Self> {
        self.start_monitor()?;
        self.start_zmq_listener()?;

        Ok(self)
    }
}

#[async_trait]
impl ZmqListenerTrait for ZmqListener {
    /// Subscribe to ZMQ events and return a receiver for them
    async fn subscribe(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>> {
        let receiver = self.event_sender.subscribe();

        Ok(receiver)
    }

    /// Check if the ZMQ listener is connected (placeholder)
    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ZmqListener {
    fn start_zmq_listener(&self) -> DAPIResult<()> {
        // Start the ZMQ listener in a background task
        let zmq_uri = self.zmq_uri.clone();
        let topics = self.topics.to_vec();
        let sender = self.event_sender.clone();
        let socket = self.socket.clone();
        let max_retry_count = self.max_retry_count;
        let connection_timeout = self.connection_timeout;
        let connected = self.connected.clone();

        tokio::task::spawn(async move {
            if let Err(e) = Self::zmq_listener_task(
                zmq_uri,
                topics,
                sender,
                socket,
                max_retry_count,
                connection_timeout,
                connected,
            )
            .await
            {
                error!("ZMQ listener task error: {}", e);
            }
        });

        Ok(())
    }
    /// ZMQ listener task that runs asynchronously
    async fn zmq_listener_task(
        zmq_uri: String,
        topics: Vec<String>,
        sender: broadcast::Sender<ZmqEvent>,
        socket_store: Arc<tokio::sync::Mutex<Option<zeromq::SubSocket>>>,
        max_retry_count: usize,
        connection_timeout: Duration,
        connected: Arc<AtomicBool>,
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
                    // Mark as connected, as the monitor might not be running yet
                    // we assume that future connected state will be maintained by the monitor task
                    connected.store(true, Ordering::SeqCst);

                    // Listen for messages
                    Self::process_messages(&socket_store, &sender).await?;
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
        socket_store: &Arc<tokio::sync::Mutex<Option<zeromq::SubSocket>>>,
        connection_timeout: Duration,
    ) -> DAPIResult<()> {
        // ensure the socket is not in use
        let mut socket_guard = socket_store.lock().await;
        let socket = socket_guard.get_or_insert_with(zeromq::SubSocket::new);

        // Set connection timeout
        tokio::time::timeout(connection_timeout, async { socket.connect(zmq_uri).await })
            .await
            .map_err(|_| DapiError::Configuration("Connection timeout".to_string()))?
            .map_err(DapiError::ZmqConnection)?;

        // Subscribe to topics
        for topic in topics {
            socket
                .subscribe(topic)
                .await
                .map_err(DapiError::ZmqConnection)?;
        }

        Ok(())
    }

    /// After successful connection, start the message processing workers that will process messages
    ///
    /// Errors returned by this method are critical and should cause the listener to restart
    async fn process_messages(
        socket_store: &Arc<Mutex<Option<zeromq::SubSocket>>>,
        sender: &broadcast::Sender<ZmqEvent>,
    ) -> DAPIResult<()> {
        // Start message workers
        let mut worker_threads = tokio::task::join_set::JoinSet::new();
        for i in 1..=ZMQ_WORKER_THREADS {
            info!("Starting ZMQ worker thread {}", i);
            // Spawn a task for each worker thread
            let worker_socket = socket_store.clone();
            let worker_sender = sender.clone();
            worker_threads.spawn(Self::message_worker(i, worker_socket, worker_sender));
        }

        // Wait for all worker threads to finish
        while let Some(result) = worker_threads.join_next().await {
            match result {
                Ok(Ok(worker_id)) => {
                    info!(worker_id, "ZMQ worker thread completed successfully");
                }
                Ok(Err((worker_id, e))) => {
                    error!(worker_id, "ZMQ worker thread failed: {}", e);
                }
                Err(e) => {
                    error!("ZMQ worker thread runtime error: {}", e);
                }
            }
        }

        // We will get here when all worker threads have finished; it means something really bad happened and we should
        // restart the listener
        error!("All ZMQ worker threads have finished unexpectedly, restarting listener");
        Err(DapiError::Internal(
            "All worker threads finished unexpectedly".to_string(),
        ))
    }

    /// Helper method to listen for ZMQ messages and forward them as events
    async fn message_worker(
        worker_id: usize,
        socket_store: Arc<Mutex<Option<zeromq::SubSocket>>>,
        sender: broadcast::Sender<ZmqEvent>,
    ) -> Result<usize, (usize, DapiError)> {
        let span = tracing::span!(tracing::Level::TRACE, "zmq_worker", id = worker_id);
        let _span = span.enter();

        loop {
            tracing::trace!("ZMQ worker waiting for messages");
            let message = {
                let mut socket_guard = socket_store.lock().await;
                if let Some(socket) = socket_guard.as_mut() {
                    socket.recv().await
                } else {
                    tracing::trace!("ZMQ socket not initialized, retry in 1s");
                    sleep(Duration::from_secs(1)).await;
                    continue; // Retry if socket is not ready
                }
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

                    return Err((worker_id, DapiError::ZmqConnection(e)));
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
    /// Start the ZMQ monitor task to track connection status
    fn start_monitor(&self) -> DAPIResult<()> {
        // Start the ZMQ monitor task to track connection status
        let monitor_socket = self.socket.clone();
        let connected = self.connected.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::zmq_monitor_task(monitor_socket, connected).await {
                error!("ZMQ monitor task error: {}", e);
            }
        }); // Start the monitor task in the background, so no await is needed

        Ok::<(), DapiError>(())
    }
    /// ZMQ monitor task that tracks connection status changes
    async fn zmq_monitor_task(
        socket_store: Arc<Mutex<Option<zeromq::SubSocket>>>,
        connected: Arc<AtomicBool>,
    ) -> DAPIResult<()> {
        // Get a monitor from the socket
        info!("Starting ZMQ monitor task");
        let mut monitor = loop {
            if let Some(socket) = socket_store.lock().await.as_mut() {
                break socket.monitor();
            }
            tracing::trace!("ZMQ socket not initialized, retrying in 1s");
            sleep(Duration::from_secs(1)).await;
        };

        tracing::trace!("ZMQ monitor started");

        // Monitor socket events
        use tokio_stream::StreamExt;
        while let Some(event) = monitor.next().await {
            match event {
                zeromq::SocketEvent::Connected(endpoint, peer) => {
                    info!(endpoint = %endpoint, peer = hex::encode(peer), "ZMQ socket connected");
                    connected.store(true, Ordering::SeqCst);
                }
                zeromq::SocketEvent::Disconnected(peer) => {
                    warn!(peer = hex::encode(peer), "ZMQ socket disconnected");
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
        let listener = ZmqListener::new("tcp://127.0.0.1:28332").unwrap();
        assert_eq!(listener.zmq_uri, "tcp://127.0.0.1:28332");
    }
}
