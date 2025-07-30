use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::traits::TenderdashClientTrait;

#[derive(Debug, Clone)]
pub struct TenderdashClient {
    client: Client,
    base_url: String,
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

impl TenderdashClient {
    pub fn new(uri: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: uri.to_string(),
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
}

#[async_trait]
impl TenderdashClientTrait for TenderdashClient {
    async fn status(&self) -> Result<TenderdashStatusResponse> {
        self.status().await
    }

    async fn net_info(&self) -> Result<NetInfoResponse> {
        self.net_info().await
    }
}
