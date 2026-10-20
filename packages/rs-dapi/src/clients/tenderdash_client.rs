use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::clients::{CONNECT_TIMEOUT, REQUEST_TIMEOUT};
use crate::error::{DAPIResult, DapiError};
use crate::utils::{
    deserialize_null_as_default, deserialize_string_number_or_null, deserialize_string_or_number,
    generate_jsonrpc_id,
};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace, warn};

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

#[cfg(test)]
impl TenderdashClient {
    /// Construct an unused fixture client without probing HTTP or WebSocket endpoints.
    pub(crate) fn without_test_connection() -> Self {
        Self {
            client: ClientBuilder::new(Client::new()).build(),
            base_url: "http://tenderdash.invalid".to_owned(),
            websocket_client: Arc::new(TenderdashWebSocketClient::new(
                "ws://tenderdash.invalid".to_owned(),
                256,
            )),
        }
    }
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
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
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

/// Consensus versions carried by a block header; `app` is the Platform protocol version.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsensusVersion {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub app: u64,
}

/// The block header fields rs-dapi reads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockHeader {
    #[serde(default)]
    pub version: ConsensusVersion,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub height: u64,
    /// RFC 3339 block time.
    #[serde(default)]
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockData {
    /// Base64-encoded transactions, in block order. Tenderdash omits or nulls an empty list.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub txs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    #[serde(default)]
    pub header: BlockHeader,
    #[serde(default)]
    pub data: BlockData,
}

/// Result of the `block` RPC.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultBlock {
    pub block: Option<Block>,
}

/// Result of the `block_results` RPC: one execution result per transaction, in block order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultBlockResults {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub txs_results: Vec<ExecTxResult>,
}

/// Header and transaction count of one block, as listed by the `blockchain` RPC.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockMeta {
    #[serde(default)]
    pub header: BlockHeader,
    /// `-1` for a header-only meta (state sync), whose transactions the store does not keep.
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub num_txs: i64,
}

/// Result of the `blockchain` RPC: block metas in descending height order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultBlockchainInfo {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub block_metas: Vec<BlockMeta>,
}

#[derive(Debug, Serialize)]
struct HeightParams {
    height: String,
}

#[derive(Debug, Serialize)]
struct BlockchainParams {
    #[serde(rename = "minHeight")]
    min_height: String,
    #[serde(rename = "maxHeight")]
    max_height: String,
}

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
    /// This method attempts to validate both HTTP and WebSocket connectivity before returning.
    /// If either check fails, the error is logged and the client is still returned so callers can
    /// operate in a degraded state while health checks surface the issue.
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

        if let Err(e) = tenderdash_client.validate_connection().await {
            warn!(
                error = %e,
                "Tenderdash HTTP connection validation failed; continuing with degraded health"
            );
        }

        if let Err(e) = TenderdashWebSocketClient::test_connection(ws_uri).await {
            warn!(
                error = %e,
                "Tenderdash WebSocket connection validation failed; continuing with retries"
            );
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
            Err(e) => Err(DapiError::server_unavailable(
                self.base_url.clone(),
                e.to_string(),
            )),
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

    /// Get the committed block at `height`.
    pub async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        let params = HeightParams {
            height: height.to_string(),
        };
        let request = JsonRpcRequest::new("block", params);

        self.post(&request).await
    }

    /// Get the execution results of the committed block at `height`.
    pub async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
        let params = HeightParams {
            height: height.to_string(),
        };
        let request = JsonRpcRequest::new("block_results", params);

        self.post(&request).await
    }

    /// List the block metas of heights `min_height..=max_height`, highest first.
    ///
    /// Tenderdash returns at most 20 metas per call, keeping the highest heights of the range.
    pub async fn blockchain(
        &self,
        min_height: u64,
        max_height: u64,
    ) -> DAPIResult<ResultBlockchainInfo> {
        let params = BlockchainParams {
            min_height: min_height.to_string(),
            max_height: max_height.to_string(),
        };
        let request = JsonRpcRequest::new("blockchain", params);

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A `block` reply carrying two transactions, as Tenderdash writes it.
    const BLOCK: &str = r#"{
    "jsonrpc": "2.0",
    "id": 7,
    "result": {
        "block_id": { "hash": "AE99F1ED1207DE4FF456E7FAD694FC160018AD43BF30D40EA1A1D8345D0AD397", "parts": { "total": 1, "hash": "85B14CF1354D01BDB4C402A0A927A485ACDFB8F8D77D200A17C2998D8EB57F89" }, "state_id": "A4A41A407933CE446FB6C943F3E745EE627C6A2275B49B6159A99DA717F254E8" },
        "block": {
            "header": {
                "version": { "block": 14, "app": 9 },
                "chain_id": "dash-1",
                "height": "2183417",
                "core_chain_locked_height": 2366051,
                "time": "2026-10-14T12:34:56.789123456Z",
                "last_block_id": { "hash": "E93CC963D65EE813FE85602AF8FBD1E934801B1F111CE7E5723B5676543F7154", "parts": { "total": 1, "hash": "55F8307240CB7E8BBE67D130E36A7DEFA91818F8F29C624694A08AED97392335" }, "state_id": "CA8763979E21D85FB14C29D9F76E296369DA06C09C12A5BCDEA92B10F371E5D0" },
                "last_commit_hash": "60002321EE6C544A2BA8A9E833E4CD4CA19C4A66B33E6E602BF23529FB16982B",
                "data_hash": "924B724AD61D6CDD4FB9ABB09C18BCDB483B398662CDFF75B0D99747492F2C42",
                "validators_hash": "1E1DF1DEBD06FBD0E8F012F0DC53EB480A09DA832F053E05879E92266A03FCB8",
                "next_validators_hash": "839331694682A13B221ADF02DABE29E79FB323191B235E7E34A9EE94459FE3C0",
                "consensus_hash": "FAE4D43D86F8F823022B63D60F4E523FF7C2470E6FFD3D0EBA0AF7CBD49AFE27",
                "next_consensus_hash": "189C8551B971FC9DC87EB0BB5C1BE0E32CD72A58C35CEF6A58DAA6DC6D636BE7",
                "app_hash": "15AACD0D7986995F962F2E2277295917A0F8B4EA1B962DDA61418B09C9CB04CD",
                "results_hash": "C12E92106D929E7735AA5E110ABF86C08370FC6BAE4C5DCE897431F383A0BBB9",
                "evidence_hash": "648CAFC7BB4D13900B647C303B0CB7D2DA27B6494791C182160C2E35DA9806CD",
                "proposer_pro_tx_hash": "68DE7CA2A3A322C38EE71E068CD6946CAB61AB7452A36118B51044B65A09882B",
                "proposed_protocol_version": 9
            },
            "data": { "txs": ["AgEAAQ==", "AwIBAA=="] },
            "core_chain_lock": {
                "core_block_height": "2366051",
                "core_block_hash": "q2bJ8hT0r6Yb3Nw4K1mS9pV7cX5eZ2aD0fG8hJ4kL6M=",
                "signature": "lVqzJ2pXH4k8n0wC5yT3eR7uI1oP9aS6dF2gH8jK4lZ1xC3vB5nM7qW0eR2tY4uI"
            },
            "evidence": [],
            "last_commit": {
                "height": 2183416,
                "round": 0,
                "block_id": { "hash": "0942EB4DC2774B6A9CB048913C5F9A95FE358075FA76C334BF8EAF7EEB6F48F9", "parts": { "total": 1, "hash": "DC88F1E1D6DEF7C0977411F778163A68606FA4A47EFA5C52A133357F86E33422" }, "state_id": "490B76A8DA6B74C8A3ED3AFBD414F7C718B0360EB0824C50D373A562C9871C44" },
                "quorum_hash": "924B631589E17B6547A9E4B70D49145CCF4192E78E981EA6F32F8FB0D08C85AA",
                "threshold_block_signature": "jJ8uK2nX5vB7mQ1wE3rT6yU9iO0pA4sD8fG2hJ5kL7zX1cV3bN6mQ9wE2rT5yU8i",
                "threshold_vote_extensions": null
            }
        }
    }
}"#;

    /// A `block` reply for a block without transactions: Tenderdash writes `null`.
    const EMPTY_BLOCK: &str = r#"{
    "jsonrpc": "2.0",
    "id": 7,
    "result": {
        "block_id": { "hash": "B1AACF56290EBEC597D08BD03A2A9C908BFD87D2D86EDD2F685E8542ED3F1B2F", "parts": { "total": 1, "hash": "CE1A8F909D565970CE39878EB1CC5145156E7F8C0685490BFE04232C55B45677" }, "state_id": "2A5B91511EBCEB2DDB66C4DD122346D8AFA02888C16B33531FE655F2CB52FA59" },
        "block": {
            "header": {
                "version": { "block": 14, "app": 9 },
                "chain_id": "dash-1",
                "height": "2183418",
                "core_chain_locked_height": 2366051,
                "time": "2026-10-14T12:34:56.789123456Z",
                "last_block_id": { "hash": "490530A13A9563004684D5F437081B7619F131EA15B99963232C5B34882919A3", "parts": { "total": 1, "hash": "0A678A315A235585B2D11281332CE012A50AB39CC0B380D15E3F2FD2047D8703" }, "state_id": "6B2096EE82A7361F6056C3F66FEE2B4E9A8C9FCF21BF5FEBE3A084CAE404523E" },
                "last_commit_hash": "121979739C8038C43F0C83A8E8C5D2BE8BFE090DE09AA0F266E98B90BBDC5955",
                "data_hash": "E57BB21BA6BD98A5EF5013765A873F22F1839A25FFE191A73E034F1E1D2BFF82",
                "validators_hash": "FB2C779344A463270DB412018723F4C41578EDD6E15E2C449FF2218B94CEB3DC",
                "next_validators_hash": "876EE0A3ADECF961992B32CE1B1D742C5AFA5609967BF9ABCBB2110DAF46BDCC",
                "consensus_hash": "C665370CA388A34ADF30B97EA2652B813B84595C2B645477B84FF6334EBE2CB5",
                "next_consensus_hash": "65282302C41B7D4B8B4D302F3FB228EDBC07E5D2A455275F349B45CC5CFA9C07",
                "app_hash": "8838A1F8AA1BE44C4CAA52198A3BDA1239A8319913D68B16F2FE758D156D2B93",
                "results_hash": "3115896D7FA65C221ADA18FD6EFF058DBBE26F20F3C5814FB858D5237277F982",
                "evidence_hash": "6C188299A2204FD772CE3C43FB512EE2EA6F6A3738D4A1BFE084E2285B9EC275",
                "proposer_pro_tx_hash": "436A17C69D04FC75BA60D5827E1FC151A4F8C44F00978B91366F555A77B878EA",
                "proposed_protocol_version": 9
            },
            "data": { "txs": null },
            "core_chain_lock": {
                "core_block_height": "2366051",
                "core_block_hash": "q2bJ8hT0r6Yb3Nw4K1mS9pV7cX5eZ2aD0fG8hJ4kL6M=",
                "signature": "lVqzJ2pXH4k8n0wC5yT3eR7uI1oP9aS6dF2gH8jK4lZ1xC3vB5nM7qW0eR2tY4uI"
            },
            "evidence": [],
            "last_commit": {
                "height": 2183417,
                "round": 0,
                "block_id": { "hash": "9D73DCD82B2B80D622E2355976CAAB36BD292B9830CE11E6596F551CB4427832", "parts": { "total": 1, "hash": "D0753BE31CF5849959E8015B745908AC6DE92C5BE138EC3284E8311AB41F373A" }, "state_id": "BD48DE3214787275A5C64D1E727B86C9D1CAACC9D5780C4AE029E80B219BE1CB" },
                "quorum_hash": "B12054DA58A68590040D936DEB02F98047AD8074DEF28E2EDB1FB6D0D02C594B",
                "threshold_block_signature": "jJ8uK2nX5vB7mQ1wE3rT6yU9iO0pA4sD8fG2hJ5kL7zX1cV3bN6mQ9wE2rT5yU8i",
                "threshold_vote_extensions": null
            }
        }
    }
}"#;

    /// `block_results` for `BLOCK`: one success (`code` omitted when 0) and one failure, numbers
    /// as Tenderdash writes them.
    const BLOCK_RESULTS: &str = r#"{
    "jsonrpc": "2.0",
    "id": 8,
    "result": {
        "height": "2183417",
        "txs_results": [
            {
                "gas_used": 1340,
                "events": [{ "type": "app", "attributes": [{ "key": "k", "value": "v", "index": true }] }]
            },
            {
                "code": 10046,
                "info": "AiQBYW5vdGhlciBpZGVudGl0eQ==",
                "log": "identity not found",
                "codespace": "platform",
                "gas_used": 210
            }
        ],
        "total_gas_used": "1550",
        "finalize_block_events": null,
        "validator_set_updates": null,
        "consensus_param_updates": null
    }
}"#;

    /// `block_results` with every number as a string, as Tenderdash's RPC reference shows them.
    const STRING_BLOCK_RESULTS: &str = r#"{
    "jsonrpc": "2.0",
    "id": "9",
    "result": {
        "height": "12",
        "txs_results": [
            { "code": "0", "data": "", "log": "", "info": "", "gas_used": "100", "events": null, "codespace": "" },
            { "code": "2", "data": "", "log": "not enough gas", "info": "", "gas_used": "100", "events": null, "codespace": "ibc" }
        ],
        "total_gas_used": "200"
    }
}"#;

    /// `block_results` for `EMPTY_BLOCK`.
    const EMPTY_BLOCK_RESULTS: &str = r#"{
    "jsonrpc": "2.0",
    "id": 10,
    "result": {
        "height": "2183418",
        "txs_results": null,
        "total_gas_used": "0",
        "finalize_block_events": null,
        "validator_set_updates": null,
        "consensus_param_updates": null
    }
}"#;

    /// A `blockchain` page, highest first: a block with transactions, an empty one and a
    /// header-only meta from state sync.
    const BLOCKCHAIN: &str = r#"{
    "jsonrpc": "2.0",
    "id": 11,
    "result": {
        "last_height": "2183420",
        "block_metas": [
            {
                "block_id": { "hash": "6A6A978BDFC07C51B03BD2BE39DB1510C4760CF98FACAD651A55A2666C40E5E9", "parts": { "total": 1, "hash": "F48E03ABDD36B9971AF8EE438FF46606853910AD7B81E409E4B441538E52E742" }, "state_id": "66B028B7F01695EAD949529BBCC1B719C3CAF99EB6D611C608019A9297E3413E" },
                "block_size": "1873",
                "header": {
                "version": { "block": 14, "app": 9 },
                "chain_id": "dash-1",
                "height": "2183417",
                "core_chain_locked_height": 2366051,
                "time": "2026-10-14T12:35:00.1Z",
                "last_block_id": { "hash": "F4C4EEBF89EF9DC0A160E5016345AB1FA761523E5ED7519FA84876DCF62AAAA9", "parts": { "total": 1, "hash": "E5A8549155AFFFBC4A47D393C267EBE61A965637A15451A84B0537EEFB37F3C4" }, "state_id": "F586032C796D45AC313393C7DDC5E55FB0A232AD4A2B1A795E984AB2AA2E211C" },
                "last_commit_hash": "8353A58FF1DCCE44F5FC1138DE59D3A304807DF6300CBF556706A9798D9F3699",
                "data_hash": "E2FE7506EEBFC35EF8A56EB3BDDE53A67F0487006687A477EE85075D535FE8EA",
                "validators_hash": "1DFA39D519C23334547E07A3DA289B772C0C48DEB60959922C2722AA469BBAA8",
                "next_validators_hash": "FB8DAA60E9C5DACB4D9465AA70906D67052CF1D98BEBDDD824C8A85D3D73404D",
                "consensus_hash": "F40FD770933471923363349A46B14A7F228E152CE0BC22B9C5302145BCF4EC19",
                "next_consensus_hash": "F8ADF09B9A1DFCFB1B81EBF5EED8FC3F7A28414CAA8004A0BE45909FC930208F",
                "app_hash": "B3D912C0EF9943254066A75047022BDFD97958463203A9E71B451AABCC8E9897",
                "results_hash": "B98834098AE87EFED4889BB483B97DB5A683C3121351957ECA433250B7DE84D3",
                "evidence_hash": "6CC32D0725EFE10D1ABD55B2DDF07506EDB4984933448208EC873F833DBB3D62",
                "proposer_pro_tx_hash": "665E866A6770156712BA22CB1745D99691CE5A9E88295F63D8BAA0FEC35F527D",
                "proposed_protocol_version": 9
            },
                "num_txs": "2",
                "round": "0",
                "has_core_chain_lock": false
            },
            {
                "block_id": { "hash": "6F4242D5DDC3A98E6E9DA469304E1384606F30370F4158FD0962861FCAA00559", "parts": { "total": 1, "hash": "D4DF7A3764E9192DA1E243B8143C7950F86FAA0EFBB686F46424E41AD35F0C79" }, "state_id": "2C633A90FDF7563B54880C27B0532CBFD26641D5EDF7D7B7D10E8932167CA2C2" },
                "block_size": "1204",
                "header": {
                "version": { "block": 14, "app": 9 },
                "chain_id": "dash-1",
                "height": "2183416",
                "core_chain_locked_height": 2366051,
                "time": "2026-10-14T12:35:00.1Z",
                "last_block_id": { "hash": "DA337ACA6498EEDB066815C5A02D09ED1E966680ABA6AF372865895E43E92F50", "parts": { "total": 1, "hash": "78FECB5C34FB6BA3D3BB8B1FE3A59EF1AEE4C931CD4991CD95E55B7D862755A3" }, "state_id": "AA5F33FD02EA2DC39576A5B91B5E813A656D1741B9AC33C545AEAA9C0E1A8A47" },
                "last_commit_hash": "E746F29F4F609A588D0EA657FD5F2DFD1CEB8384BE3555EF7F264441C83B9DB6",
                "data_hash": "A679063C72A386A05A9CF7E197AB74AAB65A3E2A2DAD155D0673BC9628E227E3",
                "validators_hash": "755BA8F755E1D7F9FE09942C1EEB52BA061EC24E762B979E603DC7B3AE945019",
                "next_validators_hash": "AF649784CA4ACA169714C13BF05856C9B38C8B5E2A4D3B37DFDB1EE683EFA6E9",
                "consensus_hash": "A9B33092AA3ACBB4B3E4A4E48620787B6A0AF9DEA6562027A6BE6DAB3390F39C",
                "next_consensus_hash": "7F9F3A4524E819C5787F21C951909DA3E006DDF790177C1B42635880D1B9A306",
                "app_hash": "4774559F1C17A95FC7C2A73E378A58EFFE1A48A9B84870DB8C9CFC6A429CC8B7",
                "results_hash": "636BC84E0A889DDD5E2225C57A7DF9CCF79BA56A83CDBC38E2CCAA18BD02BF5B",
                "evidence_hash": "70BFB71C0F50D97F1E7DBEFF66BDCAFAEBDA10BCFE58E6A4294BB61A0C12D31B",
                "proposer_pro_tx_hash": "D73FFBE98E9AB583576CCB12754AF87204C6373D2E8C6610D9DA8771992B6E93",
                "proposed_protocol_version": 9
            },
                "num_txs": "0",
                "round": "0",
                "has_core_chain_lock": false
            },
            {
                "block_id": { "hash": "D8AB3A5E7398567AB9B20C25B58135CBC91897397E3B61A007FC5AAD2A12AD3F", "parts": { "total": 1, "hash": "3C380C48733E013A3D51F8CED81C426AABB0BBB21EC5EEB084F2A3C1426A8157" }, "state_id": "FC0C12437A63B13D819389F03494D0CA954B8C574AE70211555099C66CB49FE2" },
                "block_size": "0",
                "header": {
                "version": { "block": 14, "app": 8 },
                "chain_id": "dash-1",
                "height": "2183415",
                "core_chain_locked_height": 2366051,
                "time": "2026-10-14T12:35:00.1Z",
                "last_block_id": { "hash": "86051B0796460D8ED173A12CB3764C80DEB6AB6189A0A4980CE5A75DCA2AD268", "parts": { "total": 1, "hash": "BEBDC802C5198A8E8A41F1D2ADE3777348275B8E2B02365D1CC7CD84097416C3" }, "state_id": "4A87DC0D3E584D83426F50157B0204E278B0DBD515652C009F35FF781536474D" },
                "last_commit_hash": "ABEC8FFE122979839BD1D00463F2201A2924FA0FDB7FF8349C42E38D926CE2E4",
                "data_hash": "3AB41CC57679955AC35991E3D026E919ED9CF2C2D32699DCBB09FA316A692D8F",
                "validators_hash": "8F841CE6E938F0E01651AB79D5706AB745D3469ECCC4BE976259DC461712D76A",
                "next_validators_hash": "C05175515E7AB320577EDB84BE80B54FDA27843BF18A2518647273E7CDBF43DC",
                "consensus_hash": "672647F3AD2ACEF22E516715C8BDE0158A238B649D2AC3CCC328A9F24A7B9D4C",
                "next_consensus_hash": "25B6241A300633C06EB45AD2E2C6B716EF645C8513A33E69E3BEC6F84FA7E1CD",
                "app_hash": "10BE26E40ACEC2AB646645753B7C7C3EAAFFE67199E22575B401AE8B6A624089",
                "results_hash": "07E6AD19CE639C632F9C3DB615DB3FB01306A9D302898A76A86DD031D1BF88E4",
                "evidence_hash": "27FE79F8654B87E50B350FD2B9314CFD5D17CA309E991F0F928A70F10FFDAC3B",
                "proposer_pro_tx_hash": "F224EC2CAE37CEF20B4788E024382F0C140060FA0E8439B553D5992DC7D75E5A",
                "proposed_protocol_version": 8
            },
                "num_txs": "-1",
                "round": "0",
                "has_core_chain_lock": false
            }
        ]
    }
}"#;

    fn result<T: serde::de::DeserializeOwned>(json: &str) -> T {
        let response: TenderdashResponse<T> = serde_json::from_str(json).expect("decodes");
        assert!(response.error.is_none());
        response.result.expect("a result")
    }

    #[test]
    fn should_decode_a_block_with_transactions() {
        let block = result::<ResultBlock>(BLOCK).block.expect("a block");
        assert_eq!(block.header.height, 2183417);
        assert_eq!(block.header.version.app, 9);
        assert_eq!(block.header.time, "2026-10-14T12:34:56.789123456Z");
        assert_eq!(block.data.txs, vec!["AgEAAQ==", "AwIBAA=="]);
    }

    #[test]
    fn should_decode_an_empty_block_and_its_null_results() {
        let block = result::<ResultBlock>(EMPTY_BLOCK).block.expect("a block");
        assert_eq!(block.header.height, 2183418);
        assert!(block.data.txs.is_empty());
        let results = result::<ResultBlockResults>(EMPTY_BLOCK_RESULTS);
        assert!(results.txs_results.is_empty());
    }

    #[test]
    fn should_decode_successful_and_failed_transaction_results() {
        for json in [BLOCK_RESULTS, STRING_BLOCK_RESULTS] {
            let results = result::<ResultBlockResults>(json).txs_results;
            let codes: Vec<u32> = results.iter().map(|result| result.code).collect();
            assert_eq!(codes.len(), 2, "{json}");
            assert_eq!(codes[0], 0, "{json}");
            assert_ne!(codes[1], 0, "{json}");
            assert!(results[1].gas_used > 0, "{json}");
            assert!(!results[1].log.is_empty(), "{json}");
        }
        let failed = &result::<ResultBlockResults>(BLOCK_RESULTS).txs_results[1];
        assert_eq!(failed.code, 10046);
        assert_eq!(failed.codespace, "platform");
    }

    #[test]
    fn should_decode_a_blockchain_page_of_block_metas() {
        let metas = result::<ResultBlockchainInfo>(BLOCKCHAIN).block_metas;
        let read: Vec<(u64, i64, u64)> = metas
            .iter()
            .map(|meta| (meta.header.height, meta.num_txs, meta.header.version.app))
            .collect();
        assert_eq!(
            read,
            vec![(2183417, 2, 9), (2183416, 0, 9), (2183415, -1, 8)]
        );
    }
}
