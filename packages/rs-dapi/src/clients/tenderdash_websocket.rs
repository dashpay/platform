use crate::{
    DAPIResult, DapiError,
    clients::{CONNECT_TIMEOUT, tenderdash_client::ExecTxResult},
    utils::{deserialize_string_or_number, deserialize_to_string, generate_jsonrpc_id},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{sync::broadcast, time::timeout};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};
use tracing::{debug, error, info, trace, warn};

/// The largest WebSocket message and frame accepted from Tenderdash.
///
/// Tenderdash serialises every event as one JSON-RPC message and writes it as a single
/// uncompressed frame, so tungstenite's defaults (64 MiB per message, 16 MiB per frame) decide
/// whether an event arrives at all. A `Tx` event carries the transaction base64-encoded, and a
/// `NewBlock` event carries the whole block including its transactions the same way: a
/// contract-code state transition at the 32 MiB family cap becomes about 44.7 MB of base64,
/// and a block at the 36 MiB `block_max_bytes` about 50.3 MB. A frame limit below that
/// disconnects the listener exactly when the transaction `waitForStateTransitionResult` is
/// waiting for is committed. Both limits are therefore set to this value, which a test pins
/// above the base64 size of the largest block every registered protocol version allows plus
/// the JSON envelope.
pub const MAX_TENDERDASH_WS_MESSAGE_BYTES: usize = 64 * 1024 * 1024;

/// The tungstenite configuration every connection to Tenderdash uses.
fn tenderdash_websocket_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(MAX_TENDERDASH_WS_MESSAGE_BYTES),
        max_frame_size: Some(MAX_TENDERDASH_WS_MESSAGE_BYTES),
        ..WebSocketConfig::default()
    }
}

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
struct TxEvent {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    height: u64,
    tx: Option<String>,
    result: Option<ExecTxResult>,
    events: Option<Vec<EventAttribute>>,
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
        let (mut ws_stream, _) = timeout(
            CONNECT_TIMEOUT,
            connect_async_with_config(ws_url, Some(tenderdash_websocket_config()), false),
        )
        .await
        .map_err(|e| DapiError::timeout(format!("WebSocket connection test timed out: {e}")))??;

        ws_stream
            .close(None)
            .await
            .map_err(|e| DapiError::Client(format!("WebSocket connection close failed: {e}")))?;
        tracing::trace!("WebSocket connection test successful");
        Ok(())
    }

    /// Establish a WebSocket connection, subscribe to events, and forward messages to subscribers.
    pub async fn connect_and_listen(&self) -> DAPIResult<()> {
        tracing::trace!(ws_url = self.ws_url, "Connecting to Tenderdash WebSocket");

        // Validate URL format
        let _url = url::Url::parse(&self.ws_url)?;
        let (ws_stream, _) = timeout(
            CONNECT_TIMEOUT,
            connect_async_with_config(&self.ws_url, Some(tenderdash_websocket_config()), false),
        )
        .await
        .map_err(|e| DapiError::timeout(format!("WebSocket connect timed out: {e}")))??;

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
                        info: tx_result.info.clone(),
                        data: if tx_result.data.is_empty() {
                            None
                        } else {
                            Some(tx_result.data.clone())
                        },
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

    mod large_events {
        use super::*;
        use base64::prelude::{BASE64_STANDARD, Engine};
        use dpp::version::{PLATFORM_VERSIONS, PlatformVersion};
        use std::time::Duration;
        use tokio::net::TcpListener;
        use tokio_tungstenite::{accept_async_with_config, connect_async};

        /// Base64 grows the payload by a third; the JSON-RPC envelope, the event attributes
        /// and the block header sit inside this allowance.
        const ENVELOPE_HEADROOM_BYTES: usize = 1024 * 1024;

        fn base64_len(raw_len: usize) -> usize {
            raw_len.div_ceil(3) * 4
        }

        /// The cap must hold the largest block any registered protocol version lets
        /// Tenderdash build (its `block_max_bytes`, or the genesis 2 MiB where a version sets
        /// none) after base64 encoding, with room for the envelope, or the `NewBlock` event of
        /// a full block disconnects the listener.
        #[test]
        fn message_cap_exceeds_the_base64_size_of_every_versions_largest_block() {
            const GENESIS_BLOCK_MAX_BYTES: u64 = 2 * 1024 * 1024;
            for platform_version in PLATFORM_VERSIONS {
                let block_max_bytes = platform_version
                    .consensus
                    .block_max_bytes
                    .unwrap_or(GENESIS_BLOCK_MAX_BYTES)
                    as usize;
                assert!(
                    base64_len(block_max_bytes) + ENVELOPE_HEADROOM_BYTES
                        <= MAX_TENDERDASH_WS_MESSAGE_BYTES,
                    "protocol version {} allows a {block_max_bytes} byte block whose NewBlock \
                     event would not fit the WebSocket message cap",
                    platform_version.protocol_version
                );
            }
        }

        fn tx_event_message(tx: &[u8]) -> String {
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "subscription_id": "",
                    "query": "tm.event = 'Tx'",
                    "data": {
                        "type": "tendermint/event/Tx",
                        "value": {
                            "height": 4242,
                            "tx": BASE64_STANDARD.encode(tx),
                            "result": { "gas_used": 1 }
                        }
                    },
                    "events": [
                        {
                            "type": "tx",
                            "attributes": [
                                { "key": "hash", "value": "AB".repeat(32), "index": false }
                            ]
                        }
                    ]
                }
            })
            .to_string()
        }

        fn new_block_event_message(block_bytes: &[u8]) -> String {
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "result": {
                    "subscription_id": "",
                    "query": "tm.event = 'NewBlock'",
                    "data": {
                        "type": "tendermint/event/NewBlock",
                        "value": {
                            "block": {
                                "data": { "txs": [BASE64_STANDARD.encode(block_bytes)] }
                            }
                        }
                    },
                    "events": []
                }
            })
            .to_string()
        }

        /// A local server standing in for Tenderdash: every connection receives a `Tx` event
        /// carrying a transaction at the family cap and a `NewBlock` event carrying a block at
        /// the block cap, each as one frame the way Tenderdash writes them, then a close.
        async fn spawn_event_server(tx: Vec<u8>, block: Vec<u8>) -> String {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let address = listener.local_addr().expect("local address");
            tokio::spawn(async move {
                loop {
                    let (stream, _) = listener.accept().await.expect("accept");
                    let tx = tx.clone();
                    let block = block.clone();
                    tokio::spawn(async move {
                        let server_config = WebSocketConfig {
                            max_message_size: None,
                            max_frame_size: None,
                            ..WebSocketConfig::default()
                        };
                        let mut ws = accept_async_with_config(stream, Some(server_config))
                            .await
                            .expect("server handshake");
                        // Tenderdash acknowledges the subscriptions first.
                        for _ in 0..2 {
                            if let Some(Ok(Message::Text(_))) = ws.next().await {
                                let ack = json!({ "jsonrpc": "2.0", "id": 0, "result": {} });
                                let _ = ws.send(Message::Text(ack.to_string())).await;
                            }
                        }
                        let _ = ws.send(Message::Text(tx_event_message(&tx))).await;
                        let _ = ws
                            .send(Message::Text(new_block_event_message(&block)))
                            .await;
                        let _ = ws.send(Message::Close(None)).await;
                        while let Some(Ok(_)) = ws.next().await {}
                    });
                }
            });
            format!("ws://{address}")
        }

        fn family_cap_transaction() -> Vec<u8> {
            let family_cap = PlatformVersion::latest()
                .system_limits
                .max_contract_code_state_transition_size
                .expect("the latest version bounds contract code envelopes")
                as usize;
            vec![0x5Au8; family_cap]
        }

        fn block_cap_block() -> Vec<u8> {
            let block_cap = PlatformVersion::latest()
                .consensus
                .block_max_bytes
                .expect("the latest version sets the block byte cap")
                as usize;
            vec![0xA5u8; block_cap]
        }

        /// The configured connection delivers a `Tx` event carrying a transaction at the
        /// family cap and a `NewBlock` event carrying a block at the block cap, each larger
        /// than tungstenite's default frame limit.
        #[tokio::test]
        async fn configured_connection_delivers_large_tx_and_new_block_events() {
            let tx = family_cap_transaction();
            let ws_url = spawn_event_server(tx.clone(), block_cap_block()).await;

            let client = Arc::new(TenderdashWebSocketClient::new(ws_url, 8));
            let mut tx_events = client.subscribe();
            let mut block_events = client.subscribe_blocks();
            let listener = {
                let client = Arc::clone(&client);
                tokio::spawn(async move { client.connect_and_listen().await })
            };

            let tx_event = timeout(Duration::from_secs(60), tx_events.recv())
                .await
                .expect("the Tx event arrives in time")
                .expect("the Tx event is broadcast");
            assert_eq!(tx_event.height, 4242);
            assert_eq!(tx_event.tx.as_deref(), Some(tx.as_slice()));
            timeout(Duration::from_secs(60), block_events.recv())
                .await
                .expect("the NewBlock event arrives in time")
                .expect("the NewBlock event is broadcast");

            timeout(Duration::from_secs(60), listener)
                .await
                .expect("the listener returns after the close")
                .expect("the listener task completes")
                .expect("the listener exits cleanly");
        }

        /// The same events through tungstenite's defaults are refused at the frame limit,
        /// which is what the configuration above prevents.
        #[tokio::test]
        async fn default_connection_refuses_the_same_events() {
            let ws_url = spawn_event_server(family_cap_transaction(), block_cap_block()).await;
            let (mut ws, _) = connect_async(&ws_url).await.expect("connect");
            ws.send(Message::Text(String::new())).await.expect("send");
            ws.send(Message::Text(String::new())).await.expect("send");
            let mut refused = false;
            while let Some(message) = timeout(Duration::from_secs(60), ws.next())
                .await
                .expect("the server keeps sending")
            {
                match message {
                    Ok(Message::Text(text)) if text.contains("tm.event") => {
                        panic!("a {} byte event passed the default frame limit", text.len())
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        refused = true;
                        assert!(
                            matches!(error, tokio_tungstenite::tungstenite::Error::Capacity(_)),
                            "expected the frame limit to refuse the event, got {error:?}"
                        );
                        break;
                    }
                }
            }
            assert!(
                refused,
                "the default frame limit must refuse a family-cap event"
            );
        }
    }

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

        let tx_result: ExecTxResult = serde_json::from_value(json_data).unwrap();
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

        let tx_result: ExecTxResult = serde_json::from_value(json_data).unwrap();
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

        let tx_result: ExecTxResult = serde_json::from_value(json_data).unwrap();
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
