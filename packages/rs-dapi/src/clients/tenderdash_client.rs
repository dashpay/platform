use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::clients::{CONNECT_TIMEOUT, REQUEST_TIMEOUT};
use crate::error::{DAPIResult, DapiError};
use crate::utils::{
    deserialize_string_number_or_null, deserialize_string_or_number, generate_jsonrpc_id,
};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
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
    websocket_client: Arc<TenderdashWebSocketClient>,
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

#[derive(Debug, Serialize)]
struct UnconfirmedTxParams<'a> {
    hash: &'a str,
}

#[derive(Debug, Serialize)]
struct TxParams<'a> {
    hash: &'a str,
    prove: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultStatus {
    #[serde(default)]
    pub node_info: NodeInfo,
    #[serde(default)]
    pub application_info: ApplicationInfo,
    #[serde(default)]
    pub sync_info: SyncInfo,
    #[serde(default)]
    pub validator_info: ValidatorInfo,
    #[serde(default)]
    pub light_client_info: Value,
}

pub type TenderdashStatusResponse = ResultStatus;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationInfo {
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInfo {
    #[serde(default)]
    pub protocol_version: ProtocolVersion,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub listen_addr: String,
    #[serde(rename = "ProTxHash", default)]
    pub pro_tx_hash: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub channels: Vec<u64>,
    #[serde(default)]
    pub moniker: String,
    #[serde(default)]
    pub other: NodeInfoOther,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInfoOther {
    #[serde(default)]
    pub tx_index: String,
    #[serde(default)]
    pub rpc_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolVersion {
    #[serde(default)]
    pub p2p: String,
    #[serde(default)]
    pub block: String,
    #[serde(default)]
    pub app: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncInfo {
    #[serde(default)]
    pub latest_block_hash: String,
    #[serde(default)]
    pub latest_app_hash: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub latest_block_height: i64,
    #[serde(default)]
    pub latest_block_time: String,
    #[serde(default)]
    pub earliest_block_hash: String,
    #[serde(default)]
    pub earliest_app_hash: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub earliest_block_height: i64,
    #[serde(default)]
    pub earliest_block_time: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub max_peer_block_height: i64,
    #[serde(default)]
    pub catching_up: bool,
    #[serde(default)]
    pub total_synced_time: String,
    #[serde(default)]
    pub remaining_time: String,
    #[serde(default)]
    pub total_snapshots: String,
    #[serde(default)]
    pub chunk_process_avg_time: String,
    #[serde(default)]
    pub snapshot_height: String,
    #[serde(default)]
    pub snapshot_chunks_count: String,
    #[serde(rename = "backfilled_blocks", default)]
    pub backfilled_blocks: String,
    #[serde(rename = "backfill_blocks_total", default)]
    pub backfill_blocks_total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidatorInfo {
    #[serde(default)]
    pub pro_tx_hash: String,
    #[serde(default)]
    pub voting_power: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultNetInfo {
    #[serde(default)]
    pub listening: bool,
    #[serde(default)]
    pub listeners: Vec<String>,
    #[serde(
        rename = "n_peers",
        default,
        deserialize_with = "deserialize_string_or_number"
    )]
    pub n_peers: u32,
    #[serde(default)]
    pub peers: Vec<Peer>,
}

pub type NetInfoResponse = ResultNetInfo;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Peer {
    #[serde(rename = "node_id", default)]
    pub node_id: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultBroadcastTx {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub codespace: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub info: String,
}

pub type BroadcastTxResponse = ResultBroadcastTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultCheckTx {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub info: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub gas_wanted: i64,
    #[serde(default)]
    pub codespace: String,
    #[serde(default)]
    pub sender: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub priority: i64,
}

pub type CheckTxResponse = ResultCheckTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultUnconfirmedTx {
    #[serde(default)]
    pub tx: String,
}

pub type UnconfirmedTxResponse = ResultUnconfirmedTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultTx {
    #[serde(default)]
    pub hash: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub height: i64,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub index: u32,
    #[serde(rename = "tx_result", default)]
    pub tx_result: ExecTxResult,
    #[serde(default)]
    pub tx: String,
    #[serde(default)]
    pub proof: Value,
}

pub type TxResponse = ResultTx;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecTxResult {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub code: u32,
    #[serde(default, deserialize_with = "deserialize_string_number_or_null")]
    pub data: String,
    #[serde(default, deserialize_with = "deserialize_string_number_or_null")]
    pub info: String,
    #[serde(default, deserialize_with = "deserialize_string_number_or_null")]
    pub log: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub gas_used: i64,
    #[serde(default, deserialize_with = "deserialize_string_number_or_null")]
    pub codespace: String,
    #[serde(default)]
    pub events: Vec<Value>,
}

impl ExecTxResult {
    /// Check if all fields are at their default values. Useful to detect absent results.
    pub fn is_empty(&self) -> bool {
        self.code == 0
            && self.data.is_empty()
            && self.info.is_empty()
            && self.log.is_empty()
            && self.gas_used == 0
            && self.codespace.is_empty()
            && self.events.is_empty()
    }
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

        let response_body = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "Failed to send request to Tenderdash at {}: {}",
                    self.base_url, e
                );
                DapiError::Client(format!("Failed to send request: {}", e))
            })?
            .text()
            .await
            .map_err(|e| {
                error!("Failed to read Tenderdash response body: {}", e);
                DapiError::Client(format!("Failed to read response body: {}", e))
            })?;

        let response: TenderdashResponse<T> =
            serde_json::from_str(&response_body).map_err(|e| {
                error!(
                    "Failed to parse Tenderdash response: {}; full body: {}",
                    e, response_body
                );
                DapiError::Client(format!("Failed to parse response: {}", e))
            })?;

        tracing::trace!(
            elapsed = ?start.elapsed(),
            request = ?request,
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
    /// If either check fails, client construction fails.
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
            websocket_client: websocket_client.clone(),
        };

        tenderdash_client.validate_connection().await?;

        if let Err(e) = TenderdashWebSocketClient::test_connection(ws_uri).await {
            error!(
                error = %e,
                "Tenderdash WebSocket connection validation failed"
            );
            return Err(e);
        }

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

    /// Get a single unconfirmed transaction by its hash
    pub async fn unconfirmed_tx(&self, hash: &str) -> DAPIResult<UnconfirmedTxResponse> {
        let params = UnconfirmedTxParams { hash };
        let request = JsonRpcRequest::new("unconfirmed_tx", params);

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
        self.websocket_client.subscribe()
    }
    /// Subscribe to block events from Tenderdash via WebSocket.
    pub fn subscribe_to_blocks(&self) -> broadcast::Receiver<BlockEvent> {
        self.websocket_client.subscribe_blocks()
    }

    /// Return whether the internal WebSocket client currently maintains a connection.
    pub fn is_websocket_connected(&self) -> bool {
        self.websocket_client.is_connected()
    }

    /// Return a clone of the underlying WebSocket client to allow shared listeners.
    pub fn websocket_client(&self) -> Arc<TenderdashWebSocketClient> {
        self.websocket_client.clone()
    }
}
