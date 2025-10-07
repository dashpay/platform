use crate::{
    DAPIResult, DapiError,
    utils::{deserialize_string_or_number, deserialize_to_string, generate_jsonrpc_id},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub hash: String,
    pub height: u64,
    pub result: TransactionResult,
    pub tx: Option<Vec<u8>>,
}

/// Block event placeholder (TODO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEvent {}

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
    #[serde(deserialize_with = "deserialize_string_or_number")]
    height: u64,
    tx: Option<String>,
    result: Option<TxResult>,
    events: Option<Vec<EventAttribute>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxResult {
    #[serde(
        deserialize_with = "deserialize_string_or_number",
        default = "default_code"
    )]
    code: u32,
    data: Option<String>,
    info: Option<String>,
    log: Option<String>,
}

// Default function for code field
fn default_code() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventAttribute {
    key: String,
    #[serde(deserialize_with = "deserialize_to_string")]
    value: String,
}

#[derive(Debug)]
pub struct TenderdashWebSocketClient {
    ws_url: String,
    event_sender: broadcast::Sender<TransactionEvent>,
    is_connected: Arc<AtomicBool>,
    block_sender: broadcast::Sender<BlockEvent>,
}

impl TenderdashWebSocketClient {
    /// Create a WebSocket client with broadcast channels for transactions and blocks.
    pub fn new(ws_url: String, buffer_size: usize) -> Self {
        let (event_sender, _) = broadcast::channel(buffer_size);
        let (block_sender, _) = broadcast::channel(buffer_size);

        Self {
            ws_url,
            event_sender,
            is_connected: Arc::new(AtomicBool::new(false)),
            block_sender,
        }
    }

    /// Subscribe to transaction event updates emitted by the listener.
    pub fn subscribe(&self) -> broadcast::Receiver<TransactionEvent> {
        self.event_sender.subscribe()
    }

    /// Indicate whether a WebSocket connection is currently active.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    /// Subscribe to Tenderdash new-block notifications.
    pub fn subscribe_blocks(&self) -> broadcast::Receiver<BlockEvent> {
        self.block_sender.subscribe()
    }

    /// Test WebSocket connection without establishing a persistent connection
    pub async fn test_connection(ws_url: &str) -> DAPIResult<()> {
        tracing::trace!("Testing WebSocket connection to {}", ws_url);

        // Validate URL format
        let _url = url::Url::parse(ws_url)?;

        // Try to connect
        let (_ws_stream, _) = connect_async(ws_url).await?;

        tracing::trace!("WebSocket connection test successful");
        Ok(())
    }

    /// Establish a WebSocket connection, subscribe to events, and forward messages to subscribers.
    pub async fn connect_and_listen(&self) -> DAPIResult<()> {
        tracing::trace!(ws_url = self.ws_url, "Connecting to Tenderdash WebSocket");

        // Validate URL format
        let _url = url::Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(&self.ws_url).await?;

        self.is_connected.store(true, Ordering::Relaxed);
        tracing::debug!(ws_url = self.ws_url, "Connected to Tenderdash WebSocket");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to transaction events
        let subscribe_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "id": generate_jsonrpc_id(),
            "params": {
                "query": "tm.event = 'Tx'"
            }
        });

        ws_sender
            .send(Message::Text(subscribe_msg.to_string()))
            .await?;

        // Subscribe to new block events
        let subscribe_block_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "id": generate_jsonrpc_id(),
            "params": {
                "query": "tm.event = 'NewBlock'"
            }
        });
        ws_sender
            .send(Message::Text(subscribe_block_msg.to_string()))
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

    /// Process a raw WebSocket message, dispatching block and transaction events.
    async fn handle_message(
        &self,
        message: &str,
        event_sender: &broadcast::Sender<TransactionEvent>,
    ) -> DAPIResult<()> {
        let ws_message: TenderdashWsMessage = serde_json::from_str(message).inspect_err(|e| {
            debug!(
                "Failed to parse WebSocket message as TenderdashWsMessage: {}",
                e
            );
            trace!("Raw message: {}", message);
        })?;

        // Skip subscription confirmations and other non-event messages
        if ws_message.result.is_none() {
            return Ok(());
        }

        let result = ws_message.result.unwrap();

        // NewBlock notifications include a query matching NewBlock
        if let Some(query) = result.get("query").and_then(|q| q.as_str())
            && query.contains("NewBlock")
        {
            let _ = self.block_sender.send(BlockEvent {});
            return Ok(());
        }

        // Check if this is a tx event message
        if result.get("events").is_some()
            && let Some(data) = result.get("data")
            && let Some(value) = data.get("value")
        {
            return self.handle_tx_event(value, event_sender, &result).await;
        }

        Ok(())
    }

    /// Convert a Tenderdash transaction event payload into broadcastable events.
    async fn handle_tx_event(
        &self,
        event_data: &serde_json::Value,
        event_sender: &broadcast::Sender<TransactionEvent>,
        outer_result: &serde_json::Value,
    ) -> DAPIResult<()> {
        let tx_event: TxEvent = serde_json::from_value(event_data.clone())?;

        // Extract all transaction hashes from events
        let hashes = self.extract_all_tx_hashes(&tx_event.events, outer_result)?;

        if hashes.is_empty() {
            warn!(
                ?tx_event,
                "No transaction hashes found in event attributes for event.",
            );
            return Err(DapiError::TransactionHashNotFound);
        }

        // Log if we found multiple hashes (unusual case)
        if hashes.len() > 1 {
            warn!(
                "Found {} transaction hashes in single WebSocket message: {:?}",
                hashes.len(),
                hashes
            );
        }

        // Process each hash (typically just one)
        for hash in hashes {
            let height = tx_event.height;

            // Decode transaction if present
            let tx: Option<Vec<u8>> = if let Some(tx_base64) = &tx_event.tx {
                Some(base64::prelude::Engine::decode(
                    &base64::prelude::BASE64_STANDARD,
                    tx_base64,
                )?)
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
                result: result.clone(),
                tx: tx.clone(),
            };

            debug!(hash = %hash, "Broadcasting transaction event for hash");

            // Broadcast the event (ignore if no subscribers)
            let _ = event_sender.send(transaction_event);
        }

        Ok(())
    }

    /// Gather unique transaction hashes from outer and inner event attribute sets.
    fn extract_all_tx_hashes(
        &self,
        inner_events: &Option<Vec<EventAttribute>>,
        outer_result: &serde_json::Value,
    ) -> DAPIResult<Vec<String>> {
        let mut hashes = Vec::new();

        // First extract from outer events (result.events) - this is the primary location
        if let Some(outer_events) = outer_result.get("events").and_then(|e| e.as_array()) {
            for event in outer_events {
                if let Some(event_type) = event.get("type").and_then(|t| t.as_str())
                    && event_type == "tx"
                    && let Some(attributes) = event.get("attributes").and_then(|a| a.as_array())
                {
                    for attr in attributes {
                        if let (Some(key), Some(value)) = (
                            attr.get("key").and_then(|k| k.as_str()),
                            attr.get("value").and_then(|v| v.as_str()),
                        ) && key == "hash"
                        {
                            hashes.push(normalize_event_hash(value));
                        }
                    }
                }
            }
        }

        // Also check inner events (TxEvent.events) as fallback
        if let Some(events) = inner_events {
            for event in events {
                if event.key == "hash" {
                    hashes.push(normalize_event_hash(&event.value));
                }
            }
        }

        // Remove duplicates while preserving order efficiently
        let mut seen = BTreeSet::new();
        let unique_hashes: Vec<String> = hashes
            .into_iter()
            .filter(|hash| seen.insert(hash.clone()))
            .collect();

        Ok(unique_hashes)
    }
}

/// Normalize hash strings by trimming prefixes and uppercasing hexadecimal characters.
fn normalize_event_hash(value: &str) -> String {
    let trimmed = value.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        without_prefix.to_uppercase()
    } else {
        without_prefix.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tx_event_deserialization_with_string_height() {
        let json_data = json!({
            "height": "12345",
            "tx": "dGVzdA==",
            "result": {
                "code": 0,
                "data": null,
                "info": "",
                "log": ""
            },
            "events": []
        });

        let tx_event: TxEvent = serde_json::from_value(json_data).unwrap();
        assert_eq!(tx_event.height, 12345);
    }

    #[test]
    fn test_tx_event_deserialization_with_integer_height() {
        let json_data = json!({
            "height": 12345,
            "tx": "dGVzdA==",
            "result": {
                "code": 0,
                "data": null,
                "info": "",
                "log": ""
            },
            "events": []
        });

        let tx_event: TxEvent = serde_json::from_value(json_data).unwrap();
        assert_eq!(tx_event.height, 12345);
    }

    #[test]
    fn test_tx_result_deserialization_with_string_code() {
        let json_data = json!({
            "code": "1005",
            "data": null,
            "info": "test error",
            "log": ""
        });

        let tx_result: TxResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(tx_result.code, 1005);
    }

    #[test]
    fn test_tx_result_deserialization_with_integer_code() {
        let json_data = json!({
            "code": 1005,
            "data": null,
            "info": "test error",
            "log": ""
        });

        let tx_result: TxResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(tx_result.code, 1005);
    }

    #[test]
    fn test_tx_result_deserialization_with_missing_code() {
        let json_data = json!({
            "gas_used": 905760,
            "data": null,
            "info": "",
            "log": ""
        });

        let tx_result: TxResult = serde_json::from_value(json_data).unwrap();
        assert_eq!(tx_result.code, 0); // Should default to 0 (success)
    }

    #[test]
    fn test_real_websocket_message_deserialization() {
        // This is the actual WebSocket message that was causing the "missing field `code`" error
        let json_data = json!({
            "height": 1087,
            "tx": "BwBKtJbhBYdn6SJx+oezzOb0KjQAhV2vh0pXlAsN3u0soZ1vsfjXvOK0TA6z9UnzQoIRj2entd3N2XUQ8qmFOYML/DuaygABAANBIIBqaHzVMKT/AvClrEuKY6/kwgtQmZmaOGSOrLqGEhrBVf62e/mcTkqIrUruBQ/xdtxDYs0tj/32zt+yVTJH7j8=",
            "result": {
                "gas_used": 905760
                // Note: no "code" field - should default to 0
            },
            "events": [
                {
                    "key": "hash",
                    "value": "13F2EF4097320B234DECCEF063FDAE6A0845AF4380CEC15F2185CE9FACC6EBD5"
                },
                {
                    "key": "height",
                    "value": "1087"
                }
            ]
        });

        let tx_event: TxEvent = serde_json::from_value(json_data).unwrap();

        // Verify all fields are correctly deserialized
        assert_eq!(tx_event.height, 1087);
        assert!(tx_event.tx.is_some());

        // Verify the result has default code of 0 (success)
        let result = tx_event.result.unwrap();
        assert_eq!(result.code, 0);

        // Verify events are correctly parsed
        let events = tx_event.events.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].key, "hash");
        assert_eq!(
            events[0].value,
            "13F2EF4097320B234DECCEF063FDAE6A0845AF4380CEC15F2185CE9FACC6EBD5"
        );
        assert_eq!(events[1].key, "height");
        assert_eq!(events[1].value, "1087"); // String conversion of integer value
    }

    #[test]
    fn test_full_websocket_message_deserialization() {
        // This is the complete WebSocket message that was failing, including the outer JSON-RPC wrapper
        let full_message = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "subscription_id": "",
                "query": "tm.event = 'Tx'",
                "data": {
                    "type": "tendermint/event/Tx",
                    "value": {
                        "height": 1087,
                        "tx": "BwBKtJbhBYdn6SJx+oezzOb0KjQAhV2vh0pXlAsN3u0soZ1vsfjXvOK0TA6z9UnzQoIRj2entd3N2XUQ8qmFOYML/DuaygABAANBIIBqaHzVMKT/AvClrEuKY6/kwgtQmZmaOGSOrLqGEhrBVf62e/mcTkqIrUruBQ/xdtxDYs0tj/32zt+yVTJH7j8=",
                        "result": {
                            "gas_used": 905760
                        }
                    }
                },
                "events": [
                    {
                        "type": "tm",
                        "attributes": [
                            {
                                "key": "event",
                                "value": "Tx",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "hash",
                                "value": "13F2EF4097320B234DECCEF063FDAE6A0845AF4380CEC15F2185CE9FACC6EBD5",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "height",
                                "value": "1087",
                                "index": false
                            }
                        ]
                    }
                ]
            }
        }"#;

        // Test that the outer message parses correctly
        let ws_message: TenderdashWsMessage = serde_json::from_str(full_message).unwrap();
        assert_eq!(ws_message.jsonrpc, "2.0");
        assert!(ws_message.result.is_some());

        // Test that we can extract the inner tx event data
        let result = ws_message.result.unwrap();
        let data = result.get("data").unwrap();
        let value = data.get("value").unwrap();

        // This should deserialize without the "missing field `code`" error
        let tx_event: TxEvent = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(tx_event.height, 1087);

        // Verify the result defaults to code 0 when missing
        let tx_result = tx_event.result.unwrap();
        assert_eq!(tx_result.code, 0);
    }

    #[test]
    fn test_hash_in_outer_events_websocket_message() {
        // This reproduces the actual failing WebSocket message structure where the hash
        // is in the outer events array, not in the inner tx_event.events
        let full_message = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "subscription_id": "",
                "query": "tm.event = 'Tx'",
                "data": {
                    "type": "tendermint/event/Tx",
                    "value": {
                        "height": 1143,
                        "tx": "AwAEAAACAAAAABRoMDrccS7MNWBQ3j8+Irst5weWvAAAAQIAAQAAFDDoQkib1LvN+VIdf/tBEjPb8tmgAAACAgACAAAUjB/xAqiSZfRjX/0gvUCXATi06uQAAAMCAwEAABSqQPiOK2TfNerKRS3LkaD2x8G6GwAAxgEBcFMtXqPhk3AVd47C+6SSmXWl6BS8ehgBC6CSbbbU8hQBAAAAQCPGVEX1xA4ur9Iz2LdDyyfS8YE4x5Q6mYG/SS0xAGx6v3Gcn7oGsRFemDL+rYN5/cg3CqDLrXIl2SsotyB5BI79o8jb7Nf6MwHM0ZKU3ikwss37YUwNvJkZ57UZPf4txIqg7qN0oEjEynsX4tjv6BWrPlaEWTiyVjuYOCbuvHZBpPQ55cJ4+9ya/05J1C8KdIjaGuyB1r0yA6eLaXNBmu8DAAgAAXBTLV6j4ZNwFXeOwvukkpl1pegUvHoYAQugkm221PIUAQAAAGpHMEQCIC4nPoswVruvuSo5uIMs8vW7N1IowC8PxfjYlTnUy4fXAiAsgVn9e1kGYaunZI+LOeiJ1ghEMAS7u5WPP13tS7L9ZQEhA1xnCKgAxtiWPLxpfBMPmBetAiJKQn//lQLmSMatlduV/////wLA6iEBAAAAAAJqABiPRzkAAAAAGXapFDPxaffrRV2b5uJzofsIIsP3xBWiiKwAAAAAJAEBwOohAQAAAAAZdqkUtQHJZWYFWMlOKQjvCePbD4EAi8CIrAAAQR/5fcqaM3VWmUOBwWHSHQtbDNCKopIN/L6USHBk5jNp+gne/1nL/Cd0UjtaFGkuAkJbdLTgrDEIQU1rbtZQ3lBSMbRnV8B6UIWAY3z9q2tOSeTQ3FybD5iEd0Oo/dzJldM=",
                        "result": {
                            "gas_used": 130192500
                        }
                    }
                },
                "events": [
                    {
                        "type": "tm",
                        "attributes": [
                            {
                                "key": "event",
                                "value": "Tx",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "hash",
                                "value": "FCF3B0D09B8042B7A41F514107CBE1E09BD33C222005A8669A3EBE4B1D59BDDF",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "height",
                                "value": "1143",
                                "index": false
                            }
                        ]
                    }
                ]
            }
        }"#;

        // Test that the outer message parses correctly
        let ws_message: TenderdashWsMessage = serde_json::from_str(full_message).unwrap();
        let result = ws_message.result.unwrap();
        let data = result.get("data").unwrap();
        let value = data.get("value").unwrap();

        // The inner tx event should deserialize but won't have events
        let tx_event: TxEvent = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(tx_event.height, 1143);

        // The inner tx_event.events is None, but we should be able to extract hash from outer events
        assert!(tx_event.events.is_none());

        // Test that the modified extract_all_tx_hashes function now works with outer events
        let client = TenderdashWebSocketClient::new("ws://test".to_string(), 100);
        let hashes = client
            .extract_all_tx_hashes(&tx_event.events, &result)
            .unwrap();

        assert_eq!(hashes.len(), 1);
        assert_eq!(
            hashes[0],
            "FCF3B0D09B8042B7A41F514107CBE1E09BD33C222005A8669A3EBE4B1D59BDDF"
        );
    }

    #[test]
    fn test_multiple_hashes_in_websocket_message() {
        // Test case where multiple tx events each contain a hash (edge case)
        let multiple_hash_message = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "query": "tm.event = 'Tx'",
                "data": {
                    "type": "tendermint/event/Tx",
                    "value": {
                        "height": "200",
                        "tx": "dGVzdA==",
                        "result": {}
                    }
                },
                "events": [
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "hash",
                                "value": "HASH1",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "hash", 
                                "value": "HASH2",
                                "index": false
                            }
                        ]
                    },
                    {
                        "type": "tx",
                        "attributes": [
                            {
                                "key": "height",
                                "value": "200",
                                "index": false
                            }
                        ]
                    }
                ]
            }
        }"#;

        let ws_message: TenderdashWsMessage = serde_json::from_str(multiple_hash_message).unwrap();
        let result = ws_message.result.unwrap();
        let data = result.get("data").unwrap();
        let value = data.get("value").unwrap();

        let tx_event: TxEvent = serde_json::from_value(value.clone()).unwrap();
        let client = TenderdashWebSocketClient::new("ws://test".to_string(), 100);
        let hashes = client
            .extract_all_tx_hashes(&tx_event.events, &result)
            .unwrap();

        // Should find both hashes
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0], "HASH1");
        assert_eq!(hashes[1], "HASH2");
    }

    #[test]
    fn test_event_attribute_deserialization_with_integer_value() {
        let json_data = json!({
            "key": "hash",
            "value": 1005
        });

        let event_attr: EventAttribute = serde_json::from_value(json_data).unwrap();
        assert_eq!(event_attr.value, "1005");
    }

    #[test]
    fn test_event_attribute_deserialization_with_string_value() {
        let json_data = json!({
            "key": "hash",
            "value": "abc123"
        });

        let event_attr: EventAttribute = serde_json::from_value(json_data).unwrap();
        assert_eq!(event_attr.value, "abc123");
    }
}
