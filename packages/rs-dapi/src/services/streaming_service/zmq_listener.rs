use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use zmq::{Context, Socket, SocketType};

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

/// ZMQ listener that connects to Dash Core and streams events
pub struct ZmqListener {
    zmq_uri: String,
    topics: ZmqTopics,
    event_sender: broadcast::Sender<ZmqEvent>,
    _event_receiver: broadcast::Receiver<ZmqEvent>,
}

impl ZmqListener {
    pub fn new(zmq_uri: &str) -> Self {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        Self {
            zmq_uri: zmq_uri.to_string(),
            topics: ZmqTopics::default(),
            event_sender,
            _event_receiver: event_receiver,
        }
    }

    /// Start the ZMQ listener and return a receiver for events
    pub async fn start(&self) -> Result<broadcast::Receiver<ZmqEvent>> {
        let receiver = self.event_sender.subscribe();

        // Start the ZMQ listener in a background thread
        let zmq_uri = self.zmq_uri.clone();
        let topics = self.topics.clone();
        let sender = self.event_sender.clone();

        tokio::task::spawn_blocking(move || {
            if let Err(e) = Self::zmq_listener_thread(zmq_uri, topics, sender) {
                error!("ZMQ listener thread error: {}", e);
            }
        });

        // Give the ZMQ connection a moment to establish
        sleep(Duration::from_millis(100)).await;

        Ok(receiver)
    }

    /// ZMQ listener thread that runs in a blocking context
    fn zmq_listener_thread(
        zmq_uri: String,
        topics: ZmqTopics,
        sender: broadcast::Sender<ZmqEvent>,
    ) -> Result<()> {
        info!("Starting ZMQ listener on {}", zmq_uri);

        let context = Context::new();
        let socket = context.socket(SocketType::SUB)?;

        // Subscribe to all topics
        socket.set_subscribe(topics.rawtx.as_bytes())?;
        socket.set_subscribe(topics.rawblock.as_bytes())?;
        socket.set_subscribe(topics.rawtxlocksig.as_bytes())?;
        socket.set_subscribe(topics.rawchainlocksig.as_bytes())?;
        socket.set_subscribe(topics.hashblock.as_bytes())?;

        // Set socket options
        socket.set_rcvhwm(1000)?;
        socket.set_linger(0)?;

        // Connect to Dash Core ZMQ
        socket.connect(&zmq_uri)?;
        info!("Connected to ZMQ at {}", zmq_uri);

        loop {
            match Self::receive_zmq_message(&socket, &topics) {
                Ok(Some(event)) => {
                    debug!("Received ZMQ event: {:?}", event);
                    if let Err(e) = sender.send(event) {
                        warn!("Failed to send ZMQ event to subscribers: {}", e);
                    }
                }
                Ok(None) => {
                    // No message or unknown topic, continue
                }
                Err(e) => {
                    error!("Error receiving ZMQ message: {}", e);
                    // Sleep briefly before retrying
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    /// Receive and parse a ZMQ message
    fn receive_zmq_message(socket: &Socket, topics: &ZmqTopics) -> Result<Option<ZmqEvent>> {
        // Receive multipart message (topic + data)
        let parts = socket.recv_multipart(zmq::DONTWAIT)?;

        if parts.len() < 2 {
            return Ok(None);
        }

        let topic = String::from_utf8_lossy(&parts[0]);
        let data = parts[1].clone();

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

    /// Check if the ZMQ listener is connected (placeholder)
    pub fn is_connected(&self) -> bool {
        // In a real implementation, this would check the socket state
        true
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
