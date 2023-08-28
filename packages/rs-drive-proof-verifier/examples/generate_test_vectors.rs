use core::panic;

use dapi_grpc::platform::v0::{
    self as platform_proto, get_identity_response, GetIdentityRequest, GetIdentityResponse, Proof,
    ResponseMetadata,
};
use dashcore_rpc::{
    dashcore::{
        hashes::{sha256d, Hash},
        QuorumHash,
    },
    dashcore_rpc_json::{QuorumInfoResult, QuorumType},
    jsonrpc::Response,
};
use dpp::prelude::DataContract;
use drive_abci::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};
use serde::Serialize;

pub const PLATFORM_IP: &str = "10.56.229.104";

pub const OWNER_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

#[tokio::main]
async fn main() {
    get_identity(&OWNER_ID_BYTES).await
}
macro_rules! create_metadata {
    ($response:expr, $result_type:ty) => {{
        use $result_type as Result;
        let proof = if let Result::Proof(proof) = $response.result.as_ref().expect("result") {
            proof
        } else {
            panic!("missing proof in response")
        };

        TestMetadata {
            quorum_public_key: get_quorum_key(&proof.quorum_hash),
            data_contract: None,
        }
    }};

    ($response:expr, $result_type:ty, $data_contract:expr) => {{
        let mut mtd = create_metadata!($response, $result_type);
        mtd.data_contract = $data_contract;

        mtd
    }};
}

async fn get_identity(id: &[u8; 32]) {
    let mut address_list = AddressList::new();
    let addr = rs_dapi_client::Uri::from_maybe_shared(format!("http://{}:2443", PLATFORM_IP))
        .expect("Valid URI");
    address_list.add_uri(addr);

    let mut client = DapiClient::new(address_list, RequestSettings::default());

    let request = platform_proto::GetIdentityRequest {
        id: id.to_vec(),
        prove: true,
    };

    let response: GetIdentityResponse = request
        .clone()
        .execute(&mut client, RequestSettings::default())
        .await
        .expect("unable to perform dapi request");

    let mtd = create_metadata!(response, get_identity_response::Result, None);

    print_test_vector(request, response, mtd);
}

fn get_quorum_key(quorum_hash: &[u8]) -> Vec<u8> {
    let url = format!("http://{}:", PLATFORM_IP);
    let conn = DefaultCoreRPC::open(&url, "".to_string(), "".to_string()).expect("connect to core");
    let quorum_hash = QuorumHash::from_slice(quorum_hash).expect("valid quorum hash expected");
    let quorum_info = conn
        .get_quorum_info(QuorumType::LlmqTestPlatform, &quorum_hash, None)
        .expect("get quorum info");

    quorum_info.quorum_public_key
}

fn print_test_vector<'de, I, O>(request: I, response: O, mtd: TestMetadata)
where
    I: Serialize,
    O: Serialize,
{
    let output = (request, response, mtd);
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("json generation failed")
    );
}

include!("../tests/utils.rs");
