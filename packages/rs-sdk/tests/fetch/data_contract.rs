use super::config::Config;
use dash_sdk::platform::{Fetch, FetchMany, LimitQuery};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::{DataContract, Identifier};
use drive_proof_verifier::types::DataContractHistory;

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

/// Given some existing historical data contract ID, when I fetch the data contract history, I get it.
///
///
/// ## Setup
///
/// It needs Data Contract with `keep_history` set to true, which is not available in the network
/// by default and has to be created manually.
/// At the moment tests in rs-sdk do not provide Core Wallet signer, and unable to create
/// identities and data contracts, so we cannot do that setup automatically.
///
/// To run this test, you need to create a Data Contract with `keep_history` set to true.
/// You can use `packages/platform-test-suite/test/functional/platform/DataContract.spec.js` test for this.
///
/// 1. Modify the test to display ID of the created Data Contract, for example:
///     ```diff
///     diff --git a/packages/platform-test-suite/test/functional/platform/DataContract.spec.js b/packages/platform-test-suite/test/functional/platform/DataContract.spec.js
///     index 28fe50b14..5864b3a14 100644
///     --- a/packages/platform-test-suite/test/functional/platform/DataContract.spec.js
///     +++ b/packages/platform-test-suite/test/functional/platform/DataContract.spec.js
///     @@ -133,7 +133,7 @@ describe('Platform', () => {
///         it('should be able to update an existing data contract', async () => {
///         // Additional wait time to mitigate testnet latency
///         await waitForSTPropagated();
///     -
///     +   console.log(dataContractFixture.getId().toString("hex"));
///         const fetchedDataContract = await client.platform.contracts.get(
///             dataContractFixture.getId(),
///         );
///     ```
///
/// 2. Run the test:
///    ```bash
///    cd packages/platform-test-suite
///    yarn mocha -b test/functional/platform/DataContract.spec.js
///    ```
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "This test requires special procedure to regenerate test vectors"
)]
async fn test_data_contract_history_read() {
    let cfg = Config::new();
    let id = Identifier::from_string(
        "20d16030541c0494e84064e2e72b5ec620546305849a2f9d5893a5e65072364d",
        Encoding::Hex,
    )
    .unwrap();

    let sdk = cfg.setup_api("test_data_contract_history_read").await;

    let result = DataContractHistory::fetch(&sdk, LimitQuery::from((id, 10))).await;

    assert!(matches!(result, Ok(Some(_))), "result: {:?}", result);
    let (_, contract) = result.unwrap().unwrap().pop_first().unwrap();
    assert_eq!(contract.id(), id);
}
