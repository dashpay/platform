//! Core wallet using Core GRPC API.
use async_trait::async_trait;
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
}
