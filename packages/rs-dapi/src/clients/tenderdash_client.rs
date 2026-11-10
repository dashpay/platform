use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use crate::clients::tenderdash_websocket::BlockEvent;
use crate::clients::{
    BLOCK_QUERY_KEEPALIVE, BLOCK_QUERY_KEEPALIVE_RETRIES, CONNECT_TIMEOUT, REQUEST_TIMEOUT,
};
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
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::RangeInclusive;
use std::sync::Arc;
use tokio::sync::{OnceCell, broadcast};
use tracing::{debug, error, info, trace, warn};

/// Tenderdash heights are signed 64-bit integers.
pub const MAX_TENDERDASH_HEIGHT: u64 = i64::MAX as u64;

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
    /// Sends what a block query asks of Tenderdash, with no deadline: see
    /// [`block_query_http_client`].
    block_query_client: ClientWithMiddleware,
    base_url: String,
    websocket_client: Arc<TenderdashWebSocketClient>,
    /// The network's validator quorum type, read from Tenderdash the first time it is needed.
    validator_quorum_type: OnceCell<u32>,
}

#[cfg(test)]
impl TenderdashClient {
    /// Construct an unused fixture client without probing HTTP or WebSocket endpoints.
    pub(crate) fn without_test_connection() -> Self {
        Self {
            client: http_client().expect("Tenderdash HTTP client"),
            block_query_client: block_query_http_client().expect("Tenderdash block query client"),
            base_url: "http://tenderdash.invalid".to_owned(),
            websocket_client: Arc::new(TenderdashWebSocketClient::new(
                "ws://tenderdash.invalid".to_owned(),
                256,
            )),
            validator_quorum_type: OnceCell::new(),
        }
    }

    /// Construct a fixture client whose HTTP requests go to `base_url`.
    pub(crate) fn with_test_base_url(base_url: String) -> Self {
        Self {
            base_url,
            ..Self::without_test_connection()
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

#[derive(Debug, Serialize)]
struct BlockHeightParams {
    height: u64,
}

#[derive(Debug, Serialize)]
struct ValidatorsParams {
    per_page: u32,
}

/// The answers for one height of [`TenderdashClient::block_batch`]. Each call fails on its
/// own, so a height past the tip does not take the other heights down with it.
#[derive(Debug)]
pub struct BlockBatchEntry {
    pub block: DAPIResult<ResultBlock>,
    pub results: DAPIResult<ResultBlockResults>,
    /// `None` when the commit was not asked for.
    pub commit: Option<DAPIResult<ResultCommit>>,
}

/// The JSON-RPC ids of one height's calls in a batch.
#[derive(Debug)]
struct BlockBatchIds {
    block: String,
    results: String,
    commit: Option<String>,
}

/// Tenderdash answers a batch with an array, but a failure of the whole batch with one object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchResponseBody {
    Batch(Vec<TenderdashResponse<Value>>),
    Single(Box<TenderdashResponse<Value>>),
}

/// The calls for `heights`, in one list, and which ids belong to which height.
fn block_batch_requests(
    heights: RangeInclusive<u64>,
    with_commit: bool,
) -> (Vec<JsonRpcRequest<BlockHeightParams>>, Vec<BlockBatchIds>) {
    let mut requests = Vec::new();
    let mut ids = Vec::new();
    for height in heights {
        let mut call = |method| {
            let request = JsonRpcRequest::new(method, BlockHeightParams { height });
            let id = request.id.clone();
            requests.push(request);
            id
        };
        ids.push(BlockBatchIds {
            block: call("block"),
            results: call("block_results"),
            commit: with_commit.then(|| call("commit")),
        });
    }
    (requests, ids)
}

/// Index a batch's answers by id, the order of an array being no part of JSON-RPC.
fn parse_batch_response(body: &str) -> DAPIResult<HashMap<String, TenderdashResponse<Value>>> {
    let parsed: BatchResponseBody = serde_json::from_str(body)
        .map_err(|e| DapiError::Client(format!("Failed to parse response: {}", e)))?;
    match parsed {
        BatchResponseBody::Batch(responses) => Ok(responses
            .into_iter()
            .filter_map(|response| Some((response.id.as_str()?.to_owned(), response)))
            .collect()),
        BatchResponseBody::Single(response) => match response.error {
            Some(error) => Err(DapiError::from_tenderdash_error(error)),
            None => Err(DapiError::Client(
                "Tenderdash answered a batch with a single result".to_string(),
            )),
        },
    }
}

/// Take the answer of one call out of a parsed batch.
fn take_batch_result<T: serde::de::DeserializeOwned>(
    responses: &mut HashMap<String, TenderdashResponse<Value>>,
    id: &str,
) -> DAPIResult<T> {
    let response = responses.remove(id).ok_or_else(|| {
        DapiError::Client("Tenderdash batch response is missing a call".to_string())
    })?;
    if let Some(error) = response.error {
        return Err(DapiError::from_tenderdash_error(error));
    }
    let result = response
        .result
        .ok_or_else(|| DapiError::Client("Tenderdash response missing result field".to_string()))?;
    serde_json::from_value(result)
        .map_err(|e| DapiError::Client(format!("Failed to parse response: {}", e)))
}

/// Read the answers of a whole batch out of its body, one entry for each height asked.
fn read_block_batch(body: &str, ids: Vec<BlockBatchIds>) -> DAPIResult<Vec<BlockBatchEntry>> {
    let mut responses = parse_batch_response(body)?;

    Ok(ids
        .into_iter()
        .map(|ids| BlockBatchEntry {
            block: take_batch_result(&mut responses, &ids.block),
            results: take_batch_result(&mut responses, &ids.results),
            commit: ids.commit.map(|id| take_batch_result(&mut responses, &id)),
        })
        .collect())
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

/// The part of Tenderdash's `block` result that DAPI reads.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResultBlock {
    #[serde(default)]
    pub block_id: BlockId,
    /// `None` when the node has no block at the height.
    #[serde(default)]
    pub block: Option<Block>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockId {
    /// The block hash, hex encoded.
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub parts: PartSetHeader,
    /// Hex encoded.
    #[serde(default)]
    pub state_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PartSetHeader {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub total: u32,
    /// Hex encoded.
    #[serde(default)]
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConsensusVersion {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub block: u64,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub app: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Block {
    #[serde(default)]
    pub header: BlockHeader,
    #[serde(default)]
    pub data: BlockData,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockHeader {
    #[serde(default)]
    pub version: ConsensusVersion,
    #[serde(default)]
    pub chain_id: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub height: u64,
    /// The block time, as an RFC 3339 timestamp.
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub core_chain_locked_height: u32,
    #[serde(default)]
    pub last_block_id: BlockId,
    /// Hex encoded, like the other hashes below.
    #[serde(default)]
    pub last_commit_hash: String,
    #[serde(default)]
    pub data_hash: String,
    #[serde(default)]
    pub validators_hash: String,
    #[serde(default)]
    pub next_validators_hash: String,
    #[serde(default)]
    pub consensus_hash: String,
    #[serde(default)]
    pub next_consensus_hash: String,
    #[serde(default)]
    pub app_hash: String,
    #[serde(default)]
    pub results_hash: String,
    #[serde(default)]
    pub evidence_hash: String,
    #[serde(default)]
    pub proposer_pro_tx_hash: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub proposed_protocol_version: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockData {
    /// The block's transactions, base64 encoded.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub txs: Vec<String>,
}

/// The part of Tenderdash's `block_results` result that DAPI reads.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResultBlockResults {
    /// One result for each transaction of the block, in block order.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub txs_results: Vec<BlockTxResult>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockTxResult {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub code: u32,
    /// Base64 encoded.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub data: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub gas_used: i64,
}

/// Tenderdash's `commit` result: a block's header and the quorum's signature of it.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResultCommit {
    #[serde(default)]
    pub signed_header: SignedHeader,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SignedHeader {
    #[serde(default)]
    pub header: BlockHeader,
    #[serde(default)]
    pub commit: Commit,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Commit {
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub round: u32,
    #[serde(default)]
    pub block_id: BlockId,
    /// Hex encoded.
    #[serde(default)]
    pub quorum_hash: String,
    /// Base64 encoded.
    #[serde(default)]
    pub threshold_block_signature: String,
}

/// The part of Tenderdash's `validators` result that DAPI reads.
#[derive(Debug, Clone, Deserialize)]
struct ResultValidators {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    quorum_type: u32,
}

/// The HTTP client for ordinary Tenderdash calls, each of which fails after `REQUEST_TIMEOUT`.
fn http_client() -> DAPIResult<ClientWithMiddleware> {
    with_tracing(
        Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT),
    )
}

/// The HTTP client for what a block query asks of Tenderdash, which has no deadline.
///
/// Tenderdash does not stop work whose caller has gone away, so giving up on a call would
/// only hide that it is still running there. This client waits until Tenderdash answers or
/// closes the connection, which is when the work is over, so a caller can count a call
/// against a limit for as long as Tenderdash is busy with it. Keepalive probes close the
/// connection to a peer that died without saying so.
fn block_query_http_client() -> DAPIResult<ClientWithMiddleware> {
    with_tracing(
        Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .tcp_keepalive(BLOCK_QUERY_KEEPALIVE)
            .tcp_keepalive_interval(BLOCK_QUERY_KEEPALIVE)
            .tcp_keepalive_retries(BLOCK_QUERY_KEEPALIVE_RETRIES),
    )
}

fn with_tracing(builder: reqwest::ClientBuilder) -> DAPIResult<ClientWithMiddleware> {
    let http_client = builder.build().map_err(|e| {
        error!("Failed to build Tenderdash HTTP client: {}", e);
        DapiError::Client(format!("Failed to build Tenderdash HTTP client: {}", e))
    })?;

    Ok(ClientBuilder::new(http_client)
        .with(TracingMiddleware::default())
        .build())
}

impl TenderdashClient {
    /// Generic POST method for Tenderdash RPC calls
    /// Serializes the request, performs the call, and maps protocol errors to `DapiError`.
    async fn post<T, R>(&self, request: &R) -> DAPIResult<T>
    where
        T: serde::de::DeserializeOwned + Debug,
        R: Serialize + Debug,
    {
        self.post_with(&self.client, request).await
    }

    /// `post` for a block query: the call has no deadline, see [`block_query_http_client`].
    async fn post_for_block_query<T, R>(&self, request: &R) -> DAPIResult<T>
    where
        T: serde::de::DeserializeOwned + Debug,
        R: Serialize + Debug,
    {
        self.post_with(&self.block_query_client, request).await
    }

    async fn post_with<T, R>(&self, client: &ClientWithMiddleware, request: &R) -> DAPIResult<T>
    where
        T: serde::de::DeserializeOwned + Debug,
        R: Serialize + Debug,
    {
        let start = tokio::time::Instant::now();

        let response_body = self.post_for_body(client, request).await?;

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

    /// Send a request or a batch of them, returning the body Tenderdash answered with.
    async fn post_for_body<R>(
        &self,
        client: &ClientWithMiddleware,
        request: &R,
    ) -> DAPIResult<String>
    where
        R: Serialize + Debug,
    {
        client
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

        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 256));

        let tenderdash_client = Self {
            client: http_client()?,
            block_query_client: block_query_http_client()?,
            base_url: uri.to_string(),
            websocket_client: websocket_client.clone(),
            validator_quorum_type: OnceCell::new(),
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

    /// Get the block at a height
    pub async fn block(&self, height: u64) -> DAPIResult<ResultBlock> {
        let request = JsonRpcRequest::new("block", BlockHeightParams { height });

        self.post_for_block_query(&request).await
    }

    /// Get the execution results of the block at a height
    pub async fn block_results(&self, height: u64) -> DAPIResult<ResultBlockResults> {
        let request = JsonRpcRequest::new("block_results", BlockHeightParams { height });

        self.post_for_block_query(&request).await
    }

    /// Read the block, results and optionally commit of each height in one HTTP request.
    ///
    /// Tenderdash takes one height per call, and a JSON-RPC batch carries them all at once.
    /// The whole range is read before anything is returned, so a caller that keeps only part
    /// of it has still paid for all of it.
    pub async fn block_batch(
        &self,
        heights: RangeInclusive<u64>,
        with_commit: bool,
    ) -> DAPIResult<Vec<BlockBatchEntry>> {
        let (requests, ids) = block_batch_requests(heights, with_commit);
        let body = self
            .post_for_body(&self.block_query_client, &requests)
            .await?;

        // A batch can carry ten full blocks. Reading that much JSON on an async thread would
        // hold up the other requests it serves, so it runs on the blocking pool.
        tokio::task::spawn_blocking(move || read_block_batch(&body, ids)).await?
    }

    /// Get the header of the block at a height and the quorum's signature of it
    pub async fn commit(&self, height: u64) -> DAPIResult<ResultCommit> {
        let request = JsonRpcRequest::new("commit", BlockHeightParams { height });

        self.post_for_block_query(&request).await
    }

    /// Get the network's validator quorum type, which Tenderdash fixes at genesis.
    pub async fn validator_quorum_type(&self) -> DAPIResult<u32> {
        self.validator_quorum_type
            .get_or_try_init(|| async {
                let request = JsonRpcRequest::new("validators", ValidatorsParams { per_page: 1 });
                let result: ResultValidators = self.post_for_block_query(&request).await?;
                Ok(result.quorum_type)
            })
            .await
            .copied()
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
    use serde_json::json;

    fn answer(id: &str, result: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "result": result })
    }

    fn failure(id: &str, data: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32603, "message": "Internal error", "data": data },
        })
    }

    #[test]
    fn should_ask_for_each_height_in_one_list() {
        let (requests, ids) = block_batch_requests(7..=8, false);

        let calls = requests
            .iter()
            .map(|request| (request.method, request.params.height))
            .collect::<Vec<_>>();
        assert_eq!(
            calls,
            [
                ("block", 7),
                ("block_results", 7),
                ("block", 8),
                ("block_results", 8),
            ]
        );
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|ids| ids.commit.is_none()));
    }

    #[test]
    fn should_ask_for_the_commit_only_when_told_to() {
        let (requests, ids) = block_batch_requests(7..=7, true);

        let methods = requests
            .iter()
            .map(|request| request.method)
            .collect::<Vec<_>>();
        assert_eq!(methods, ["block", "block_results", "commit"]);
        assert_eq!(ids[0].commit.as_deref(), Some(requests[2].id.as_str()));
    }

    #[test]
    fn should_serialize_a_batch_as_a_json_array() {
        let (requests, _) = block_batch_requests(7..=7, false);

        let body = serde_json::to_value(&requests).unwrap();

        assert_eq!(body[0]["method"], "block");
        assert_eq!(body[0]["params"]["height"], 7);
        assert_eq!(body[1]["method"], "block_results");
    }

    #[test]
    fn should_match_batch_answers_by_id_in_any_order() {
        let body = json!([
            answer("b", json!({ "txs_results": [{ "code": 5 }] })),
            answer("a", json!({ "block_id": { "hash": "AB" } })),
        ])
        .to_string();

        let mut responses = parse_batch_response(&body).unwrap();

        let block: ResultBlock = take_batch_result(&mut responses, "a").unwrap();
        let results: ResultBlockResults = take_batch_result(&mut responses, "b").unwrap();
        assert_eq!(block.block_id.hash, "AB");
        assert_eq!(results.txs_results[0].code, 5);
    }

    #[test]
    fn should_keep_a_failed_call_from_failing_the_others() {
        let body = json!([
            failure("a", "could not find results for height #6"),
            answer("b", json!({ "txs_results": [] })),
        ])
        .to_string();

        let mut responses = parse_batch_response(&body).unwrap();

        let failed: DAPIResult<ResultBlockResults> = take_batch_result(&mut responses, "a");
        assert!(failed.unwrap_err().is_missing_tenderdash_height());
        let answered: ResultBlockResults = take_batch_result(&mut responses, "b").unwrap();
        assert!(answered.txs_results.is_empty());
    }

    #[test]
    fn should_read_a_batch_into_one_entry_for_each_height() {
        let (_, ids) = block_batch_requests(7..=8, true);
        let body = json!([
            answer(
                ids[1].commit.as_deref().unwrap(),
                json!({ "signed_header": { "header": { "height": "8" } } })
            ),
            failure(&ids[1].results, "could not find results for height #8"),
            answer(&ids[1].block, json!({ "block_id": { "hash": "08" } })),
            answer(&ids[0].results, json!({ "txs_results": [{ "code": 5 }] })),
            answer(&ids[0].block, json!({ "block_id": { "hash": "07" } })),
        ])
        .to_string();

        let mut entries = read_block_batch(&body, ids).unwrap().into_iter();

        let first = entries.next().unwrap();
        assert_eq!(first.block.unwrap().block_id.hash, "07");
        assert_eq!(first.results.unwrap().txs_results[0].code, 5);
        assert!(matches!(first.commit, Some(Err(DapiError::Client(_)))));
        let second = entries.next().unwrap();
        assert_eq!(second.block.unwrap().block_id.hash, "08");
        assert!(second.results.unwrap_err().is_missing_tenderdash_height());
        let commit = second.commit.unwrap().unwrap();
        assert_eq!(commit.signed_header.header.height, 8);
        assert!(entries.next().is_none());
    }

    #[test]
    fn should_report_a_call_tenderdash_did_not_answer() {
        let mut responses =
            parse_batch_response(&json!([answer("a", json!({}))]).to_string()).unwrap();

        let missing: DAPIResult<ResultBlock> = take_batch_result(&mut responses, "b");

        assert!(matches!(missing, Err(DapiError::Client(_))));
    }

    #[test]
    fn should_fail_when_tenderdash_rejects_the_whole_batch() {
        let body = failure("", "batch contains 101 requests, but the maximum is 100").to_string();

        assert!(matches!(
            parse_batch_response(&body),
            Err(DapiError::TenderdashClientError(_))
        ));
    }

    #[test]
    fn should_fail_on_a_body_that_is_not_a_batch_response() {
        assert!(matches!(
            parse_batch_response("not json"),
            Err(DapiError::Client(_))
        ));
        assert!(matches!(
            parse_batch_response(&answer("a", json!({})).to_string()),
            Err(DapiError::Client(_))
        ));
    }

    #[test]
    fn should_read_the_validator_quorum_type() {
        let result: ResultValidators = serde_json::from_value(json!({
            "block_height": "7",
            "validators": [],
            "count": "0",
            "total": "0",
            "quorum_type": 6,
            "quorum_hash": "AB",
        }))
        .unwrap();

        assert_eq!(result.quorum_type, 6);
    }
}
