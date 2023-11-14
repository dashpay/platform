use crate::config::Config;
use dpp::prelude::{DataContract, Identifier};
use rs_sdk::platform::Fetch;

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read_not_found() {
    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let cfg = Config::new();
    let mut api = cfg.setup_api().await;

    let result = DataContract::fetch(&mut api, id).await;

    assert!(matches!(result, Ok(None)), "result: {:?}", result);
}

/// Given some existing data contract ID, when I fetch data contract, I get the data contract.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read() {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;

    let mut api = cfg.setup_api().await;

    let result = DataContract::fetch(&mut api, id).await;

    assert!(matches!(result, Ok(Some(_))), "result: {:?}", result);
    assert_eq!(result.unwrap().unwrap().id(), id);
}
