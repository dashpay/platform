mod test_vector;

use core::panic;

use base64::Engine;
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
    serialization::PlatformDeserializableWithBytesLenFromVersionedStructure,
    util::cbor_serializer,
    version::{PlatformVersion, PlatformVersionCurrentVersion},
};

use drive_abci::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use drive_proof_verifier::{get_proof, FromProof, QuorumInfoProvider};
use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};
use test_vector::TestVector;
use tokio::sync::Mutex;

pub const PLATFORM_IP: &str = "10.56.229.104";
pub const CORE_PORT: u16 = 30002;
pub const CORE_USER: &str = "iHAedM4G";
pub const CORE_PASSWORD: &str = "Cigmoac4RGIm";
pub const PLATFORM_PORT: u16 = 2443;

pub const IDENTITY_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

pub const CONTRACT_ID: &str = "aYMQ2D64Q40H/uJII2QZZpGzgOSAJ6Jy0L5BVJ1axUA=";
pub const DOCUMENT_ID: &str = "yjF6JZyblrHUrHxbGq0K7mLKsnN5lM7ij+ZAyGaDMg8=";
pub const DOCUMENT_TYPE: &str = "indexedDocument";

pub const DATA_CONTRACT_FULL:&str="AL6HZpFttAsUU3Bl46/ZjOBMWgATevQPNQib+ivReoLGAAAAAQABAcC7x/c7FAtVStFwfjR+LfUMu+U9xBHhDA3xnX3bBtmPAAMPaW5kZXhlZERvY3VtZW50FgUSBHR5cGUSBm9iamVjdBIHaW5kaWNlcxUGFgMSBG5hbWUSBmluZGV4MRIKcHJvcGVydGllcxUCFgESCCRvd25lcklkEgNhc2MWARIJZmlyc3ROYW1lEgNhc2MSBnVuaXF1ZRMBFgMSBG5hbWUSBmluZGV4MhIKcHJvcGVydGllcxUCFgESCCRvd25lcklkEgNhc2MWARIIbGFzdE5hbWUSA2FzYxIGdW5pcXVlEwEWAhIEbmFtZRIGaW5kZXgzEgpwcm9wZXJ0aWVzFQEWARIIbGFzdE5hbWUSA2FzYxYCEgRuYW1lEgZpbmRleDQSCnByb3BlcnRpZXMVAhYBEgokY3JlYXRlZEF0EgNhc2MWARIKJHVwZGF0ZWRBdBIDYXNjFgISBG5hbWUSBmluZGV4NRIKcHJvcGVydGllcxUBFgESCiR1cGRhdGVkQXQSA2FzYxYCEgRuYW1lEgZpbmRleDYSCnByb3BlcnRpZXMVARYBEgokY3JlYXRlZEF0EgNhc2MSCnByb3BlcnRpZXMWAhIJZmlyc3ROYW1lFgISBHR5cGUSBnN0cmluZxIJbWF4TGVuZ3RoA34SCGxhc3ROYW1lFgISBHR5cGUSBnN0cmluZxIJbWF4TGVuZ3RoA34SCHJlcXVpcmVkFQQSCWZpcnN0TmFtZRIKJGNyZWF0ZWRBdBIKJHVwZGF0ZWRBdBIIbGFzdE5hbWUSFGFkZGl0aW9uYWxQcm9wZXJ0aWVzEwAMbmljZURvY3VtZW50FgQSBHR5cGUSBm9iamVjdBIKcHJvcGVydGllcxYBEgRuYW1lFgESBHR5cGUSBnN0cmluZxIIcmVxdWlyZWQVARIKJGNyZWF0ZWRBdBIUYWRkaXRpb25hbFByb3BlcnRpZXMTAA53aXRoQnl0ZUFycmF5cxYFEgR0eXBlEgZvYmplY3QSB2luZGljZXMVARYCEgRuYW1lEgZpbmRleDESCnByb3BlcnRpZXMVARYBEg5ieXRlQXJyYXlGaWVsZBIDYXNjEgpwcm9wZXJ0aWVzFgISDmJ5dGVBcnJheUZpZWxkFgMSBHR5cGUSBWFycmF5EglieXRlQXJyYXkTARIIbWF4SXRlbXMDIBIPaWRlbnRpZmllckZpZWxkFgUSBHR5cGUSBWFycmF5EglieXRlQXJyYXkTARIQY29udGVudE1lZGlhVHlwZRIhYXBwbGljYXRpb24veC5kYXNoLmRwcC5pZGVudGlmaWVyEghtaW5JdGVtcwNAEghtYXhJdGVtcwNAEghyZXF1aXJlZBUBEg5ieXRlQXJyYXlGaWVsZBIUYWRkaXRpb25hbFByb3BlcnRpZXMTAA==";

#[tokio::main]
async fn main() {
    PlatformVersion::set_current(PlatformVersion::latest());
    let api = Api::default();
    run_tests(api).await;
}

async fn run_tests(api: Api) {
    println!(
        "\"get_identity\": {{\n{}\n}}\n",
        api.get_identity(&IDENTITY_ID_BYTES).await
    );

    let b64 = base64::engine::general_purpose::STANDARD;
    let contract_id: [u8; 32] = b64
        .decode(CONTRACT_ID)
        .expect("base64 decode")
        .try_into()
        .expect("contract identifier size");

    // let (contract, contract_test_vector) = api.get_contract(&contract_id).await;
    // println!(r##""get_contract":{{\n{}\n}}"##, contract_test_vector);

    let contract_bytes = b64.decode(DATA_CONTRACT_FULL).expect("base64 decode");
    let (contract, _) = DataContract::versioned_deserialize_with_bytes_len(
        &contract_bytes,
        true,
        PlatformVersion::first(),
    )
    .expect("data contract");

    // let doctype = contract
    //     .document_types()
    //     .first_key_value()
    //     .expect("contract contains doc type");
    // let doc_type_name = doctype.1.name().to_owned();
    let doc_type_name = DOCUMENT_TYPE.to_string();

    println!(
        "\"get_documents\": {{\n{}\n}}\n",
        api.get_documents(contract, &doc_type_name).await
    );
}
struct Api {
    dapi: Mutex<DapiClient>,
    // TODO: Replace with rs-sdk implementation when it's ready
    core: Mutex<DefaultCoreRPC>,
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
            dapi: Mutex::new(dapi),
            core: Mutex::new(core),
        }
    }

    async fn get_identity(&self, id: &[u8; 32]) -> String {
        let request = platform_proto::GetIdentityRequest {
            id: id.to_vec(),
            prove: true,
        };

        let mut client = self.dapi.lock().await;
        let response: GetIdentityResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        let proof = get_proof!(response, get_identity_response::Result)
            .expect("proof not present in response");

        self.test_vector(&request, &response, proof, None).await
    }

    async fn get_contract(&self, id: &[u8; 32]) -> (DataContract, String) {
        let request = platform_proto::GetDataContractRequest {
            id: id.to_vec(),
            prove: false,
        };
        println!("Contract ID: {}", hex::encode(id));

        let mut client = self.dapi.lock().await;
        let mut response: platform_proto::GetDataContractResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        if let Some(mtd) = &mut response.metadata {
            mtd.chain_id = "dashmate_local_32".to_string();
        }

        let proof = get_proof!(response, platform_proto::get_data_contract_response::Result)
            .expect("proof not present in response");

        let contract = DataContract::from_proof(request.clone(), response.clone(), &self)
            .expect("get data contract from proof");

        (
            contract,
            self.test_vector(&request, &response, proof, None).await,
        )
    }

    async fn get_documents(&self, data_contract: DataContract, doc_type_name: &str) -> String {
        let data_contract_id = data_contract.id();

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

        let mut client = self.dapi.lock().await;
        let response: platform_proto::GetDocumentsResponse = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        let proof = get_proof!(response, platform_proto::get_documents_response::Result)
            .expect("proof not present in response");

        self.test_vector(&request, &response, proof, Some(data_contract))
            .await
    }

    async fn get_quorum_key(&self, quorum_hash: &[u8], quorum_type: u32) -> Vec<u8> {
        let quorum_hash = QuorumHash::from_slice(quorum_hash).expect("valid quorum hash expected");

        let core = self.core.lock().await;
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
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        let key_fut = self.get_quorum_key(&quorum_hash, quorum_type);
        let key =
            tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(key_fut));

        let key = key.try_into().expect("quorum key size");
        Ok(key)
    }
}
