use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::error::{DAPIResult, DapiError};
use crate::utils::generate_jsonrpc_id;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace};

/// Default timeout for all Tenderdash HTTP requests
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Connection timeout for establishing HTTP connections; as we do local, 1s is enough
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

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

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: &'static str,
    method: &'static str,
    params: T,
    id: String,
}

impl<T> JsonRpcRequest<T> {
    fn new(method: &'static str, params: T) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
            id: generate_jsonrpc_id(),
        }
    }
}

#[derive(Debug, Serialize, Default)]
struct EmptyParams {}

#[derive(Debug, Serialize)]
struct BroadcastTxParams<'a> {
    tx: &'a str,
}

#[derive(Debug, Serialize)]
struct CheckTxParams<'a> {
    tx: &'a str,
}

#[derive(Debug, Serialize, Default)]
struct UnconfirmedTxsParams {
    #[serde(rename = "page", skip_serializing_if = "Option::is_none")]
    page: Option<String>,
    #[serde(rename = "per_page", skip_serializing_if = "Option::is_none")]
    per_page: Option<String>,
}

#[derive(Debug, Serialize)]
struct TxParams<'a> {
    hash: &'a str,
    prove: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultStatus {
    #[serde(default)]
    pub node_info: Option<NodeInfo>,
    #[serde(default)]
    pub application_info: Option<ApplicationInfo>,
    #[serde(default)]
    pub sync_info: Option<SyncInfo>,
    #[serde(default)]
    pub validator_info: Option<ValidatorInfo>,
    #[serde(default)]
    pub light_client_info: Option<Value>,
}

pub type TenderdashStatusResponse = ResultStatus;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationInfo {
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInfo {
    #[serde(default)]
    pub protocol_version: Option<ProtocolVersion>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub listen_addr: Option<String>,
    #[serde(rename = "ProTxHash", default)]
    pub pro_tx_hash: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub channels: Option<Value>,
    #[serde(default)]
    pub moniker: Option<String>,
    #[serde(default)]
    pub other: Option<NodeInfoOther>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInfoOther {
    #[serde(default)]
    pub tx_index: Option<String>,
    #[serde(default)]
    pub rpc_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolVersion {
    #[serde(default)]
    pub p2p: Option<String>,
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncInfo {
    #[serde(default)]
    pub latest_block_hash: Option<String>,
    #[serde(default)]
    pub latest_app_hash: Option<String>,
    #[serde(default)]
    pub latest_block_height: Option<String>,
    #[serde(default)]
    pub latest_block_time: Option<String>,
    #[serde(default)]
    pub earliest_block_hash: Option<String>,
    #[serde(default)]
    pub earliest_app_hash: Option<String>,
    #[serde(default)]
    pub earliest_block_height: Option<String>,
    #[serde(default)]
    pub earliest_block_time: Option<String>,
    #[serde(default)]
    pub max_peer_block_height: Option<String>,
    #[serde(default)]
    pub catching_up: Option<bool>,
    #[serde(default)]
    pub total_synced_time: Option<String>,
    #[serde(default)]
    pub remaining_time: Option<String>,
    #[serde(default)]
    pub total_snapshots: Option<String>,
    #[serde(default)]
    pub chunk_process_avg_time: Option<String>,
    #[serde(default)]
    pub snapshot_height: Option<String>,
    #[serde(default)]
    pub snapshot_chunks_count: Option<String>,
    #[serde(rename = "backfilled_blocks", default)]
    pub backfilled_blocks: Option<String>,
    #[serde(rename = "backfill_blocks_total", default)]
    pub backfill_blocks_total: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidatorInfo {
    #[serde(default)]
    pub pro_tx_hash: Option<String>,
    #[serde(default)]
    pub voting_power: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultNetInfo {
    #[serde(default)]
    pub listening: Option<bool>,
    #[serde(default)]
    pub listeners: Option<Vec<String>>,
    #[serde(rename = "n_peers", default)]
    pub n_peers: Option<String>,
    #[serde(default)]
    pub peers: Option<Vec<Peer>>,
}

pub type NetInfoResponse = ResultNetInfo;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Peer {
    #[serde(rename = "node_id", default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultBroadcastTx {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub codespace: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub info: Option<String>,
}

pub type BroadcastTxResponse = ResultBroadcastTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultCheckTx {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default)]
    pub info: Option<String>,
    #[serde(default)]
    pub gas_wanted: Option<String>,
    #[serde(default)]
    pub gas_used: Option<String>,
    #[serde(default)]
    pub events: Option<Value>,
    #[serde(default)]
    pub codespace: Option<String>,
}

pub type CheckTxResponse = ResultCheckTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultUnconfirmedTxs {
    #[serde(rename = "n_txs", default)]
    pub count: Option<String>,
    #[serde(default)]
    pub total: Option<String>,
    #[serde(rename = "total_bytes", default)]
    pub total_bytes: Option<String>,
    #[serde(default)]
    pub txs: Option<Vec<String>>,
}

pub type UnconfirmedTxsResponse = ResultUnconfirmedTxs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultTx {
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub height: Option<String>,
    #[serde(default)]
    pub index: Option<u32>,
    #[serde(rename = "tx_result", default)]
    pub tx_result: Option<ExecTxResult>,
    #[serde(default)]
    pub tx: Option<String>,
    #[serde(default)]
    pub proof: Option<Value>,
}

pub type TxResponse = ResultTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecTxResult {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub info: Option<String>,
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default)]
    pub gas_wanted: Option<String>,
    #[serde(default)]
    pub gas_used: Option<String>,
    #[serde(default)]
    pub codespace: Option<String>,
    #[serde(default)]
    pub events: Option<Value>,
}

pub type TxResult = ExecTxResult;

impl TenderdashClient {
    /// Generic POST method for Tenderdash RPC calls
    /// Serializes the request, performs the call, and maps protocol errors to `DapiError`.
    async fn post<T, R>(&self, request: &R) -> DAPIResult<T>
    where
        T: serde::de::DeserializeOwned + Debug,
        R: Serialize + Debug,
    {
        let start = tokio::time::Instant::now();

        let request_value = serde_json::to_value(request).map_err(|e| {
            error!("Failed to serialize Tenderdash request: {}", e);
            DapiError::Client(format!("Failed to serialize request: {}", e))
        })?;

        let response: TenderdashResponse<T> = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(request)
            .timeout(REQUEST_TIMEOUT)
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
            request = ?request_value,
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
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| {
                error!("Failed to build Tenderdash HTTP client: {}", e);
                DapiError::Client(format!("Failed to build Tenderdash HTTP client: {}", e))
            })?;

        let client = ClientBuilder::new(http_client)
            .with(TracingMiddleware::default())
            .build();

        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 256));

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
        let request = JsonRpcRequest::new("status", EmptyParams::default());

        self.post(&request).await
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
        let request = JsonRpcRequest::new("net_info", EmptyParams::default());

        self.post(&request).await
    }

    /// Broadcast a transaction to the Tenderdash network
    pub async fn broadcast_tx(&self, tx: String) -> DAPIResult<BroadcastTxResponse> {
        trace!("Broadcasting transaction to Tenderdash: {} bytes", tx.len());
        let params = BroadcastTxParams { tx: tx.as_str() };
        let request = JsonRpcRequest::new("broadcast_tx_sync", params);

        self.post(&request).await
    }

    /// Check a transaction without adding it to the mempool
    pub async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse> {
        let params = CheckTxParams { tx: tx.as_str() };
        let request = JsonRpcRequest::new("check_tx", params);

        self.post(&request).await
    }

    /// Get unconfirmed transactions from the mempool
    pub async fn unconfirmed_txs(&self, limit: Option<u32>) -> DAPIResult<UnconfirmedTxsResponse> {
        let params = UnconfirmedTxsParams {
            page: None,
            per_page: limit.map(|value| value.to_string()),
        };
        let request = JsonRpcRequest::new("unconfirmed_txs", params);

        self.post(&request).await
    }

    /// Get transaction by hash
    pub async fn tx(&self, hash: String) -> DAPIResult<TxResponse> {
        let params = TxParams {
            hash: hash.as_str(),
            prove: false,
        };
        let request = JsonRpcRequest::new("tx", params);

        self.post(&request).await
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
