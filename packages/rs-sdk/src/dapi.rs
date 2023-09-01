//! Dash API implementation

use crate::error::Error;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::dashcore::{hashes::Hash, QuorumHash};
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_proof_verifier::QuorumInfoProvider;
use rs_dapi_client::{AddressList, DapiClient, RequestSettings};
use tokio::sync::{RwLock, RwLockWriteGuard};

#[async_trait::async_trait]
pub trait DashAPI: Send + Sync {
    async fn core_client(&self) -> RwLockWriteGuard<crate::core::CoreClient>;
    async fn platform_client(&self) -> RwLockWriteGuard<crate::platform::PlatformClient>;
    fn quorum_info_provider<'a>(&'a self) -> Result<Box<dyn QuorumInfoProvider + 'a>, Error>;
}

pub struct Api {
    dapi: tokio::sync::RwLock<crate::platform::PlatformClient>,
    // TODO: Replace with rs-sdk implementation when it's ready
    core: tokio::sync::RwLock<crate::core::CoreClient>,
}

impl Api {
    pub fn new(
        address: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
        platform_port: u16,
    ) -> Result<Self, Error> {
        let mut address_list = AddressList::new();
        let platform_addr = rs_dapi_client::Uri::from_maybe_shared(format!(
            "http://{}:{}",
            address, platform_port
        ))?;
        address_list.add_uri(platform_addr);
        let dapi = DapiClient::new(address_list, RequestSettings::default());

        let core_addr = format!("http://{}:{}", address, core_port);
        let core =
            DefaultCoreRPC::open(&core_addr, core_user.to_string(), core_password.to_string())?;

        Ok(Self {
            dapi: RwLock::new(dapi),
            core: RwLock::new(Box::new(core)),
        })
    }

    async fn get_quorum_key(&self, quorum_hash: &[u8], quorum_type: u32) -> Result<Vec<u8>, Error> {
        let quorum_hash = QuorumHash::from_slice(quorum_hash).map_err(|e| {
            Error::Proof(drive_proof_verifier::Error::InvalidQuorum {
                error: e.to_string(),
            })
        })?;

        let core = self.core.write().await;
        let quorum_info =
            core.get_quorum_info(QuorumType::from(quorum_type), &quorum_hash, None)?;

        Ok(quorum_info.quorum_public_key)
    }
}

#[async_trait::async_trait]
impl DashAPI for Api {
    async fn core_client(&self) -> RwLockWriteGuard<crate::core::CoreClient> {
        self.core.write().await
    }
    async fn platform_client(&self) -> RwLockWriteGuard<crate::platform::PlatformClient> {
        self.dapi.write().await
    }
    fn quorum_info_provider<'a>(&'a self) -> Result<Box<dyn QuorumInfoProvider + 'a>, Error> {
        Ok(Box::new(self))
    }
}

impl QuorumInfoProvider for &Api {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: Vec<u8>,
        _core_chain_locked_height: u32,
    ) -> Result<Vec<u8>, drive_proof_verifier::Error> {
        let key_fut = self.get_quorum_key(&quorum_hash, quorum_type);
        let key =
            tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(key_fut))
                .map_err(|e| drive_proof_verifier::Error::InvalidQuorum {
                    error: e.to_string(),
                })?;

        Ok(key)
    }
}
