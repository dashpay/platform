use dpp::prelude::{DataContract, Identifier};
use rs_sdk::platform::Fetch;

include!("common.rs");

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read_not_found() {
    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let mut api = setup_api();

    let result = DataContract::fetch(&mut api, id).await;

    assert!(matches!(result, Ok(None)), "result: {:?}", result);
}

#[ignore = "needs working platform"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read() {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    let id = base64_identifier(DATA_CONTRACT_ID);

    let mut api = setup_api();

    let result = DataContract::fetch(&mut api, id).await;

    assert!(matches!(result, Ok(Some(_))), "result: {:?}", result);
    assert_eq!(result.unwrap().unwrap().id(), id);
}
