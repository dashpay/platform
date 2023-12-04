//! Core wallet using Core GRPC API.
use async_trait::async_trait;
use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::{bls_signatures::PrivateKey, prelude::AssetLockProof};

use crate::{wallet::CoreWallet, Error};

use super::core_client::CoreClient;

/// Core wallet using Core GRPC API.
#[derive(Debug)]
pub struct CoreGrpcWallet {
    pub(crate) core_client: CoreClient,
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
  
}
