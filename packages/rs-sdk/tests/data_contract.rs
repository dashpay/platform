use dpp::{data_contract::accessors::v0::DataContractV0Getters, prelude::Identifier};
use rs_sdk::crud::ReadOnly;

include!("credentials.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read_not_found() {
    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let api = rs_sdk::dapi::Api::new(
        PLATFORM_IP,
        CORE_PORT,
        CORE_USER,
        CORE_PASSWORD,
        PLATFORM_PORT,
    )
    .expect("initialize api");

    let result = rs_sdk::platform::data_contract::DataContract::read(&api, &id).await;

    assert!(matches!(
        result,
        Err(rs_sdk::error::Error::Proof(
            drive_proof_verifier::Error::NotFound
        ))
    ));
}
