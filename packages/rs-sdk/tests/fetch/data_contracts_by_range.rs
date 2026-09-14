//! Tests of the paginated contract enumeration (`getDataContractsByRange`).
//!
//! The recorded vectors come from a local devnet seeded with the SDK test data (system
//! contracts plus the test contract). Until they are recorded with
//! `scripts/generate_test_vectors.sh <test name>`, these tests are ignored in offline mode.

use crate::fetch::config::Config;
use dash_sdk::platform::data_contracts_by_range::{
    DataContractsByRange, DataContractsByRangeQuery, DataContractsByRangeStart,
};
use dash_sdk::platform::{Fetch, Identifier};
use dash_sdk::Sdk;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

async fn fetch_page(sdk: &Sdk, query: DataContractsByRangeQuery) -> DataContractsByRange {
    DataContractsByRange::fetch(sdk, query)
        .await
        .expect("fetch a page of data contracts")
        .expect("a page is never absent, only empty")
}

fn ids(page: &DataContractsByRange) -> Vec<Identifier> {
    page.0.keys().copied().collect()
}

fn assert_ascending(ids: &[Identifier]) {
    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "contract ids must be strictly ascending: {ids:?}"
    );
}

/// The first page lists the devnet contracts in ascending id order, including the
/// configured existing contract, each entry carrying its contract.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_by_range_first_page() {
    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_data_contracts_by_range_first_page")
        .await;

    let page = fetch_page(&sdk, DataContractsByRangeQuery::first_page()).await;

    let page_ids = ids(&page);
    assert!(
        !page_ids.is_empty(),
        "a devnet has at least the system contracts"
    );
    assert_ascending(&page_ids);
    assert!(
        page_ids.contains(&cfg.existing_data_contract_id),
        "the configured existing contract is in the first page"
    );
    for (id, contract) in &page.0 {
        let contract = contract
            .as_ref()
            .expect("a full page carries every contract");
        assert_eq!(&contract.id(), id);
    }
}

/// Two-contract pages chain through `start_after`, and `start_at` includes the cursor.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_by_range_pagination() {
    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_data_contracts_by_range_pagination")
        .await;

    let first_query = DataContractsByRangeQuery {
        limit: Some(2),
        ..DataContractsByRangeQuery::first_page()
    };
    let first = fetch_page(&sdk, first_query.clone()).await;
    let first_ids = ids(&first);
    assert_eq!(first_ids.len(), 2, "the devnet has more than two contracts");
    assert_ascending(&first_ids);

    let cursor = first
        .next_start_after()
        .expect("a two-row page has a cursor");
    let second_query = first_query
        .next_page(&first)
        .expect("a non-empty page has a next page");
    let second = fetch_page(&sdk, second_query).await;
    let second_ids = ids(&second);
    assert!(
        second_ids.iter().all(|id| *id > cursor),
        "start_after excludes the cursor and everything before it"
    );
    assert_ascending(&second_ids);

    let from_cursor = fetch_page(
        &sdk,
        DataContractsByRangeQuery {
            start: Some(DataContractsByRangeStart::At(cursor)),
            limit: Some(2),
            ids_only: false,
        },
    )
    .await;
    assert_eq!(
        ids(&from_cursor).first(),
        Some(&cursor),
        "start_at includes the cursor"
    );
}

/// An ids-only page lists the same ids as the full page, with no contract bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_by_range_ids_only() {
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_data_contracts_by_range_ids_only").await;

    let full = fetch_page(&sdk, DataContractsByRangeQuery::first_page()).await;
    let ids_only = fetch_page(
        &sdk,
        DataContractsByRangeQuery {
            ids_only: true,
            ..DataContractsByRangeQuery::first_page()
        },
    )
    .await;

    assert_eq!(ids(&ids_only), ids(&full));
    assert!(ids_only.0.values().all(Option::is_none));
}

/// Starting after the greatest possible id yields an empty, still proved, page.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "offline-testing",
    ignore = "requires recorded test vectors; run scripts/generate_test_vectors.sh against a running Platform"
)]
async fn test_data_contracts_by_range_past_end() {
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_data_contracts_by_range_past_end").await;

    let query = DataContractsByRangeQuery {
        start: Some(DataContractsByRangeStart::After(Identifier::new(
            [0xff; 32],
        ))),
        limit: Some(10),
        ids_only: false,
    };
    let page = fetch_page(&sdk, query.clone()).await;

    assert!(page.0.is_empty());
    assert!(page.is_last_page(10));
    assert!(query.next_page(&page).is_none());
}
