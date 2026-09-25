//! Tests of the contract group queries (`getContractGroupInfo`, `getContractGroupMembers`,
//! `getContractGroupsForContract`).
//!
//! No contract group exists on the SDK test network yet: registering one takes a
//! `DataContractCreateTransitionV1`, which the SDK does not build until the creation surfaces
//! land. These tests therefore read absent ids, which every query answers and proves, and
//! are ignored in offline mode until vectors are recorded with
//! `scripts/generate_test_vectors.sh <test name>`.

use crate::fetch::config::Config;
use dash_sdk::platform::contract_groups::{
    ContractGroupInfo, ContractGroupMembersPage, ContractGroupMembersPageQuery,
    ContractGroupMembershipsForContract,
};
use dash_sdk::platform::{Fetch, FetchUnproved, Identifier};

/// An id no group has fetches as `None`, proved and unproved alike.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_contract_group_info_absent() {
    let cfg = Config::new();
    let unknown_id = Identifier::from_bytes(&[1; 32]).expect("32 byte identifier");
    let sdk = cfg.setup_api("test_contract_group_info_absent").await;

    let proved = ContractGroupInfo::fetch(&sdk, unknown_id)
        .await
        .expect("fetch the group info");
    assert_eq!(proved, None, "no group has this id");

    let unproved = ContractGroupInfo::fetch_unproved(&sdk, unknown_id)
        .await
        .expect("fetch the unproved group info");
    assert_eq!(unproved, None);
}

/// The members of an absent group are an empty page of the requested kind, and the page
/// has no successor.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_contract_group_members_absent() {
    let cfg = Config::new();
    let unknown_id = Identifier::from_bytes(&[1; 32]).expect("32 byte identifier");
    let sdk = cfg.setup_api("test_contract_group_members_absent").await;

    let query = ContractGroupMembersPageQuery::document_types(unknown_id).with_limit(10);
    let page = ContractGroupMembersPage::fetch(&sdk, query.clone())
        .await
        .expect("fetch the members page")
        .expect("a page is always returned");
    assert_eq!(page, ContractGroupMembersPage::DocumentTypes(vec![]));
    assert_eq!(query.after(&page), None, "an empty page is the last one");

    let unproved = ContractGroupMembersPage::fetch_unproved(&sdk, query)
        .await
        .expect("fetch the unproved members page");
    assert_eq!(unproved, Some(page));
}

/// A contract in no group, here the configured contract, has empty memberships.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_contract_groups_for_contract_none() {
    let cfg = Config::new();
    let id = cfg.existing_data_contract_id;
    let sdk = cfg
        .setup_api("test_contract_groups_for_contract_none")
        .await;

    let memberships = ContractGroupMembershipsForContract::fetch(&sdk, id)
        .await
        .expect("fetch the memberships")
        .expect("memberships are always returned");
    assert!(memberships.is_empty(), "the contract belongs to no group");

    let unproved = ContractGroupMembershipsForContract::fetch_unproved(&sdk, id)
        .await
        .expect("fetch the unproved memberships");
    assert_eq!(unproved, Some(memberships));
}
