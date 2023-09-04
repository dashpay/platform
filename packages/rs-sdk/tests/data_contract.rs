use dpp::prelude::Identifier;
use rs_sdk::crud::Readable;

include!("common.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read_not_found() {
    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let api = setup_api();

    let result = rs_sdk::platform::data_contract::SdkDataContract::read(&api, &id).await;

    assert!(matches!(
        result,
        Err(rs_sdk::error::Error::Proof(
            drive_proof_verifier::Error::NotFound
        ))
    ));
}
