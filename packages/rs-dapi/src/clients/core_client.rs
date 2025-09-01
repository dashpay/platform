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
}
