use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::error::{DAPIResult, DapiError};
use crate::utils::generate_jsonrpc_id;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
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
    workers: crate::sync::Workers,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenderdashResponse<T> {
    pub jsonrpc: String,
    pub id: Value,
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
    pub code: i64,
    pub data: Option<String>,
    pub info: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckTxResponse {
    pub code: i64,
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
    pub code: i64,
    pub data: Option<String>,
    pub info: Option<String>,
    pub log: Option<String>,
}

impl TenderdashClient {
    /// Generic POST method for Tenderdash RPC calls
    /// Serializes the request, performs the call, and maps protocol errors to `DapiError`.
    async fn post<T>(&self, request_body: serde_json::Value) -> DAPIResult<T>
    where
        T: serde::de::DeserializeOwned + Debug,
    {
        let start = tokio::time::Instant::now();
        let response: TenderdashResponse<T> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request_body).map_err(|e| {
                error!("Failed to serialize request body: {}", e);
                DapiError::Client(format!("Failed to serialize request body: {}", e))
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

        tracing::trace!(
            elapsed = ?start.elapsed(),
            request = ?request_body,
            response = ?response,
            "tenderdash_client request executed");

        if let Some(error) = response.error {
            debug!("Tenderdash RPC returned error: {}", error);
            return Err(DapiError::from_tenderdash_error(error));
        }

        response.result.ok_or_else(|| {
            DapiError::Client("Tenderdash response missing result field".to_string())
        })
    }

    /// Create a new TenderdashClient with HTTP and WebSocket support.
    ///
    /// This method validates both HTTP and WebSocket connectivity before returning.
    pub async fn new(uri: &str, ws_uri: &str) -> DAPIResult<Self> {
        trace!(
            uri = %uri,
            ws_uri = %ws_uri,
            "Creating Tenderdash client with WebSocket support"
        );

        let http_client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                error!("Failed to build Tenderdash HTTP client: {}", e);
                DapiError::Client(format!("Failed to build Tenderdash HTTP client: {}", e))
            })?;

        let client = ClientBuilder::new(http_client)
            .with(TracingMiddleware::default())
            .build();
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 1000));

        let tenderdash_client = Self {
            client,
            base_url: uri.to_string(),
            websocket_client: Some(websocket_client.clone()),
            workers: Default::default(),
        };

        // Validate HTTP connection
        tenderdash_client.validate_connection().await?;

        // Validate WebSocket connection
        match TenderdashWebSocketClient::test_connection(ws_uri).await {
            Ok(_) => {
                info!("Tenderdash WebSocket connection validated successfully");
            }
            Err(e) => {
                error!(
                    "Tenderdash WebSocket connection validation failed at {}: {}",
                    ws_uri, e
                );
                return Err(DapiError::server_unavailable(ws_uri, e));
            }
        };

        // Start listening for WebSocket events
        tenderdash_client
            .workers
            .spawn(async move { websocket_client.connect_and_listen().await });

        Ok(tenderdash_client)
    }

    /// Perform a lightweight status call to ensure the Tenderdash HTTP endpoint is reachable.
    async fn validate_connection(&self) -> DAPIResult<()> {
        // Validate HTTP connection by making a test status call
        trace!(
            "Validating Tenderdash HTTP connection at: {}",
            self.base_url
        );
        match self.status().await {
            Ok(_) => {
                info!("Tenderdash HTTP connection validated successfully");
                Ok(())
            }
            Err(e) => {
                error!(
                    "Tenderdash HTTP connection validation failed at {}: {}",
                    self.base_url, e
                );
                Err(DapiError::server_unavailable(
                    self.base_url.clone(),
                    e.to_string(),
                ))
            }
        }
    }

    /// Query Tenderdash for node and sync status information via JSON-RPC `status`.
    pub async fn status(&self) -> DAPIResult<TenderdashStatusResponse> {
        trace!("Making status request to Tenderdash at: {}", self.base_url);
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "status",
            "params": {},
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
    }

    /// Retrieve network peer statistics, falling back to defaults on transport errors.
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

    /// Internal helper that performs the `net_info` RPC call without error masking.
    async fn net_info_internal(&self) -> DAPIResult<NetInfoResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "net_info",
            "params": {},
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
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
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
    }

    /// Check a transaction without adding it to the mempool
    pub async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "check_tx",
            "params": {
                "tx": tx
            },
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
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
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
    }

    /// Get transaction by hash
    pub async fn tx(&self, hash: String) -> DAPIResult<TxResponse> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "tx",
            "params": {
                "hash": hash
            },
            "id": generate_jsonrpc_id()
        });

        self.post(request_body).await
    }
    /// Subscribe to streaming Tenderdash transaction events if WebSocket is available.
    pub fn subscribe_to_transactions(&self) -> broadcast::Receiver<TransactionEvent> {
        if let Some(ws_client) = &self.websocket_client {
            ws_client.subscribe()
        } else {
            // Return a receiver that will never receive messages
            let (_, rx) = broadcast::channel(1);
            rx
        }
    }
    /// Subscribe to block events from Tenderdash via WebSocket.
    pub fn subscribe_to_blocks(&self) -> broadcast::Receiver<BlockEvent> {
        if let Some(ws_client) = &self.websocket_client {
            ws_client.subscribe_blocks()
        } else {
            // Return a receiver that will never receive messages
            let (_, rx) = broadcast::channel(1);
            rx
        }
    }

    /// Return whether the internal WebSocket client currently maintains a connection.
    pub fn is_websocket_connected(&self) -> bool {
        if let Some(ws_client) = &self.websocket_client {
            ws_client.is_connected()
        } else {
            false
        }
    }
}
