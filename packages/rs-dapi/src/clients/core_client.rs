use crate::error::MapToDapiResult;
use crate::{DAPIResult, DapiError};
use dashcore_rpc::{jsonrpc, Auth, Client, RpcApi};
use std::sync::Arc;
use tracing::trace;
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct CoreClient {
    client: Arc<Client>,
}

impl CoreClient {
    pub fn new(url: String, user: String, pass: Zeroizing<String>) -> DAPIResult<Self> {
        let client = Client::new(&url, Auth::UserPass(user, pass.to_string()))
            .map_err(|e| DapiError::client(format!("Failed to create Core RPC client: {}", e)))?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    pub async fn get_block_count(&self) -> DAPIResult<u32> {
        trace!("Core RPC: get_block_count");
        let client = self.client.clone();
        let height = tokio::task::spawn_blocking(move || client.get_block_count())
            .await
            .to_dapi_result()?;

        Ok(height as u32)
    }

    pub async fn get_transaction_info(
        &self,
        txid_hex: &str,
    ) -> DAPIResult<dashcore_rpc::json::GetRawTransactionResult> {
        use std::str::FromStr;
        trace!("Core RPC: get_raw_transaction_info");
        let txid = dashcore_rpc::dashcore::Txid::from_str(txid_hex)
            .map_err(|e| DapiError::client(format!("Invalid txid: {}", e)))?;
        let client = self.client.clone();
        let info =
            tokio::task::spawn_blocking(move || client.get_raw_transaction_info(&txid, None))
                .await
                .to_dapi_result()?;
        Ok(info)
    }

    pub async fn send_raw_transaction(&self, raw: &[u8]) -> DAPIResult<String> {
        trace!("Core RPC: send_raw_transaction");
        let raw_vec = raw.to_vec();
        let client = self.client.clone();
        let txid = tokio::task::spawn_blocking(move || client.send_raw_transaction(&raw_vec))
            .await
            .to_dapi_result()?;
        Ok(txid.to_string())
    }

    pub async fn get_block_hash(
        &self,
        height: u32,
    ) -> DAPIResult<dashcore_rpc::dashcore::BlockHash> {
        trace!("Core RPC: get_block_hash");
        let client = self.client.clone();
        let hash = tokio::task::spawn_blocking(move || client.get_block_hash(height))
            .await
            .to_dapi_result()?;
        Ok(hash)
    }

    pub async fn get_block_bytes_by_hash(
        &self,
        hash: dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<Vec<u8>> {
        use dashcore_rpc::dashcore::consensus::encode::serialize;
        trace!("Core RPC: get_block (bytes)");
        let client = self.client.clone();
        let block = tokio::task::spawn_blocking(move || client.get_block(&hash))
            .await
            .to_dapi_result()?;
        Ok(serialize(&block))
    }

    pub async fn get_block_bytes_by_hash_hex(&self, hash_hex: &str) -> DAPIResult<Vec<u8>> {
        use std::str::FromStr;
        let hash = dashcore_rpc::dashcore::BlockHash::from_str(hash_hex)
            .map_err(|e| DapiError::client(format!("Invalid block hash: {}", e)))?;
        self.get_block_bytes_by_hash(hash).await
    }

    pub async fn get_block_header_info(
        &self,
        hash: &dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<dashcore_rpc::json::GetBlockHeaderResult> {
        trace!("Core RPC: get_block_header_info");
        let hash = *hash;
        let client = self.client.clone();
        let header = tokio::task::spawn_blocking(move || client.get_block_header_info(&hash))
            .await
            .to_dapi_result()?;
        Ok(header)
    }

    pub async fn get_best_chain_lock(
        &self,
    ) -> DAPIResult<Option<dashcore_rpc::dashcore::ChainLock>> {
        trace!("Core RPC: get_best_chain_lock");
        let client = self.client.clone();
        match tokio::task::spawn_blocking(move || client.get_best_chain_lock()).await {
            Ok(Ok(chain_lock)) => Ok(Some(chain_lock)),
            Ok(Err(dashcore_rpc::Error::JsonRpc(jsonrpc::Error::Rpc(rpc))))
                if rpc.code == -32603 =>
            {
                // Dash Core returns -32603 when no chain lock is available yet
                Ok(None)
            }
            Ok(Err(e)) => Err(DapiError::from(e)),
            Err(e) => Err(DapiError::from(e)),
        }
    }

    pub async fn mn_list_diff(
        &self,
        base_block: &dashcore_rpc::dashcore::BlockHash,
        block: &dashcore_rpc::dashcore::BlockHash,
    ) -> DAPIResult<serde_json::Value> {
        trace!("Core RPC: getmnlistdiff");
        let base_hex = base_block.to_string();
        let block_hex = block.to_string();
        let client = self.client.clone();

        let diff = tokio::task::spawn_blocking(move || {
            let params = [
                serde_json::Value::String(base_hex),
                serde_json::Value::String(block_hex),
            ];
            client.call("getmnlistdiff", &params)
        })
        .await
        .to_dapi_result()?;
        Ok(diff)
    }

    pub async fn get_blockchain_info(
        &self,
    ) -> DAPIResult<dashcore_rpc::json::GetBlockchainInfoResult> {
        trace!("Core RPC: get_blockchain_info");
        let client = self.client.clone();
        let info = tokio::task::spawn_blocking(move || client.get_blockchain_info())
            .await
            .to_dapi_result()?;
        Ok(info)
    }

    pub async fn get_network_info(&self) -> DAPIResult<dashcore_rpc::json::GetNetworkInfoResult> {
        trace!("Core RPC: get_network_info");
        let client = self.client.clone();
        let info = tokio::task::spawn_blocking(move || client.get_network_info())
            .await
            .to_dapi_result()?;
        Ok(info)
    }

    pub async fn estimate_smart_fee_btc_per_kb(&self, blocks: u16) -> DAPIResult<Option<f64>> {
        trace!("Core RPC: estimatesmartfee");
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || client.estimate_smart_fee(blocks, None))
            .await
            .to_dapi_result()?;
        Ok(result.fee_rate.map(|a| a.to_dash()))
    }

    pub async fn get_masternode_status(&self) -> DAPIResult<dashcore_rpc::json::MasternodeStatus> {
        trace!("Core RPC: masternode status");
        let client = self.client.clone();
        let st = tokio::task::spawn_blocking(move || client.get_masternode_status())
            .await
            .to_dapi_result()?;
        Ok(st)
    }

    pub async fn mnsync_status(&self) -> DAPIResult<dashcore_rpc::json::MnSyncStatus> {
        trace!("Core RPC: mnsync status");
        let client = self.client.clone();
        let st = tokio::task::spawn_blocking(move || client.mnsync_status())
            .await
            .to_dapi_result()?;
        Ok(st)
    }

    pub async fn get_masternode_pos_penalty(
        &self,
        pro_tx_hash_hex: &str,
    ) -> DAPIResult<Option<u32>> {
        use std::collections::HashMap;
        trace!("Core RPC: masternode list (filter)");
        let filter = pro_tx_hash_hex.to_string();
        let client = self.client.clone();
        let map: HashMap<String, dashcore_rpc::json::Masternode> =
            tokio::task::spawn_blocking(move || {
                client.get_masternode_list(Some("json"), Some(&filter))
            })
            .await
            .to_dapi_result()?;

        // Find the entry matching the filter
        if let Some((_k, v)) = map.into_iter().next() {
            return Ok(Some(v.pos_penalty_score));
        }
        Ok(None)
    }
}
