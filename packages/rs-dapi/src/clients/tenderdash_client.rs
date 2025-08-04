use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace};

use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use super::traits::TenderdashClientTrait;
use crate::error::{DAPIResult, DapiError};

#[derive(Debug, Clone)]
pub struct TenderdashClient {
    client: Client,
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
    pub fn new(uri: &str) -> Self {
        info!("Creating Tenderdash client for: {}", uri);
        Self {
            client: Client::new(),
            base_url: uri.to_string(),
            websocket_client: None,
        }
    }

    pub fn with_websocket(uri: &str, ws_uri: &str) -> Self {
        info!(
            "Creating Tenderdash client for: {} with WebSocket: {}",
            uri, ws_uri
        );
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 1000));

        Self {
            client: Client::new(),
            base_url: uri.to_string(),
            websocket_client: Some(websocket_client),
        }
    }

    pub async fn status(&self) -> DAPIResult<TenderdashStatusResponse> {
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
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to Tenderdash at {}: {}", self.base_url, e);
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
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "net_info",
            "params": {},
            "id": 2
        });

        let response: TenderdashResponse<NetInfoResponse> = self
            .client
            .post(&self.base_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send net_info request to Tenderdash at {}: {}", self.base_url, e);
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
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send broadcast_tx request to Tenderdash at {}: {}", self.base_url, e);
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
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send check_tx request to Tenderdash at {}: {}", self.base_url, e);
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
            .json(&request_body)
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
            .json(&request_body)
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
