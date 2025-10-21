//! ZMQ listener for Dash Core events
//!
//! This module provides functionality to connect to Dash Core's ZMQ interface.
//!
//! See [`ZmqListener`] for the main entry point.
//!
//! ## Control flow
//!
//! - `ZmqListener::new` creates a new listener and starts the connection task with [`ZmqConnection::new`]
//! - `ZmqConnection::new` establishes a new ZMQ connection and spawns [dispatcher](ZmqDispatcher)
//!   and [monitor](ZmqConnection::start_monitor) tasks
//! - Whenever new message arrives, [`ZmqDispatcher`] forwards it through a channel to [`ZmqConnection::recv`]
//! - [`ZmqListener::process_messages`] reads messages from the connection with [`ZmqConnection::recv`]
//! - [`ZmqListener::parse_zmq_message`] parses raw ZMQ messages into structured [`ZmqEvent`]
//! - subscribers subscribe to events via [`ZmqListener::subscribe`] to receive [`ZmqEvent`]s
//!
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use crate::error::{DAPIResult, DapiError};
use crate::sync::Workers;
use async_trait::async_trait;
use dashcore_rpc::dashcore::Transaction as CoreTransaction;
use dashcore_rpc::dashcore::consensus::Decodable;
use futures::StreamExt;
use std::io::Cursor;
use tokio::select;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, sleep};
use tokio_util::sync::CancellationToken;
use tracing::span;
use tracing::{debug, trace};
use zeromq::SocketEvent;
use zeromq::SubSocket;
use zeromq::ZmqError;
use zeromq::ZmqMessage;
use zeromq::ZmqResult;
use zeromq::prelude::*;

/// Start time for calculating durations
static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

/// ZMQ topics that we subscribe to from Dash Core

#[derive(Debug, Clone)]
pub struct ZmqTopics {
    // pub hashtx: String, -- not used
    // pub hashtxlock: String, -- not used
    pub hashblock: String,
    pub rawblock: String,
    pub rawtx: String,
    // pub rawtxlock: String, -- not used, it doesn't contain required data, we use rawtxlocksig instead
    pub rawtxlocksig: String,
    pub rawchainlock: String,
    pub rawchainlocksig: String,
}

impl Default for ZmqTopics {
    fn default() -> Self {
        Self {
            // hashtx: "hashtx".to_string(),
            // hashtxlock: "hashtxlock".to_string(),
            hashblock: "hashblock".to_string(),
            rawblock: "rawblock".to_string(),
            rawtx: "rawtx".to_string(),
            // rawtxlock: "rawtxlock".to_string(),
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
            self.rawchainlock.clone(),
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
    RawTransactionLock {
        tx_bytes: Option<Vec<u8>>,
        lock_bytes: Vec<u8>,
    },
    /// Raw chain lock data
    RawChainLock { data: Vec<u8> },
    /// New block hash notification
    HashBlock { hash: Vec<u8> },
}

#[derive(Clone)]
struct ZmqConnection {
    cancel: CancellationToken,
    /// Messages from zmq server, forwarded by  [ZmqDispatcher]; consumed in [`ZmqConnection::recv`]
    rx: Arc<Mutex<mpsc::Receiver<ZmqMessage>>>,
    connected: Arc<AtomicBool>,
    workers: Workers,
    subscribed_topics: Vec<String>,
}

impl Drop for ZmqConnection {
    fn drop(&mut self) {
        // Cancel the connection when dropped
        self.cancel.cancel();
    }
}

impl ZmqConnection {
    /// Create new ZmqConnection with running dispatcher and monitor.
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

        let (tx, rx) = mpsc::channel(1000);

        let mut connection = Self {
            cancel: cancel.clone(),
            rx: Arc::new(Mutex::new(rx)),
            connected: connected.clone(),
            workers: Workers::default(),
            subscribed_topics: Vec::new(),
        };
        // Start monitor
        connection.start_monitor(socket.monitor());

        // Set connection timeout
        tokio::time::timeout(connection_timeout, async { socket.connect(zmq_uri).await })
            .await
            .map_err(|e| {
                DapiError::Timeout(format!(
                    "Upstream ZMQ connect timeout {:.2}s exceeded: {}",
                    connection_timeout.as_secs_f32(),
                    e
                ))
            })?
            .map_err(DapiError::ZmqConnection)?;

        connection.zmq_subscribe(&mut socket, topics).await?;

        connection.start_dispatcher(socket, tx);

        Ok(connection)
    }

    async fn zmq_subscribe(&mut self, socket: &mut SubSocket, topics: &[String]) -> DAPIResult<()> {
        // Subscribe to topics
        let mut first_error = None;

        for topic in topics {
            let result = socket
                .subscribe(topic)
                .await
                .map_err(DapiError::ZmqConnection);

            match result {
                Ok(_) => self.subscribed_topics.push(topic.clone()),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }

        if let Some(error) = first_error {
            debug!(
                ?error,
                "ZMQ subscription errors occured, trying to unsubscribe from successful topics",
            );

            self.zmq_unsubscribe_all(socket).await?;
            // return the first error
            return Err(error);
        };

        Ok(())
    }

    /// Unsubscribe from all topics. Returns first error encountered, if any.
    async fn zmq_unsubscribe_all(&mut self, socket: &mut SubSocket) -> DAPIResult<()> {
        let mut first_error = None;
        for topic in &self.subscribed_topics {
            if let Err(e) = socket.unsubscribe(topic).await {
                trace!(
                    topic = %topic,
                    error = %e,
                    "Error unsubscribing from ZMQ topic",
                );
                first_error.get_or_insert(DapiError::ZmqConnection(e));
            }
        }

        // Clear the list of subscribed topics; even if errors occurred, we consider ourselves unsubscribed
        self.subscribed_topics.clear();

        first_error.map(Err).unwrap_or(Ok(()))
    }

    fn disconnected(&self) {
        self.connected.store(false, Ordering::SeqCst);
        self.cancel.cancel();
    }

    fn start_dispatcher(&self, socket: SubSocket, tx: mpsc::Sender<ZmqMessage>) {
        let cancel = self.cancel.clone();

        ZmqDispatcher {
            socket,
            zmq_tx: tx,
            cancel: cancel.clone(),
            connected: self.connected.clone(),
            last_recv: Arc::new(AtomicU64::new(0)),
        }
        .spawn(&self.workers);
    }

    /// Start monitor that will get connection updates.
    fn start_monitor(&self, mut monitor: futures::channel::mpsc::Receiver<SocketEvent>) {
        let connected = self.connected.clone();
        let cancel = self.cancel.clone();
        // Start the monitor to listen for connection events
        self.workers.spawn(with_cancel(cancel.clone(), async move {
            while let Some(event) = monitor.next().await {
                if let Err(e) = Self::monitor_event(event, connected.clone(), cancel.clone()).await
                {
                    debug!(error = %e, "ZMQ monitor event error");
                }
            }
            debug!("ZMQ monitor channel closed, stopping monitor");
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
                trace!(endpoint = %endpoint, peer = hex::encode(peer), "ZMQ socket connected");
                connected.store(true, Ordering::SeqCst);
            }
            zeromq::SocketEvent::Disconnected(peer) => {
                debug!(
                    peer = hex::encode(peer),
                    "ZMQ socket disconnected, requesting restart"
                );
                // this does NOT work, we never receive a Disconnected event
                // See [`ZmqDispatcher::tick_event_10s`] for workaround we use
                connected.store(false, Ordering::SeqCst);
                cancel.cancel();
            }
            zeromq::SocketEvent::Closed => {
                debug!("ZMQ socket closed, requesting restart");
                connected.store(false, Ordering::SeqCst);
                cancel.cancel();
            }
            zeromq::SocketEvent::ConnectRetried => {
                debug!("ZMQ connection retry attempt");
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

/// ZMQ listener that connects to Dash Core and streams events.
///
/// This is the main entry point for ZMQ streaming.
pub struct ZmqListener {
    zmq_uri: String,
    topics: ZmqTopics,
    event_sender: broadcast::Sender<ZmqEvent>,
    cancel: CancellationToken,
    workers: Workers,
}

impl ZmqListener {
    pub fn new(zmq_uri: &str) -> DAPIResult<Self> {
        let (event_sender, _event_receiver) = broadcast::channel(1000);

        let mut instance = Self {
            zmq_uri: zmq_uri.to_string(),
            topics: ZmqTopics::default(),
            event_sender,
            cancel: CancellationToken::new(),
            workers: Workers::default(),
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

        self.workers.spawn(with_cancel(cancel.clone(), async move {
            // we use child token so that cancelling threads started inside zmq_listener_task
            // does not cancel the zmq_listener_task itself, as it needs to restart the
            // connection if it fails
            if let Err(e) =
                Self::zmq_listener_task(zmq_uri, topics, sender, cancel.child_token()).await
            {
                debug!(error = %e, "ZMQ listener task error");
                // we cancel parent task to stop all spawned threads
                cancel.cancel();
            }
            Err::<(), _>(DapiError::ConnectionClosed)
        }));

        Ok(())
    }

    /// Subscribe to ZMQ events and return a receiver for them
    pub fn subscribe(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>> {
        Ok(self.event_sender.subscribe())
    }

    /// Check if the ZMQ listener is connected (placeholder)
    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
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

            if cancel.is_cancelled() {
                debug!("ZMQ listener task cancelled, exiting");
                return Err(DapiError::ConnectionClosed);
            }

            // Try to establish connection
            match ZmqConnection::new(&zmq_uri, &topics, Duration::from_secs(5), cancel).await {
                Ok(mut connection) => {
                    retry_count = 0; // Reset retry count on successful connection
                    delay = Duration::from_millis(1000); // Reset delay
                    trace!("ZMQ connected to {}", zmq_uri);

                    // Listen for messages with connection recovery

                    match Self::process_messages(&mut connection, sender.clone()).await {
                        Ok(_) => {
                            trace!("ZMQ message processing ended normally");
                        }
                        Err(e) => {
                            debug!(error = %e, "ZMQ message processing failed");
                            continue; // Restart connection
                        }
                    }
                }
                Err(e) => {
                    debug!(error = %e, "ZMQ connection failed");
                    retry_count += 1;

                    debug!(
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
                        let summary = super::summarize_zmq_event(&event);
                        tracing::trace!(event = %summary, "Received ZMQ event");
                        if let Err(e) = sender.send(event) {
                            tracing::trace!("Cannot send ZMQ event, dropping: {}", e);
                        }
                    }
                }
                Err(ZmqError::NoMessage) => {
                    // No message received
                    tracing::debug!("No ZMQ message received, connection closed? Exiting worker");
                    return Err(DapiError::ConnectionClosed);
                }
                Err(e) => {
                    debug!(error = %e, "Error receiving ZMQ message");
                    return Err(DapiError::ZmqConnection(e));
                }
            }
        }
    }

    /// Parse ZMQ message frames into events
    fn parse_zmq_message(frames: Vec<Vec<u8>>) -> Option<ZmqEvent> {
        tracing::trace!(frames_count = frames.len(), "Parsing new ZMQ message");
        if frames.len() < 2 {
            return None;
        }

        let topic = String::from_utf8_lossy(&frames[0]);
        let data = frames[1].clone();

        match topic.as_ref() {
            "rawtx" => Some(ZmqEvent::RawTransaction { data }),
            "rawblock" => Some(ZmqEvent::RawBlock { data }),
            "rawtxlocksig" => {
                tracing::trace!(
                    data = hex::encode(&data),
                    "Parsing rawtxlocksig ZMQ message"
                );
                let (tx_bytes, lock_bytes_opt) = split_tx_and_lock(data);
                if let Some(lock_bytes) = lock_bytes_opt
                    && !lock_bytes.is_empty()
                {
                    Some(ZmqEvent::RawTransactionLock {
                        tx_bytes,
                        lock_bytes,
                    })
                } else {
                    debug!("rawtxlocksig payload missing instant lock bytes");
                    None
                }
            }
            // We ignore rawtxlock, we need rawtxlocksig only
            // "rawtxlock" => Some(ZmqEvent::RawTransactionLock { data }),
            "rawchainlocksig" => Some(ZmqEvent::RawChainLock { data }),
            // Some Core builds emit rawchainlock without signature suffix
            "rawchainlock" => Some(ZmqEvent::RawChainLock { data }),
            "hashblock" => Some(ZmqEvent::HashBlock { hash: data }),
            _ => {
                debug!("Unknown ZMQ topic: {}", topic);
                None
            }
        }
    }
}

fn split_tx_and_lock(data: Vec<u8>) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let mut cursor = Cursor::new(data.as_slice());
    match CoreTransaction::consensus_decode(&mut cursor) {
        Ok(_) => {
            let consumed = cursor.position() as usize;
            if consumed >= data.len() {
                // Transaction consumed all bytes, no lock data present
                (Some(data), None)
            } else {
                let lock_bytes = data[consumed..].to_vec();
                let tx_bytes = data[..consumed].to_vec();
                (Some(tx_bytes), Some(lock_bytes))
            }
        }
        Err(_) => (None, Some(data)),
    }
}

impl Drop for ZmqListener {
    fn drop(&mut self) {
        // Cancel all running tasks when dropped
        self.cancel.cancel();
    }
}

/// ZMQ dispatcher that receives messages from the socket and forwards them
/// to the provided sender (usually ZmqListener).
struct ZmqDispatcher {
    socket: SubSocket,
    /// Sender to forward received ZMQ messages, consumed by [ZmqConnection::recv]
    zmq_tx: mpsc::Sender<ZmqMessage>,
    /// Cancellation token to stop all spawned threads; cancelled when the connection is lost
    cancel: CancellationToken,
    connected: Arc<AtomicBool>,
    /// Time of last received message, in seconds since [START_TIME]
    last_recv: Arc<AtomicU64>,
}

impl ZmqDispatcher {
    /// Create a new ZmqDispatcher
    fn spawn(self, workers: &Workers) {
        let cancel = self.cancel.clone();
        workers.spawn(with_cancel(cancel, self.dispatcher_worker()));
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
                        Ok(msg) =>
                        {
                            if let Err(e) = self.zmq_tx.send(msg).await {
                                debug!(error = %e, "Error sending ZMQ event to receiver, receiver may have exited");
                                // receiver exited? I think it is fatal, we exit as it makes no sense to continue
                                self.connected.store(false, Ordering::SeqCst);
                                self.cancel.cancel();
                                return Err(DapiError::ClientGone("ZMQ receiver exited".to_string()));
                            } else {
                                // update last received timestamp
                                self.last_recv_update();
                            }
                        },
                        Err(e) => {
                            debug!(error = %e, "Error receiving ZMQ message, restarting connection");
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
        // if we have received a message in less than 10s, we are connected
        if self.last_recv_elapsed() < Duration::from_secs(10) {
            self.connected.store(true, Ordering::SeqCst);
            return;
        }

        // fallback to subscribing to some dummy `ping` topic.
        // This is a hack to ensure the connection is alive, as the monitor fails to notify us about disconnects.
        let current_status = self.socket.subscribe("ping").await.is_ok();
        // Unsubscribe immediately to avoid resource waste
        self.socket
            .unsubscribe("ping")
            .await
            .inspect_err(|e| {
                debug!(error = %e, "Error unsubscribing from ping topic during health check");
            })
            .ok();

        // If the status changed, log it
        let previous_status = self.connected.swap(current_status, Ordering::SeqCst);
        if current_status != previous_status {
            if current_status {
                debug!("ZMQ connection recovered");
            } else {
                debug!("ZMQ connection is lost, connection will be restarted");
                // disconnect the socket
                self.cancel.cancel();
            }
        }

        // if we are connected, we assume last_recv is now
        if current_status {
            self.last_recv_update();
        }
    }

    /// Get duration since last received message.
    /// Defaults to [START_TIME] on error.
    fn last_recv_elapsed(&self) -> Duration {
        let now = Instant::now();
        let start_time = *START_TIME;

        let last_recv_secs = self.last_recv.load(Ordering::Relaxed);
        let last_recv = START_TIME
            .checked_add(Duration::from_secs(last_recv_secs))
            .unwrap_or_else(|| {
                tracing::warn!(?start_time, ?now, "zmq last receive time out of bounds");
                *START_TIME
            });

        now.duration_since(last_recv)
    }

    /// Update the last received timestamp
    fn last_recv_update(&self) {
        let duration = Instant::now().duration_since(*START_TIME);

        self.last_recv.store(duration.as_secs(), Ordering::Relaxed);
    }
}

/// Helper function to run a future with cancellation support.
async fn with_cancel<T>(
    cancel: CancellationToken,
    future: impl Future<Output = DAPIResult<T>>,
) -> DAPIResult<T> {
    select! {
        _ = cancel.cancelled() => {
            debug!("Cancelled before future completed");
            Err(DapiError::ConnectionClosed)
        }
        result = future => result,
    }
}

#[cfg(test)]
mod tests {
    use super::split_tx_and_lock;
    use super::*;
    use dpp::dashcore::consensus::Decodable;
    use dpp::dashcore::{InstantLock, Transaction};
    use hex::FromHex;

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

    #[test]
    fn split_tx_and_lock_extracts_components() {
        let hex_bytes = "03000800014d6d36c50d484aa79f7db080f971c3f6845407f652c7d5865756017fa06969c1010000006a47304402200136894a2ebb4967cf2766c10e238d69c53c24bf330758e4432eb4753def03de02202a2afb05475a064a419a6cc1c582e3504fcb36c2e22b610b5d320f7656573f7f0121028fdb0a3f730bb20f477536d98ca830efa56412dd05992c801219ba0ff35ad530ffffffff028801030000000000026a00288d9500000000001976a9148d40dfe30494080a1c1187c74066956043ff13fb88ac0000000024010188010300000000001976a914aa85a9fb4f84bc63046a574ac4f2ce3361f0db0d88ac01014d6d36c50d484aa79f7db080f971c3f6845407f652c7d5865756017fa06969c1010000008155cc5d9fe5da3b0508c28d02c88fb6d3d4cf44ef4ffcd77162afa338d1a181ad7300e92255a7a7cf031d6de6bac99df9f1b94735ea603b3f03060c3ebf1f37acc4c1d8ddea77f3d4d816e467571f51ae216715fb3e47d68831adeee6aa1640b26cdf085bb8dd0b4920d15eed83e8c50de8b4b0508db47f08451f7807194d68758a92b367ef6074b516336f689c75c5e22b87aa71d50157875f1018a305a957";
        let data = Vec::from_hex(hex_bytes).expect("hex should decode");

        let (tx_bytes, lock_bytes) = split_tx_and_lock(data);

        assert!(tx_bytes.is_some(), "transaction bytes should be extracted");
        // Parse tx_bytes to ensure it's valid
        let tx = Transaction::consensus_decode(&mut Cursor::new(tx_bytes.as_ref().unwrap()))
            .expect("transaction bytes should decode");
        assert_eq!(tx.version, 3, "transaction version should be 3");

        // Parse lock_bytes to ensure it's valid
        assert!(
            lock_bytes.as_ref().is_some_and(|b| !b.is_empty()),
            "instant lock bytes should be present for rawtxlocksig payloads"
        );
        InstantLock::consensus_decode(&mut Cursor::new(lock_bytes.as_ref().unwrap()))
            .expect("instant asset lock should be correct");
    }
}
