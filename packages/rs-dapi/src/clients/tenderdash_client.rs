use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent};
use super::traits::TenderdashClientTrait;

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
        Self {
            client: Client::new(),
            base_url: uri.to_string(),
            websocket_client: None,
        }
    }

    pub fn with_websocket(uri: &str, ws_uri: &str) -> Self {
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(ws_uri.to_string(), 1000));

        Self {
            client: Client::new(),
            base_url: uri.to_string(),
            websocket_client: Some(websocket_client),
        }
    }

    pub async fn status(&self) -> Result<TenderdashStatusResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Tenderdash status response missing result field"))
    }

    pub async fn net_info(&self) -> Result<NetInfoResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Tenderdash net_info response missing result field"))
    }

    /// Broadcast a transaction to the Tenderdash network
    pub async fn broadcast_tx(&self, tx: String) -> Result<BroadcastTxResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Tenderdash broadcast_tx response missing result field"))
    }

    /// Check a transaction without adding it to the mempool
    pub async fn check_tx(&self, tx: String) -> Result<CheckTxResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Tenderdash check_tx response missing result field"))
    }

    /// Get unconfirmed transactions from the mempool
    pub async fn unconfirmed_txs(&self, limit: Option<u32>) -> Result<UnconfirmedTxsResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response.result.ok_or_else(|| {
            anyhow::anyhow!("Tenderdash unconfirmed_txs response missing result field")
        })
    }

    /// Get transaction by hash
    pub async fn tx(&self, hash: String) -> Result<TxResponse> {
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
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tenderdash RPC error: {}", error));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Tenderdash tx response missing result field"))
    }
}

#[async_trait]
impl TenderdashClientTrait for TenderdashClient {
    async fn status(&self) -> Result<TenderdashStatusResponse> {
        self.status().await
    }

    async fn net_info(&self) -> Result<NetInfoResponse> {
        self.net_info().await
    }

    async fn broadcast_tx(&self, tx: String) -> Result<BroadcastTxResponse> {
        self.broadcast_tx(tx).await
    }

    async fn check_tx(&self, tx: String) -> Result<CheckTxResponse> {
        self.check_tx(tx).await
    }

    async fn unconfirmed_txs(&self, limit: Option<u32>) -> Result<UnconfirmedTxsResponse> {
        self.unconfirmed_txs(limit).await
    }

    async fn tx(&self, hash: String) -> Result<TxResponse> {
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
