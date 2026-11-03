//! Tests of the current-versions lookup (`getDataContractsLatestVersions`).
//!
//! The recorded vectors come from a local devnet seeded with the SDK test data. Until they
//! are recorded with `scripts/generate_test_vectors.sh <test name>`, these tests are ignored
//! in offline mode.

use crate::fetch::config::Config;
use dash_sdk::platform::data_contracts_latest_versions::{
    DataContractLatestVersion, DataContractsLatestVersions, DataContractsLatestVersionsQuery,
};
use dash_sdk::platform::{FetchMany, FetchUnproved, Identifier};
use dpp::data_contract::accessors::v0::DataContractV0Getters;

/// The proved lookup of the configured contract and an unknown id returns the version of the
/// first, no entry for the second, and no contracts.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_latest_versions_read() {
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;
    let unknown_id = Identifier::from_bytes(&[1; 32]).expect("32 byte identifier");
    let sdk = cfg
        .setup_api("test_data_contracts_latest_versions_read")
        .await;

    let versions = DataContractLatestVersion::fetch_many(&sdk, vec![id, unknown_id])
        .await
        .expect("fetch the contract versions");

    assert_eq!(versions.0.len(), 2, "one entry per requested id");
    assert!(
        versions
            .version_of(&id)
            .expect("the configured contract exists")
            >= 1,
        "contract versions start at 1"
    );
    assert_eq!(
        versions.0[&unknown_id], None,
        "an unknown id has no version"
    );
    assert!(
        versions.0[&id]
            .as_ref()
            .expect("found")
            .data_contract
            .is_none(),
        "contracts are only returned on request"
    );
}

/// With `include_contracts`, the proved lookup also returns the contracts, at those versions.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_latest_versions_read_with_contracts() {
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;
    let sdk = cfg
        .setup_api("test_data_contracts_latest_versions_read_with_contracts")
        .await;

    let versions = DataContractLatestVersion::fetch_many(
        &sdk,
        DataContractsLatestVersionsQuery::with_contracts(vec![id]),
    )
    .await
    .expect("fetch the contract versions with the contracts");

    let entry = versions.0[&id]
        .clone()
        .expect("the configured contract exists");
    let contract = entry.data_contract.expect("the contract was asked for");
    assert_eq!(contract.id(), id);
    assert_eq!(contract.version(), entry.version);
}

/// The unproved lookup (the fast path) agrees with the proved one.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_latest_versions_unproved() {
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;
    let unknown_id = Identifier::from_bytes(&[1; 32]).expect("32 byte identifier");
    let sdk = cfg
        .setup_api("test_data_contracts_latest_versions_unproved")
        .await;

    let proved = DataContractLatestVersion::fetch_many(&sdk, vec![id, unknown_id])
        .await
        .expect("fetch the proved contract versions");
    let unproved = DataContractsLatestVersions::fetch_unproved(&sdk, vec![id, unknown_id])
        .await
        .expect("fetch the unproved contract versions")
        .expect("the response carries entries");

    assert_eq!(unproved.version_of(&id), proved.version_of(&id));
    assert_eq!(unproved.0[&unknown_id], None);
    assert!(
        unproved.0[&id]
            .as_ref()
            .expect("found")
            .data_contract
            .is_none(),
        "contracts are only returned on request"
    );
}
