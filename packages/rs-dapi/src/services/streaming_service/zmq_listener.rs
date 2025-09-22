use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::error::{DAPIResult, DapiError};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::select;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::span;
use tracing::{error, info, warn};
use zeromq::SocketEvent;
use zeromq::SubSocket;
use zeromq::ZmqError;
use zeromq::ZmqMessage;
use zeromq::ZmqResult;
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

#[derive(Clone)]
pub struct ZmqConnection {
    cancel: CancellationToken,
    // Receiver for ZMQ messages; see `next()` method for usage
    rx: Arc<Mutex<mpsc::Receiver<ZmqMessage>>>,
    connected: Arc<AtomicBool>,
}

impl Drop for ZmqConnection {
    fn drop(&mut self) {
        // Cancel the connection when dropped
        self.cancel.cancel();
    }
}

impl ZmqConnection {
    /// Create new ZmqConnection with runnning dispatcher and monitor.
    ///
    /// Messages will be received using [`ZmqConnection::recv`].
    async fn new(
        zmq_uri: &str,
        topics: &[String],
        connection_timeout: Duration,
        parent_cancel: CancellationToken,
    ) -> DAPIResult<ZmqConnection> {
        // we want to be able to only clean up ZmqConnection threads, without affecting the caller
        let cancel = parent_cancel.child_token();
        // ensure the socket is not in use
        let mut socket = SubSocket::new();

        // updated in monitor
        let connected = Arc::new(AtomicBool::new(false));

        // Start monitor
        Self::start_monitor(socket.monitor(), connected.clone(), cancel.clone());

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

        let (tx, rx) = mpsc::channel(1000);

        ZmqDispatcher {
            socket,
            zmq_tx: tx,
            cancel: cancel.clone(),
            connected: connected.clone(),
        }
        .spawn();

        Ok(Self {
            cancel,
            rx: Arc::new(Mutex::new(rx)),
            connected,
        })
    }

    fn disconnected(&self) {
        self.connected.store(false, Ordering::SeqCst);
        self.cancel.cancel();
    }

    /// Start monitor that will get connection updates.
    fn start_monitor(
        mut monitor: futures::channel::mpsc::Receiver<SocketEvent>,
        connected: Arc<AtomicBool>,
        cancel: CancellationToken,
    ) {
        // Start the monitor to listen for connection events
        tokio::spawn(with_cancel(cancel.clone(), async move {
            while let Some(event) = monitor.next().await {
                if let Err(e) = Self::monitor_event(event, connected.clone(), cancel.clone()).await
                {
                    error!("ZMQ monitor event error: {}", e);
                }
            }
            error!("ZMQ monitor channel closed, stopping monitor");
            Err::<(), _>(DapiError::ConnectionClosed)
        }));
    }

    /// Act on monitor event
    async fn monitor_event(
        event: SocketEvent,
        connected: Arc<AtomicBool>,
        cancel: CancellationToken,
    ) -> DAPIResult<()> {
        // Get a monitor from the socket
        let span = span!(tracing::Level::TRACE, "zmq_monitor");
        let _span = span.enter();

        match event {
            zeromq::SocketEvent::Connected(endpoint, peer) => {
                info!(endpoint = %endpoint, peer = hex::encode(peer), "ZMQ socket connected");
                connected.store(true, Ordering::SeqCst);
            }
            zeromq::SocketEvent::Disconnected(peer) => {
                warn!(
                    peer = hex::encode(peer),
                    "ZMQ socket disconnected, requesting restart"
                );
                // this does NOT work, we never receive a Disconnected event
                // See [`ZmqDispatcher::tick_event_10s`] for workaround we use
                connected.store(false, Ordering::SeqCst);
                cancel.cancel();
            }
            zeromq::SocketEvent::Closed => {
                error!("ZMQ socket closed, requesting restart");
                connected.store(false, Ordering::SeqCst);
                cancel.cancel();
            }
            zeromq::SocketEvent::ConnectRetried => {
                warn!("ZMQ connection retry attempt");
            }
            _ => {
                // Log other events for debugging
                tracing::trace!("ZMQ socket event: {:?}", event);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SocketRecv for ZmqConnection {
    async fn recv(&mut self) -> ZmqResult<ZmqMessage> {
        let mut rx = self.rx.lock().await;
        let received = rx.recv().await;
        drop(rx); // unlock

        match received {
            Some(msg) => return Ok(msg),
            None => {
                // If the channel is closed, we should handle it gracefully
                self.disconnected();
                return Err(ZmqError::NoMessage);
            }
        }
    }
}

/// ZMQ listener that connects to Dash Core and streams events
pub struct ZmqListener {
    zmq_uri: String,
    topics: ZmqTopics,
    event_sender: broadcast::Sender<ZmqEvent>,
    cancel: CancellationToken,
}

impl ZmqListener {
    pub fn new(zmq_uri: &str) -> DAPIResult<Self> {
        let (event_sender, _event_receiver) = broadcast::channel(1000);

        let mut instance = Self {
            zmq_uri: zmq_uri.to_string(),
            topics: ZmqTopics::default(),
            event_sender,
            cancel: CancellationToken::new(),
        };
        instance.connect()?;
        Ok(instance)
    }

    fn connect(&mut self) -> DAPIResult<()> {
        // Start the ZMQ listener in a background task
        let zmq_uri = self.zmq_uri.clone();
        let topics = self.topics.to_vec();
        let sender = self.event_sender.clone();

        let cancel = self.cancel.clone();

        tokio::task::spawn(with_cancel(cancel.clone(), async move {
            // we use child token so that cancelling threads started inside zmq_listener_task
            // does not cancel the zmq_listener_task itself, as it needs to restart the
            // connection if it fails
            if let Err(e) =
                Self::zmq_listener_task(zmq_uri, topics, sender, cancel.child_token()).await
            {
                error!("ZMQ listener task error: {}", e);
                // we cancel parent task to stop all spawned threads
                cancel.cancel();
            }
            Err::<(), _>(DapiError::ConnectionClosed)
        }));

        Ok(())
    }
    /// ZMQ listener task that runs asynchronously
    async fn zmq_listener_task(
        zmq_uri: String,
        topics: Vec<String>,
        sender: broadcast::Sender<ZmqEvent>,
        cancel_parent: CancellationToken,
    ) -> DAPIResult<()> {
        let mut retry_count = 0;
        let mut delay = Duration::from_millis(1000); // Start with 1 second delay

        loop {
            // We don't want to cancel parent task by mistake
            let cancel = cancel_parent.child_token();

            // Try to establish connection
            match ZmqConnection::new(&zmq_uri, &topics, Duration::from_secs(5), cancel).await {
                Ok(mut connection) => {
                    retry_count = 0; // Reset retry count on successful connection
                    delay = Duration::from_millis(1000); // Reset delay
                    info!("ZMQ connected to {}", zmq_uri);

                    // Listen for messages with connection recovery

                    match Self::process_messages(&mut connection, sender.clone()).await {
                        Ok(_) => {
                            info!("ZMQ message processing ended normally");
                        }
                        Err(e) => {
                            error!("ZMQ message processing failed: {}", e);
                            continue; // Restart connection
                        }
                    }
                }
                Err(e) => {
                    error!("ZMQ connection failed: {}", e);
                    retry_count += 1;

                    warn!(
                        "ZMQ connection attempt {} failed: {}. Retrying in {:?}",
                        retry_count, e, delay
                    );
                    sleep(delay).await;

                    // Exponential backoff with jitter, capped at 300 seconds
                    delay = std::cmp::min(delay * 2, Duration::from_secs(300));
                }
            }
        }
    }

    /// After successful connection, start the message processing workers that will process messages
    ///
    /// Errors returned by this method are critical and should cause the listener to restart
    async fn process_messages(
        connection: &mut ZmqConnection,
        sender: broadcast::Sender<ZmqEvent>,
    ) -> DAPIResult<()> {
        tracing::trace!("ZMQ worker waiting for messages");

        loop {
            let message = connection.recv().await;

            match message {
                Ok(msg) => {
                    let frames: Vec<Vec<u8>> = msg
                        .into_vec()
                        .into_iter()
                        .map(|bytes| bytes.to_vec())
                        .collect();
                    if let Some(event) = Self::parse_zmq_message(frames) {
                        let summary = super::StreamingServiceImpl::summarize_zmq_event(&event);
                        tracing::trace!(event = %summary, "Received ZMQ event");
                        if let Err(e) = sender.send(event) {
                            tracing::trace!("Cannot send ZMQ event, dropping: {}", e);
                        }
                    }
                }
                Err(ZmqError::NoMessage) => {
                    // No message received
                    tracing::warn!("No ZMQ message received, connection closed? Exiting worker");
                    return Err(DapiError::ConnectionClosed);
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
        !self.cancel.is_cancelled()
    }
}

struct ZmqDispatcher {
    socket: SubSocket,
    zmq_tx: mpsc::Sender<ZmqMessage>,
    /// Cancellation token to stop all spawned threads; cancelled when the connection is lost
    cancel: CancellationToken,
    connected: Arc<AtomicBool>,
}

impl ZmqDispatcher {
    /// Create a new ZmqDispatcher
    fn spawn(self) {
        let cancel = self.cancel.clone();
        tokio::spawn(with_cancel(cancel, self.dispatcher_worker()));
    }

    /// Receive messages from the ZMQ socket and dispatch them to the provided sender.
    /// It also supports connection health monitoring.
    async fn dispatcher_worker(mut self) -> DAPIResult<()> {
        let mut interval_10s = tokio::time::interval(Duration::from_secs(10));
        interval_10s.reset();

        loop {
            select! {
                msg = self.socket.recv() => {
                    match msg {
                        Ok(msg) => if let Err(e) = self.zmq_tx.send(msg).await {
                            error!("Error sending ZMQ event to receiver: {}, receiver may have exited", e);
                            // receiver exited? I think it is fatal, we exit as it makes no sense to continue
                            self.connected.store(false, Ordering::SeqCst);
                            self.cancel.cancel();
                            return Err(DapiError::ClientGone("ZMQ receiver exited".to_string()));
                        },
                        Err(e) => {
                            warn!("Error receiving ZMQ message: {}, restarting connection", e);
                            // most likely the connection is lost, we exit as this will abort the task anyway
                            self.connected.store(false, Ordering::SeqCst);
                            self.cancel.cancel();

                            return Err(DapiError::ConnectionClosed);
                        }
                    }
                }
                _ = interval_10s.tick() => {
                    self.tick_event_10s().await;
                }
            };
        }
    }

    /// Event that happens every ten seconds to check connection status
    async fn tick_event_10s(&mut self) {
        // Health check of zmq connection
        // This is a hack to ensure the connection is alive, as the monitor fails to notify us about disconnects
        let current_status = self.socket.subscribe("ping").await.is_ok();

        // If the status changed, log it
        let previous_status = self.connected.swap(current_status, Ordering::SeqCst);
        if current_status != previous_status {
            if current_status {
                debug!("ZMQ connection recovered");
            } else {
                error!("ZMQ connection is lost, connection will be restarted");
                // disconnect the socket
                self.cancel.cancel();
            }
        }
    }
}

/// Helper function to run a future with cancellation support.
async fn with_cancel<T>(
    cancel: CancellationToken,
    future: impl Future<Output = DAPIResult<T>>,
) -> DAPIResult<T> {
    select! {
        _ = cancel.cancelled() => {
            warn!("Cancelled before future completed");
            Err(DapiError::ConnectionClosed)
        }
        result = future => result,
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

    #[tokio::test]
    async fn test_zmq_listener_creation() {
        let listener = ZmqListener::new("tcp://127.0.0.1:28332").unwrap();
        assert_eq!(listener.zmq_uri, "tcp://127.0.0.1:28332");
    }
}
