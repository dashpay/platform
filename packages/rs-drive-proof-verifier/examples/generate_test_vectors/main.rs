mod test_vector;

use base64::Engine;
use dapi_grpc::platform::v0::{self as platform_proto, get_identity_response, GetIdentityResponse};

use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    prelude::DataContract,
    util::cbor_serializer,
    version::{PlatformVersion, PlatformVersionCurrentVersion},
};

use drive_proof_verifier::{get_proof, FromProof};
use rs_sdk::platform::dapi::{
    transport::TransportRequest, AddressList, DapiRequest, RequestSettings, Uri,
};

pub const PLATFORM_IP: &str = "10.56.229.104";
pub const CORE_PORT: u16 = 30002;
pub const CORE_USER: &str = "546b8x1g";
pub const CORE_PASSWORD: &str = "ur4mn8Z6ObI3";
pub const PLATFORM_PORT: u16 = 2443;

pub const IDENTITY_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

const DATA_CONTRACT_ID: &str = "rt863oRafQ4pkBAUJ5r1PAf8lZ9O7C0qBeLwmHXDEro=";

#[tokio::main]
async fn main() {
    PlatformVersion::set_current(PlatformVersion::latest());
    let api = Api::default();
    run_tests(api).await;
}

async fn run_tests(mut api: Api) {
    println!(
        "\"get_identity\": {{\n{}\n}}\n",
        api.get_identity(&IDENTITY_ID_BYTES).await
    );

    let b64 = base64::engine::general_purpose::STANDARD;
    let contract_id: [u8; 32] = b64
        .decode(DATA_CONTRACT_ID)
        .expect("base64 decode")
        .try_into()
        .expect("contract identifier size");

    let (contract, contract_test_vector) = api.get_contract(&contract_id).await;
    println!(r##""get_contract":{{\n{}\n}}"##, contract_test_vector);

    let doctype = contract
        .document_types()
        .first_key_value()
        .expect("contract contains doc type");
    let doc_type_name = doctype.1.name().to_owned();

    println!(
        "\"get_documents\": {{\n{}\n}}\n",
        api.get_documents(contract, &doc_type_name).await
    );
}
struct Api {
    sdk: rs_sdk::Sdk,
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
        let platform_addr = Uri::from_maybe_shared(format!("http://{}:{}", address, platform_port))
            .expect("platform address");
        address_list.add_uri(platform_addr);

        let sdk = rs_sdk::SdkBuilder::new(address_list)
            .with_core(address, core_port, core_user, core_password)
            .build()
            .expect("configure dash sdk");

        Self { sdk }
    }

    async fn call<R: TransportRequest>(&mut self, request: &R) -> R::Response {
        let response: R::Response = request
            .clone()
            .execute(&mut self.sdk, RequestSettings::default())
            .await
            .expect("unable to perform dapi request");

        response
    }

    async fn get_identity(&mut self, id: &[u8; 32]) -> String {
        let request = platform_proto::GetIdentityRequest {
            id: id.to_vec(),
            prove: true,
        };

        let response: GetIdentityResponse = self.call(&request).await;

        let proof = get_proof!(response, get_identity_response::Result)
            .expect("proof not present in response");

        self.test_vector(&request, &response, proof, None).await
    }

    async fn get_contract(&mut self, id: &[u8; 32]) -> (DataContract, String) {
        let request = platform_proto::GetDataContractRequest {
            id: id.to_vec(),
            prove: true,
        };
        println!("Contract ID: {}", hex::encode(id));

        let response: platform_proto::GetDataContractResponse = self.call(&request).await;

        let proof = get_proof!(response, platform_proto::get_data_contract_response::Result)
            .expect("proof not present in response");

        let contract = DataContract::from_proof(request.clone(), response.clone(), &mut self.sdk)
            .expect("get data contract from proof");

        (
            contract,
            self.test_vector(&request, &response, proof, None).await,
        )
    }

    async fn get_documents(&mut self, data_contract: DataContract, doc_type_name: &str) -> String {
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

        let response: platform_proto::GetDocumentsResponse = self.call(&request).await;

        let proof = get_proof!(response, platform_proto::get_documents_response::Result)
            .expect("proof not present in response");

        self.test_vector(&request, &response, proof, Some(data_contract))
            .await
    }
}
