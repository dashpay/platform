use anyhow::Result;
use async_trait::async_trait;

use crate::clients::{
    tenderdash_client::{
        BroadcastTxResponse, CheckTxResponse, NetInfoResponse, NodeInfo, ProtocolVersion, SyncInfo,
        TenderdashStatusResponse, TxResponse, UnconfirmedTxsResponse,
    },
    traits::TenderdashClientTrait,
};

#[derive(Debug, Clone, Default)]
pub struct MockTenderdashClient;

impl MockTenderdashClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TenderdashClientTrait for MockTenderdashClient {
    async fn status(&self) -> Result<TenderdashStatusResponse> {
        // Return mock data that matches the test expectations
        Ok(TenderdashStatusResponse {
            node_info: Some(NodeInfo {
                protocol_version: Some(ProtocolVersion {
                    p2p: Some("8".to_string()),
                    block: Some("11".to_string()),
                    app: Some("1".to_string()),
                }),
                id: Some("mock_node_id".to_string()),
                pro_tx_hash: Some("mock_pro_tx_hash".to_string()),
                network: Some("testnet".to_string()),
                version: Some("0.11.0".to_string()),
            }),
            sync_info: Some(SyncInfo {
                latest_block_hash: Some("mock_hash".to_string()),
                latest_app_hash: Some("mock_app_hash".to_string()),
                latest_block_height: Some("1000".to_string()),
                latest_block_time: Some("2023-11-01T12:00:00Z".to_string()),
                earliest_block_hash: Some("genesis_hash".to_string()),
                earliest_app_hash: Some("genesis_app_hash".to_string()),
                earliest_block_height: Some("1".to_string()),
                earliest_block_time: Some("2023-01-01T00:00:00Z".to_string()),
                max_peer_block_height: Some("1000".to_string()),
                catching_up: Some(false),
                total_synced_time: Some("0".to_string()),
                remaining_time: Some("0".to_string()),
                total_snapshots: Some("0".to_string()),
                chunk_process_avg_time: Some("0".to_string()),
                snapshot_height: Some("0".to_string()),
                snapshot_chunks_count: Some("0".to_string()),
                backfilled_blocks: Some("0".to_string()),
                backfill_blocks_total: Some("0".to_string()),
            }),
        })
    }

    async fn net_info(&self) -> Result<NetInfoResponse> {
        Ok(NetInfoResponse {
            listening: Some(true),
            n_peers: Some("8".to_string()),
        })
    }

    async fn broadcast_tx(&self, _tx: String) -> Result<BroadcastTxResponse> {
        Ok(BroadcastTxResponse {
            code: 0,
            data: None,
            info: None,
            hash: Some("mock_tx_hash".to_string()),
        })
    }

    async fn check_tx(&self, _tx: String) -> Result<CheckTxResponse> {
        Ok(CheckTxResponse {
            code: 0,
            info: None,
            data: None,
        })
    }

    async fn unconfirmed_txs(&self, _limit: Option<u32>) -> Result<UnconfirmedTxsResponse> {
        Ok(UnconfirmedTxsResponse {
            txs: Some(vec![]),
            total: Some("0".to_string()),
        })
    }

    async fn tx(&self, _hash: String) -> Result<TxResponse> {
        Ok(TxResponse {
            tx_result: None,
            tx: None,
        })
    }

    fn subscribe_to_transactions(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::clients::TransactionEvent> {
        // Return a receiver that will never receive messages for testing
        let (_, rx) = tokio::sync::broadcast::channel(1);
        rx
    }

    fn is_websocket_connected(&self) -> bool {
        true // Mock always connected
    }
}
