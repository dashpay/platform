use dpp::prelude::{DataContract, Identifier};
use rs_sdk::platform::Fetch;

// Some constants we need to connect to the platform
include!("../tests/common.rs");

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let api = setup_api(); // faux

    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");
    let result = DataContract::fetch(&api, id).await;

    assert!(matches!(
        result,
        Err(rs_sdk::error::Error::Proof(
            drive_proof_verifier::Error::NotFound
        ))
    ));
}
