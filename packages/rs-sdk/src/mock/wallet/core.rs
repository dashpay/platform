//! Core wallet using Core GRPC API.
use async_trait::async_trait;
use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::{bls_signatures::PrivateKey, prelude::AssetLockProof};

use crate::{wallet::CoreWallet, Error};

use super::core_client::CoreClient;

/// Core wallet using Core GRPC API.
pub struct CoreGrpcWallet {
    core_client: CoreClient,
}

impl CoreGrpcWallet {
    /// Create new Core wallet using Core GRPC API.
    pub fn new(ip: &str, port: u16, user: &str, password: &str) -> Result<Self, Error> {
        let core_client = CoreClient::new(ip, port, user, password)?;
        Ok(Self { core_client })
    }
}

#[async_trait]
impl CoreWallet for CoreGrpcWallet {
    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
        todo!("Not yet implemented")
    }

    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
        let unspent: Vec<dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry> =
            self.core_client.list_unspent(sum)?;
        Ok(unspent)
    }

    async fn core_balance(&self) -> Result<u64, Error> {
        Ok(self.core_client.get_balance()?.to_sat())
    }
}
