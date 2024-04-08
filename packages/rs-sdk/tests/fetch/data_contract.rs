use super::config::Config;
use dpp::prelude::{DataContract, Identifier};
use rs_sdk::platform::{Fetch, FetchMany};

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read_not_found() {
    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_data_contract_read_not_found").await;

    let result = DataContract::fetch(&sdk, id).await;

    assert!(matches!(result, Ok(None)), "result: {:?}", result);
}

/// Given some existing data contract ID, when I fetch data contract, I get the data contract.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contract_read() {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;

    let sdk = cfg.setup_api("test_data_contract_read").await;

    let result = DataContract::fetch_by_identifier(&sdk, id).await;

    assert!(matches!(result, Ok(Some(_))), "result: {:?}", result);
    assert_eq!(result.unwrap().unwrap().id(), id);
}

/// Given existing and non-existing data contract IDs, when I fetch them, I get exitingthe data contract.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contracts_1_ok_1_nx() {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;
    let nx_id = Identifier::from_bytes(&[1; 32]).expect("parse identity id");
    let ids = [id, nx_id];

    let sdk = cfg.setup_api("test_data_contracts_1_ok_1_nx").await;

    let result = DataContract::fetch_by_identifiers(&sdk, ids)
        .await
        .expect("fetch many data contracts");

    assert!(!result.is_empty());
    // existing one
    assert_eq!(
        id,
        result
            .get(&id)
            .expect("found in result")
            .as_ref()
            .expect("exists")
            .id(),
        "existing data contract id match"
    );
    // not existing one
    assert!(
        result.get(&nx_id).expect("found in result").is_none(),
        "proof of non-existence failed"
    );
}

/// Given two non-existing data contract IDs, I get None for both.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_data_contracts_2_nx() {
    let cfg = Config::new();
    let nx_id_1 = Identifier::from_bytes(&[0; 32]).expect("parse identity id");
    let nx_id_2 = Identifier::from_bytes(&[1; 32]).expect("parse identity id");
    let ids = vec![nx_id_1, nx_id_2];

    let sdk = cfg.setup_api("test_data_contracts_2_nx").await;

    let result = DataContract::fetch_many(&sdk, ids)
        .await
        .expect("fetch many data contracts");

    assert!(
        result.get(&nx_id_1).expect("found in result").is_none(),
        "proof of non-existence 1 failed"
    );
    assert!(
        result.get(&nx_id_2).expect("found in result").is_none(),
        "proof of non-existence 2 failed"
    );
}
