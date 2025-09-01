use dashcore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;
use tracing::trace;

use crate::{DAPIResult, DapiError};
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
            .map_err(|e| DapiError::client(format!("Join error: {}", e)))
            .and_then(|res| res.map_err(|e| DapiError::client(e.to_string())))?;

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
        let info = tokio::task::spawn_blocking(move || client.get_raw_transaction_info(&txid, None))
            .await
            .map_err(|e| DapiError::client(format!("Join error: {}", e)))
            .and_then(|res| res.map_err(|e| DapiError::client(e.to_string())))?;
        Ok(info)
    }

    pub async fn send_raw_transaction(&self, raw: &[u8]) -> DAPIResult<String> {
        trace!("Core RPC: send_raw_transaction");
        let raw_vec = raw.to_vec();
        let client = self.client.clone();
        let txid = tokio::task::spawn_blocking(move || client.send_raw_transaction(&raw_vec))
            .await
            .map_err(|e| DapiError::client(format!("Join error: {}", e)))
            .and_then(|res| res.map_err(|e| DapiError::client(e.to_string())))?;
        Ok(txid.to_string())
    }
}
