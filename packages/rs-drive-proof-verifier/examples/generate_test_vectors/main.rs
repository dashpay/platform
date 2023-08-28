#[macro_use]
mod test_vector;

use core::panic;

use dapi_grpc::platform::v0::{self as platform_proto, get_identity_response, GetIdentityResponse};
use dashcore_rpc::{
    dashcore::{hashes::Hash, QuorumHash},
    dashcore_rpc_json::QuorumType,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    prelude::DataContract,
};

use drive_abci::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use drive_proof_verifier::proof::from_proof::{FromProof, QuorumInfoProvider};
use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};
use test_vector::TestVector;
use tokio::sync::RwLock;

pub const PLATFORM_IP: &str = "10.56.229.104";
pub const CORE_PORT: u16 = 20002;
pub const CORE_USER: &str = "uBdzLuhP";
pub const CORE_PASSWORD: &str = "1TRQ7BFqSuIn";
pub const PLATFORM_PORT: u16 = 2443;

pub const IDENTITY_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

#[tokio::main]
async fn main() {
    let api = Api::default();
    println!(
        r##""get_identity":{{\n{}\n}}"##,
        api.get_identity(&IDENTITY_ID_BYTES).await
    );

    let (contract, contract_test_vector) = api.get_contract(&IDENTITY_ID_BYTES).await;
    println!(r##""get_contract":{{\n{}\n}}"##, contract_test_vector);

    let doctype = contract
        .document_types()
        .first_key_value()
        .expect("contract contains doc type");
    let doc_type_name = doctype.1.name().to_owned();

    println!(
        r##""get_documents":{{\n{}\n}}"##,
        api.get_documents(contract, &doc_type_name).await
    );
}

struct Api {
    dapi: RwLock<DapiClient>,
    // TODO: Replace with rs-sdk implementation when it's ready
    core: RwLock<DefaultCoreRPC>,
}

impl Default for Api {
    fn default() -> Self {
        Self::new(
            PLATFORM_IP,
            CORE_PORT,
            CORE_USER,
            CORE_PASSWORD,
            PLATFORM_PORT,
        )
    }
}

impl Api {
    pub fn new(
        address: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
        platform_port: u16,
    ) -> Self {
        let mut address_list = AddressList::new();
        let platform_addr =
            rs_dapi_client::Uri::from_maybe_shared(format!("http://{}:{}", address, platform_port))
                .expect("platform address");
        address_list.add_uri(platform_addr);
        let dapi = DapiClient::new(address_list, RequestSettings::default());

        let core_addr = format!("http://{}:{}", address, core_port);
        let core =
            DefaultCoreRPC::open(&core_addr, core_user.to_string(), core_password.to_string())
                .expect("connect to core");

        Self {
            dapi: RwLock::new(dapi),
            core: RwLock::new(core),
        }
    }

    async fn get_identity(&self, id: &[u8; 32]) -> String {
        let request = platform_proto::GetIdentityRequest {
            id: id.to_vec(),
            prove: true,
        };

        let mut client = self.dapi.write().await;
        let response: GetIdentityResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        let proof = get_proof!(response, get_identity_response::Result);

        self.test_vector(&request, &response, proof, None).await
    }

    async fn get_contract(&self, id: &[u8; 32]) -> (DataContract, String) {
        let request = platform_proto::GetDataContractRequest {
            id: id.to_vec(),
            prove: true,
        };

        let mut client = self.dapi.write().await;
        let response: platform_proto::GetDataContractResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        let proof = get_proof!(response, platform_proto::get_data_contract_response::Result);

        let contract = DataContract::from_proof(&request, &response, Box::new(self))
            .expect("get data contract from proof");

        (
            contract,
            self.test_vector(&request, &response, proof, None).await,
        )
    }

    async fn get_documents(&self, data_contract: DataContract, doc_type_name: &str) -> String {
        let data_contract_id = data_contract.id();
        // let dc: DataContractV0 = data_contract.into_v0().expect("data contract v0");
        // dc.id();

        let request = platform_proto::GetDocumentsRequest {
            data_contract_id: data_contract_id.to_vec(),
            document_type: doc_type_name.to_string(),
            limit: 10,
            order_by: Vec::new(),
            r#where: Vec::new(),
            start: None,
            prove: true,
        };

        let mut client = self.dapi.write().await;
        let response: platform_proto::GetDocumentsResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        let proof = get_proof!(response, platform_proto::get_documents_response::Result);

        self.test_vector(&request, &response, proof, None).await
    }

    async fn get_quorum_key(&self, quorum_hash: &[u8], quorum_type: u32) -> Vec<u8> {
        let quorum_hash = QuorumHash::from_slice(quorum_hash).expect("valid quorum hash expected");

        let core = self.core.write().await;
        let quorum_info = core
            .get_quorum_info(QuorumType::from(quorum_type), &quorum_hash, None)
            .expect("get quorum info");

        quorum_info.quorum_public_key
    }
}

impl QuorumInfoProvider for &Api {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: Vec<u8>,
        _core_chain_locked_height: u32,
    ) -> Result<Vec<u8>, drive_proof_verifier::Error> {
        let rt = tokio::runtime::Handle::current();
        let key = rt.block_on(self.get_quorum_key(&quorum_hash, quorum_type));

        Ok(key)
    }
}
