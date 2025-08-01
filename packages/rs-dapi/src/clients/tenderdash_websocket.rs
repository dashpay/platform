use crate::{DAPIResult, DapiError};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub hash: String,
    pub height: u64,
    pub result: TransactionResult,
    pub tx: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionResult {
    Success,
    Error {
        code: u32,
        info: String,
        data: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TenderdashWsMessage {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventData {
    #[serde(rename = "type")]
    event_type: String,
    value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxEvent {
    height: String,
    tx: Option<String>,
    result: Option<TxResult>,
    events: Option<Vec<EventAttribute>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxResult {
    code: u32,
    data: Option<String>,
    info: Option<String>,
    log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventAttribute {
    key: String,
    value: String,
}

#[derive(Debug)]
pub struct TenderdashWebSocketClient {
    ws_url: String,
    event_sender: broadcast::Sender<TransactionEvent>,
    is_connected: Arc<AtomicBool>,
}

impl TenderdashWebSocketClient {
    pub fn new(ws_url: String, buffer_size: usize) -> Self {
        let (event_sender, _) = broadcast::channel(buffer_size);

        Self {
            ws_url,
            event_sender,
            is_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TransactionEvent> {
        self.event_sender.subscribe()
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub async fn connect_and_listen(&self) -> DAPIResult<()> {
        info!("Connecting to Tenderdash WebSocket at {}", self.ws_url);

        // Validate URL format
        let _url = url::Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(&self.ws_url).await?;

        self.is_connected.store(true, Ordering::Relaxed);
        info!("Connected to Tenderdash WebSocket");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to transaction events
        let subscribe_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "id": 1,
            "params": {
                "query": "tm.event = 'Tx'"
            }
        });

        ws_sender
            .send(Message::Text(subscribe_msg.to_string()))
            .await?;

        debug!("Subscribed to Tenderdash transaction events");

        let event_sender = self.event_sender.clone();
        let is_connected = Arc::clone(&self.is_connected);

        // Listen for messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.handle_message(&text, &event_sender).await {
                        warn!("Failed to handle WebSocket message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {
                    // Ignore other message types (ping, pong, binary)
                }
            }
        }

        is_connected.store(false, Ordering::Relaxed);
        info!("Disconnected from Tenderdash WebSocket");

        Ok(())
    }

    async fn handle_message(
        &self,
        message: &str,
        event_sender: &broadcast::Sender<TransactionEvent>,
    ) -> DAPIResult<()> {
        let ws_message: TenderdashWsMessage = serde_json::from_str(message)?;

        // Skip subscription confirmations and other non-event messages
        if ws_message.result.is_none() {
            return Ok(());
        }

        let result = ws_message.result.unwrap();

        // Check if this is an event message
        if result.get("events").is_some() {
            if let Some(data) = result.get("data") {
                if let Some(value) = data.get("value") {
                    return self.handle_tx_event(value, event_sender).await;
                }
            }
        }

        Ok(())
    }

    async fn handle_tx_event(
        &self,
        event_data: &serde_json::Value,
        event_sender: &broadcast::Sender<TransactionEvent>,
    ) -> DAPIResult<()> {
        let tx_event: TxEvent = serde_json::from_value(event_data.clone())?;

        // Extract transaction hash from events
        let hash = self.extract_tx_hash(&tx_event.events)?;

        let height = tx_event.height.parse::<u64>().unwrap_or(0);

        // Decode transaction if present
        let tx = if let Some(tx_base64) = &tx_event.tx {
            base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, tx_base64).ok()
        } else {
            None
        };

        // Determine transaction result
        let result = if let Some(tx_result) = &tx_event.result {
            if tx_result.code == 0 {
                TransactionResult::Success
            } else {
                TransactionResult::Error {
                    code: tx_result.code,
                    info: tx_result.info.clone().unwrap_or_default(),
                    data: tx_result.data.clone(),
                }
            }
        } else {
            TransactionResult::Success
        };

        let transaction_event = TransactionEvent {
            hash: hash.clone(),
            height,
            result,
            tx,
        };

        debug!("Broadcasting transaction event for hash: {}", hash);

        // Broadcast the event (ignore if no subscribers)
        let _ = event_sender.send(transaction_event);

        Ok(())
    }

    fn extract_tx_hash(&self, events: &Option<Vec<EventAttribute>>) -> DAPIResult<String> {
        if let Some(events) = events {
            for event in events {
                if event.key == "hash" {
                    return Ok(event.value.clone());
                }
            }
        }

        Err(DapiError::Client(
            "Transaction hash not found in event attributes".to_string(),
        ))
    }
}
