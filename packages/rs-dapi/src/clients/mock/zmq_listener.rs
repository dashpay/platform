// Mock ZMQ listener for testing

use crate::error::DAPIResult;
use crate::services::streaming_service::{ZmqEvent, ZmqListenerTrait};
use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::time::Duration;

/// Mock ZMQ listener that doesn't connect to real ZMQ
pub struct MockZmqListener {
    event_sender: broadcast::Sender<ZmqEvent>,
    _event_receiver: broadcast::Receiver<ZmqEvent>,
}

impl MockZmqListener {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        Self {
            event_sender,
            _event_receiver: event_receiver,
        }
    }

    /// Send a mock event for testing
    pub fn send_mock_event(
        &self,
        event: ZmqEvent,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.event_sender.send(event)
    }

    /// Send mock transaction data
    pub fn send_mock_transaction(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.send_mock_event(ZmqEvent::RawTransaction { data })
    }

    /// Send mock block data
    pub fn send_mock_block(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.send_mock_event(ZmqEvent::RawBlock { data })
    }

    /// Send mock chain lock data
    pub fn send_mock_chain_lock(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.send_mock_event(ZmqEvent::RawChainLock { data })
    }

    /// Send mock instant lock data
    pub fn send_mock_instant_lock(
        &self,
        data: Vec<u8>,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.send_mock_event(ZmqEvent::RawTransactionLock { data })
    }

    /// Send mock block hash
    pub fn send_mock_block_hash(
        &self,
        hash: Vec<u8>,
    ) -> std::result::Result<usize, broadcast::error::SendError<ZmqEvent>> {
        self.send_mock_event(ZmqEvent::HashBlock { hash })
    }
}

impl Default for MockZmqListener {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ZmqListenerTrait for MockZmqListener {
    /// Start the mock ZMQ listener and return a receiver for events
    async fn subscribe(&self) -> DAPIResult<broadcast::Receiver<ZmqEvent>> {
        let receiver = self.event_sender.subscribe();

        // No actual ZMQ connection needed for mock
        // Optionally sleep briefly to simulate startup time
        tokio::time::sleep(Duration::from_millis(1)).await;

        Ok(receiver)
    }

    /// Mock is always "connected"
    fn is_connected(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_zmq_listener_creation() {
        let listener = MockZmqListener::new();
        assert!(listener.is_connected());
    }

    #[tokio::test]
    async fn test_mock_zmq_listener_start() {
        let listener = MockZmqListener::new();
        let _receiver = listener
            .subscribe()
            .await
            .expect("Should start successfully");
        // Test passes if no panic occurs
    }

    #[tokio::test]
    async fn test_mock_zmq_listener_events() {
        let listener = MockZmqListener::new();
        let mut receiver = listener
            .subscribe()
            .await
            .expect("Should start successfully");

        // Send a mock transaction
        let test_data = vec![1, 2, 3, 4, 5];
        listener
            .send_mock_transaction(test_data.clone())
            .expect("Should send mock event");

        // Receive the event
        let event = receiver.recv().await.expect("Should receive event");
        match event {
            ZmqEvent::RawTransaction { data } => {
                assert_eq!(data, test_data);
            }
            _ => panic!("Expected RawTransaction event"),
        }
    }
}
