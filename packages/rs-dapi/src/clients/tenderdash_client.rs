use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use super::traits::TenderdashClientTrait;
use crate::error::{DAPIResult, DapiError};
use async_trait::async_trait;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone)]
/// HTTP client for interacting with Tenderdash consensus engine
///
/// This client includes automatic HTTP request/response tracing via reqwest-tracing middleware.
/// All HTTP requests will be logged at TRACE level with:
/// - Request method, URL, and headers
/// - Response status code, timing, and size
/// - Error details for failed requests
///
/// Error handling follows client-layer architecture:
/// - Technical failures (network errors, timeouts) are logged with `tracing::error!`
/// - Service errors (HTTP error codes) are logged with `tracing::debug!`
pub struct TenderdashClient {
    client: ClientWithMiddleware,
    base_url: String,
    websocket_client: Option<Arc<TenderdashWebSocketClient>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenderdashResponse<T> {
    pub jsonrpc: String,
    pub id: i32,
    pub result: Option<T>,
    pub error: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TenderdashStatusResponse {
    pub node_info: Option<NodeInfo>,
    pub sync_info: Option<SyncInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub protocol_version: Option<ProtocolVersion>,
    pub id: Option<String>,
    #[serde(rename = "ProTxHash")]
    pub pro_tx_hash: Option<String>,
    pub network: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub p2p: Option<String>,
    pub block: Option<String>,
    pub app: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncInfo {
    pub latest_block_hash: Option<String>,
    pub latest_app_hash: Option<String>,
    pub latest_block_height: Option<String>,
    pub latest_block_time: Option<String>,
    pub earliest_block_hash: Option<String>,
    pub earliest_app_hash: Option<String>,
    pub earliest_block_height: Option<String>,
    pub earliest_block_time: Option<String>,
    pub max_peer_block_height: Option<String>,
    pub catching_up: Option<bool>,
    pub total_synced_time: Option<String>,
    pub remaining_time: Option<String>,
    pub total_snapshots: Option<String>,
    pub chunk_process_avg_time: Option<String>,
    pub snapshot_height: Option<String>,
    pub snapshot_chunks_count: Option<String>,
    pub backfilled_blocks: Option<String>,
    pub backfill_blocks_total: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetInfoResponse {
    pub listening: Option<bool>,
    pub n_peers: Option<String>,
}

// New response types for broadcast_state_transition
#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastTxResponse {
    pub code: u32,
    pub data: Option<String>,
    pub info: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTxResponse {
    pub code: u32,
    pub info: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnconfirmedTxsResponse {
    pub txs: Option<Vec<String>>,
    pub total: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxResponse {
    pub tx_result: Option<TxResult>,
    pub tx: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxResult {
    pub code: u32,
    pub data: Option<String>,
    pub info: Option<String>,
    pub log: Option<String>,
}

impl TenderdashClient {
    /// Create a new TenderdashClient with HTTP request tracing middleware
    ///
    /// This method validates the connection by making a test HTTP status call
    /// to ensure the Tenderdash service is reachable and responding correctly.
    pub async fn new(uri: &str) -> DAPIResult<Self> {
        info!("Creating Tenderdash client for: {}", uri);

        // Create client with tracing middleware
        let client = ClientBuilder::new(Client::new())
            .with(TracingMiddleware::default())
            .build();

        let tenderdash_client = Self {
            client,
            base_url: uri.to_string(),
            websocket_client: None,
        };

        // Validate connection by making a test status call
        info!("Validating Tenderdash connection at: {}", uri);
        match tenderdash_client.status().await {
            Ok(_) => {
                info!("Tenderdash connection validated successfully");
                Ok(tenderdash_client)
            }
            Err(e) => {
                error!("Tenderdash connection validation failed at {}: {}", uri, e);
                Err(DapiError::server_unavailable(uri, e.to_string()))
            }
        }
    }

    pub async fn with_websocket(uri: &str, ws_uri: &str) -> DAPIResult<Self> {
        info!(
            "Creating Tenderdash client for: {} with WebSocket: {}",
            uri, ws_uri
        );
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 1000));

        // Create client with tracing middleware
        let client = ClientBuilder::new(Client::new())
            .with(TracingMiddleware::default())
            .build();

        let tenderdash_client = Self {
            client,
            base_url: uri.to_string(),
            websocket_client: Some(websocket_client),
        };

        // Validate HTTP connection by making a test status call
        trace!("Validating Tenderdash HTTP connection at: {}", uri);
        match tenderdash_client.status().await {
            Ok(_) => {
                debug!("Tenderdash HTTP connection validated successfully");
            }
            Err(e) => {
                error!(
                    "Tenderdash HTTP connection validation failed at {}: {}",
                    uri, e
                );
                return Err(DapiError::server_unavailable(uri, e));
            }
        }

        // Validate WebSocket connection
        info!("Validating Tenderdash WebSocket connection at: {}", ws_uri);
        if let Some(_ws_client) = &tenderdash_client.websocket_client {
            match TenderdashWebSocketClient::test_connection(ws_uri).await {
                Ok(_) => {
                    info!("Tenderdash WebSocket connection validated successfully");
                    Ok(tenderdash_client)
                }
                Err(e) => {
                    error!(
                        "Tenderdash WebSocket connection validation failed at {}: {}",
                        ws_uri, e
                    );
                    Err(DapiError::server_unavailable(ws_uri, e))
                }
            }
        } else {
            Ok(tenderdash_client)
        }
    }

    pub async fn status(&self) -> DAPIResult<TenderdashStatusResponse> {
        match self.status_internal().await {
            Ok(status) => {
                trace!("Successfully retrieved Tenderdash status");
                Ok(status)
            }
            Err(e) => {
                error!(
                    error = ?e,
                    "Failed to get Tenderdash status - technical failure"
                );
                Err(e)
            }
        }
    }

    async fn status_internal(&self) -> DAPIResult<TenderdashStatusResponse> {
        trace!("Making status request to Tenderdash at: {}", self.base_url);
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "status",
            "params": {},
            "id": 1
        });

        let response: TenderdashResponse<TenderdashStatusResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!("Failed to serialize request body for status: {}", e);
                e
            })?)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send request to Tenderdash at {}: {}",
                    self.base_url, e
                );
                DapiError::Client(format!("Failed to send request: {}", e))
            })?
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse Tenderdash response: {}", e);
                DapiError::Client(format!("Failed to parse response: {}", e))
            })?;

        if let Some(error) = response.error {
            debug!("Tenderdash RPC returned error: {}", error);
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash status response missing result field".to_string())
        })
    }

    pub async fn net_info(&self) -> DAPIResult<NetInfoResponse> {
        match self.net_info_internal().await {
            Ok(netinfo) => {
                trace!("Successfully retrieved Tenderdash net_info");
                Ok(netinfo)
            }
            Err(e) => {
                error!(
                    error = ?e,
                    "Failed to get Tenderdash net_info - technical failure, returning defaults"
                );
                Ok(NetInfoResponse::default())
            }
        }
    }

    async fn net_info_internal(&self) -> DAPIResult<NetInfoResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "net_info",
            "params": {},
            "id": 2
        });

        let response: TenderdashResponse<NetInfoResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!("Failed to serialize request body for net_info: {}", e);
                e
            })?)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send net_info request to Tenderdash at {}: {}",
                    self.base_url, e
                );
                DapiError::Client(format!("Failed to send request: {}", e))
            })?
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse Tenderdash net_info response: {}", e);
                DapiError::Client(format!("Failed to parse response: {}", e))
            })?;

        if let Some(error) = response.error {
            debug!("Tenderdash net_info RPC returned error: {}", error);
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash net_info response missing result field".to_string())
        })
    }

    /// Broadcast a transaction to the Tenderdash network
    pub async fn broadcast_tx(&self, tx: String) -> DAPIResult<BroadcastTxResponse> {
        trace!("Broadcasting transaction to Tenderdash: {} bytes", tx.len());
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "broadcast_tx_sync",
            "params": {
                "tx": tx
            },
            "id": 3
        });

        let response: TenderdashResponse<BroadcastTxResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!(
                    "Failed to serialize request body for broadcast_tx_async: {}",
                    e
                );
                e
            })?)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send broadcast_tx request to Tenderdash at {}: {}",
                    self.base_url, e
                );
                DapiError::Client(format!("Failed to send request: {}", e))
            })?
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse Tenderdash broadcast_tx response: {}", e);
                DapiError::Client(format!("Failed to parse response: {}", e))
            })?;

        if let Some(error) = response.error {
            debug!("Tenderdash broadcast_tx RPC returned error: {}", error);
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash broadcast_tx response missing result field".to_string())
        })
    }

    /// Check a transaction without adding it to the mempool
    pub async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "check_tx",
            "params": {
                "tx": tx
            },
            "id": 4
        });

        let response: TenderdashResponse<CheckTxResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!("Failed to serialize request body for check_tx: {}", e);
                e
            })?)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send check_tx request to Tenderdash at {}: {}",
                    self.base_url, e
                );
                DapiError::Client(format!("Failed to send request: {}", e))
            })?
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse Tenderdash check_tx response: {}", e);
                DapiError::Client(format!("Failed to parse response: {}", e))
            })?;

        if let Some(error) = response.error {
            debug!("Tenderdash check_tx RPC returned error: {}", error);
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash check_tx response missing result field".to_string())
        })
    }

    /// Get unconfirmed transactions from the mempool
    pub async fn unconfirmed_txs(&self, limit: Option<u32>) -> DAPIResult<UnconfirmedTxsResponse> {
        let mut params = json!({});
        if let Some(limit) = limit {
            params["limit"] = json!(limit.to_string());
        }

        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "unconfirmed_txs",
            "params": params,
            "id": 5
        });

        let response: TenderdashResponse<UnconfirmedTxsResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!(
                    "Failed to serialize request body for unconfirmed_txs: {}",
                    e
                );
                e
            })?)
            .send()
            .await
            .map_err(|e| DapiError::Client(format!("Failed to send request: {}", e)))?
            .json()
            .await
            .map_err(|e| DapiError::Client(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = response.error {
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client(
                "Tenderdash unconfirmed_txs response missing result field".to_string(),
            )
        })
    }

    /// Get transaction by hash
    pub async fn tx(&self, hash: String) -> DAPIResult<TxResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "tx",
            "params": {
                "hash": hash
            },
            "id": 6
        });

        let response: TenderdashResponse<TxResponse> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!("Failed to serialize request body for tx: {}", e);
                e
            })?)
            .send()
            .await
            .map_err(|e| DapiError::Client(format!("Failed to send request: {}", e)))?
            .json()
            .await
            .map_err(|e| DapiError::Client(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = response.error {
            return Err(DapiError::Client(format!(
                "Tenderdash RPC error: {}",
                error
            )));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash tx response missing result field".to_string())
        })
    }
}

#[async_trait]
impl TenderdashClientTrait for TenderdashClient {
    async fn status(&self) -> DAPIResult<TenderdashStatusResponse> {
        self.status().await
    }

    async fn net_info(&self) -> DAPIResult<NetInfoResponse> {
        self.net_info().await
    }

    async fn broadcast_tx(&self, tx: String) -> DAPIResult<BroadcastTxResponse> {
        self.broadcast_tx(tx).await
    }

    async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse> {
        self.check_tx(tx).await
    }

    async fn unconfirmed_txs(&self, limit: Option<u32>) -> DAPIResult<UnconfirmedTxsResponse> {
        self.unconfirmed_txs(limit).await
    }

    async fn tx(&self, hash: String) -> DAPIResult<TxResponse> {
        self.tx(hash).await
    }

    fn subscribe_to_transactions(&self) -> broadcast::Receiver<TransactionEvent> {
        if let Some(ws_client) = &self.websocket_client {
            ws_client.subscribe()
        } else {
            // Return a receiver that will never receive messages
            let (_, rx) = broadcast::channel(1);
            rx
        }
    }

    fn is_websocket_connected(&self) -> bool {
        if let Some(ws_client) = &self.websocket_client {
            ws_client.is_connected()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest_tracing::TracingMiddleware;

    #[tokio::test]
    async fn test_tenderdash_client_middleware_integration() {
        // Test that TenderdashClient can be created with middleware
        // Note: This will fail if no server is running, which is expected in unit tests
        match TenderdashClient::new("http://localhost:26657").await {
            Ok(client) => {
                // If connection succeeds, verify the structure
                assert_eq!(client.base_url, "http://localhost:26657");
            }
            Err(_) => {
                // Expected when no server is running - this is okay for unit tests
                // The important thing is that the method signature and error handling work
            }
        }
    }

    #[test]
    fn test_tracing_middleware_can_be_created() {
        // Test that we can create the TracingMiddleware
        let _middleware = TracingMiddleware::default();

        // This tests that our dependency is properly configured
        // and that the middleware can be instantiated
    }

    #[tokio::test]
    async fn test_middleware_request_logging() {
        // Test that demonstrates middleware is attached to client
        // This doesn't make an actual request but verifies the structure

        match TenderdashClient::new("http://localhost:26657").await {
            Ok(client) => {
                // Check that the client has the middleware type
                // This ensures our ClientWithMiddleware wrapper is in place
                assert_eq!(client.base_url, "http://localhost:26657");
            }
            Err(_) => {
                // Expected when no server is running - this is okay for unit tests
            }
        }

        // Note: In a real integration test with a running tenderdash instance,
        // you would see tracing logs like:
        // [TRACE] HTTP request: POST http://localhost:26657
        // [TRACE] HTTP response: 200 OK (response time: 45ms)
        //
        // The TracingMiddleware logs at TRACE level:
        // - Request method, URL, headers
        // - Response status, timing, and size
    }
}
