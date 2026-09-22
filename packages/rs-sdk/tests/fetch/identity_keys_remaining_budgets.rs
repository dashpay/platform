//! Tests of the `getIdentityKeysRemainingBudgets` query.
//!
//! No identity on the SDK test network holds a budgeted key yet: registering one takes a
//! version 1 key, which the SDK does not build until the creation helpers land. The network
//! test therefore reads the keys of the configured identity, none of which has a budget, and
//! is ignored in offline mode until vectors are recorded with
//! `scripts/generate_test_vectors.sh <test name>`.

use crate::fetch::config::Config;
use dash_sdk::platform::identity_keys_remaining_budgets::{
    IdentityKeysRemainingBudgets, IdentityKeysRemainingBudgetsQuery,
};
use dash_sdk::platform::{Fetch, FetchUnproved, Identifier};
use dash_sdk::Sdk;

/// Given some remaining budgets, when I fetch them using mock API, then I get the same budgets,
/// a key without a budget included.
#[tokio::test]
async fn test_mock_fetch_identity_keys_remaining_budgets() {
    let mut sdk = Sdk::new_mock();

    let query = IdentityKeysRemainingBudgetsQuery {
        identity_id: Identifier::from_bytes(&[7; 32]).expect("32 byte identifier"),
        key_ids: vec![3, 4, 5],
    };
    let expected: IdentityKeysRemainingBudgets = [(3, Some(250_000_000)), (4, Some(0)), (5, None)]
        .into_iter()
        .collect();

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("register the expectation");

    let retrieved = IdentityKeysRemainingBudgets::fetch(&sdk, query)
        .await
        .expect("fetch the remaining budgets")
        .expect("budgets are returned");

    assert_eq!(retrieved, expected);
    assert_eq!(
        retrieved.get(&4),
        Some(&Some(0)),
        "a spent budget is not an absent one"
    );
    assert_eq!(retrieved.get(&5), Some(&None));
}

/// Keys without a budget are answered, and proved, as having none.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_identity_keys_remaining_budgets_of_unbudgeted_keys() {
    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_identity_keys_remaining_budgets_of_unbudgeted_keys")
        .await;

    let query = IdentityKeysRemainingBudgetsQuery {
        identity_id: cfg.existing_identity_id,
        key_ids: vec![0, 1],
    };

    let proved = IdentityKeysRemainingBudgets::fetch(&sdk, query.clone())
        .await
        .expect("fetch the remaining budgets")
        .expect("every requested key is answered");
    let expected: IdentityKeysRemainingBudgets = [(0, None), (1, None)].into_iter().collect();
    assert_eq!(proved, expected);

    let unproved = IdentityKeysRemainingBudgets::fetch_unproved(&sdk, query)
        .await
        .expect("fetch the unproved remaining budgets");
    assert_eq!(unproved, Some(expected));
}
