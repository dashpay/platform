//! Dash API implementation

use crate::error::Error;
use dapi_grpc::platform::v0::{self as platform_proto};
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::dashcore::{hashes::Hash, QuorumHash};
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_proof_verifier::proof::from_proof::{self, QuorumInfoProvider};
use rs_dapi_client::{AddressList, DapiClient, RequestSettings};
use tokio::sync::{RwLock, RwLockWriteGuard};

#[async_trait::async_trait]
pub trait DAPI: Send + Sync {
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
    /*
        async fn get_documents(&self, data_contract: DataContract, doc_type_name: &str) -> String {
            let data_contract_id = data_contract.id();
            // let dc: DataContractV0 = data_contract.into_v0().expect("data contract v0");
            // dc.id();
            //get_documents::GetDocumentsRequest
            let empty: Vec<u8> = Vec::new();
            let empty =
                cbor_serializer::serializable_value_to_cbor(&empty, None).expect("serialize empty vec");

            let request: platform_proto::GetDocumentsRequest = platform_proto::GetDocumentsRequest {
                data_contract_id: data_contract_id.to_vec(),
                document_type: doc_type_name.to_string(),
                limit: 10,
                start: None,
                order_by: empty.clone(),
                r#where: empty,
                prove: true,
            }
            .into();

            // return serde_json::to_string_pretty(&request).expect("request json serialization");

            let mut client = self.dapi.write().await;
            let response: platform_proto::GetDocumentsResponse = request
                .clone()
                .execute(&mut client, RequestSettings::default())
                .await
                .expect("unable to perform dapi request");

            let proof = get_proof!(response, platform_proto::get_documents_response::Result);

            self.test_vector(&request, &response, proof, Some(data_contract))
                .await
        }
    */
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
impl DAPI for Api {
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
