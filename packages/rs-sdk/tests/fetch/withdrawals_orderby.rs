//! Reproduces the asc/desc discrepancy reported in
//! https://github.com/dashpay/platform/issues/2409 — querying the system
//! `withdrawals` data contract with the same `where` clauses but flipped
//! `orderBy` direction returns *different* sets of documents (should be
//! the same set, different order).
//!
//! This module is only compiled under `--features network-testing` and
//! talks to live **mainnet** DAPI. When the bug is fixed, these tests
//! will fail — that is the point: they are regression detectors for the
//! current broken behavior.
//!
//! Run:
//!
//! ```bash
//! cargo test -p dash-sdk --no-default-features --features network-testing \
//!   --test fetch withdrawals_orderby -- --nocapture
//! ```

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::Arc;

use super::common::setup_logs;
use dash_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dash_sdk::SdkBuilder;
use dpp::dashcore::Network;
use dpp::document::Document;
use dpp::platform_value::{Identifier, IdentifierBytes32, Value};
use dpp::prelude::DataContract;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use rs_dapi_client::AddressList;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

/// Mainnet DAPI evonode addresses, matching the list used by
/// `packages/rs-sdk-ffi/src/sdk.rs` for `dash_sdk_create_trusted`.
const MAINNET_DAPI_ADDRESSES: &str = concat!(
    "https://149.28.241.190:443,",
    "https://198.7.115.48:443,",
    "https://134.255.182.186:443,",
    "https://93.115.172.39:443,",
    "https://5.189.164.253:443,",
    "https://178.215.237.134:443,",
    "https://157.66.81.162:443,",
    "https://173.212.232.90:443",
);

/// Hardcoded withdrawals system contract ID (base58:
/// `4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6`).
const WITHDRAWALS_CONTRACT_ID_BYTES: [u8; 32] = [
    54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109, 81, 24, 11,
    216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127,
];

fn withdrawals_contract_id() -> Identifier {
    Identifier(IdentifierBytes32(WITHDRAWALS_CONTRACT_ID_BYTES))
}

async fn build_mainnet_sdk() -> dash_sdk::Sdk {
    let address_list: AddressList = MAINNET_DAPI_ADDRESSES
        .parse()
        .expect("parse mainnet address list");

    let context_provider = TrustedHttpContextProvider::new(
        Network::Mainnet,
        None,
        NonZeroUsize::new(100).expect("non-zero cache size"),
    )
    .expect("build trusted context provider for mainnet");

    SdkBuilder::new(address_list)
        .with_network(Network::Mainnet)
        .with_context_provider(context_provider)
        .build()
        .expect("build mainnet SDK")
}

async fn fetch_withdrawals_contract(sdk: &dash_sdk::Sdk) -> Arc<DataContract> {
    Arc::new(
        DataContract::fetch(sdk, withdrawals_contract_id())
            .await
            .expect("fetch withdrawals system data contract")
            .expect("withdrawals contract must exist on mainnet"),
    )
}

fn id_list(docs: &[(Identifier, Option<Document>)]) -> Vec<Identifier> {
    docs.iter().map(|(id, _)| *id).collect()
}

fn id_set(docs: &[(Identifier, Option<Document>)]) -> BTreeSet<Identifier> {
    docs.iter().map(|(id, _)| *id).collect()
}

/// Simple reproducer matching the UI screenshot:
/// `orderBy: [["$ownerId", asc|desc]]` with no `where`.
///
/// On a correctly-working index scan the two queries would return the same
/// document set (just reversed). Issue #2409 claims they do not. This test
/// asserts the buggy behavior (sets differ); it will start failing once
/// the bug is fixed.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn withdrawals_orderby_owner_id_asc_vs_desc() {
    setup_logs();

    let sdk = build_mainnet_sdk().await;
    let contract = fetch_withdrawals_contract(&sdk).await;

    let limit = 100u32;
    let make_query = |ascending: bool| {
        let mut q = DocumentQuery::new(Arc::clone(&contract), "withdrawal")
            .expect("create DocumentQuery for `withdrawal`")
            .with_order_by(OrderClause {
                field: "$ownerId".to_string(),
                ascending,
            });
        q.limit = limit;
        q
    };

    let asc: Vec<(Identifier, Option<Document>)> = Document::fetch_many(&sdk, make_query(true))
        .await
        .expect("fetch asc")
        .into_iter()
        .collect();
    let desc: Vec<(Identifier, Option<Document>)> = Document::fetch_many(&sdk, make_query(false))
        .await
        .expect("fetch desc")
        .into_iter()
        .collect();

    let asc_set = id_set(&asc);
    let desc_set = id_set(&desc);
    let only_asc: Vec<Identifier> = asc_set.difference(&desc_set).copied().collect();
    let only_desc: Vec<Identifier> = desc_set.difference(&asc_set).copied().collect();

    tracing::info!(
        asc_count = asc.len(),
        desc_count = desc.len(),
        asc_ids = ?id_list(&asc),
        desc_ids = ?id_list(&desc),
        only_in_asc = ?only_asc,
        only_in_desc = ?only_desc,
        "orderBy $ownerId asc vs desc"
    );

    assert!(
        !asc.is_empty() && !desc.is_empty(),
        "both asc and desc should return at least one document \
         (asc={} desc={}) — cannot verify issue #2409 on empty data",
        asc.len(),
        desc.len(),
    );

    assert_ne!(
        asc_set,
        desc_set,
        "issue #2409 reproducer: asc and desc returned the same set — \
         either the bug is fixed or mainnet state changed. \
         asc_count={} desc_count={}",
        asc.len(),
        desc.len(),
    );
}

/// Exact query from issue #2409:
///
/// ```js
/// where:   [['transactionIndex', 'in', [0,1,2,3,4,5]], ['status', '>', 0]]
/// orderBy: [['status', order], ['transactionIndex', order]]
/// ```
///
/// Asserts the sets differ between asc and desc (reproducing the bug).
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn withdrawals_orderby_issue_2409_range_plus_in() {
    setup_logs();

    let sdk = build_mainnet_sdk().await;
    let contract = fetch_withdrawals_contract(&sdk).await;

    let transaction_index_in_values: Vec<Value> = (0i64..=5i64).map(Value::I64).collect();
    let limit = 100u32;

    let make_query = |ascending: bool| {
        let mut q = DocumentQuery::new(Arc::clone(&contract), "withdrawal")
            .expect("create DocumentQuery for `withdrawal`")
            .with_where(WhereClause {
                field: "transactionIndex".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(transaction_index_in_values.clone()),
            })
            .with_where(WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::I64(0),
            })
            .with_order_by(OrderClause {
                field: "status".to_string(),
                ascending,
            })
            .with_order_by(OrderClause {
                field: "transactionIndex".to_string(),
                ascending,
            });
        q.limit = limit;
        q
    };

    let asc_result = Document::fetch_many(&sdk, make_query(true)).await;
    let desc_result = Document::fetch_many(&sdk, make_query(false)).await;

    tracing::info!(
        asc_ok = asc_result.is_ok(),
        desc_ok = desc_result.is_ok(),
        "range+in query (issue #2409) results"
    );

    let asc: Vec<(Identifier, Option<Document>)> = asc_result
        .expect("asc query must not error")
        .into_iter()
        .collect();
    let desc: Vec<(Identifier, Option<Document>)> = desc_result
        .expect("desc query must not error")
        .into_iter()
        .collect();

    let asc_set = id_set(&asc);
    let desc_set = id_set(&desc);

    tracing::info!(
        asc_count = asc.len(),
        desc_count = desc.len(),
        asc_ids = ?id_list(&asc),
        desc_ids = ?id_list(&desc),
        only_in_asc = ?asc_set.difference(&desc_set).collect::<Vec<_>>(),
        only_in_desc = ?desc_set.difference(&asc_set).collect::<Vec<_>>(),
        "issue #2409 asc vs desc document sets"
    );

    assert_ne!(
        asc_set,
        desc_set,
        "issue #2409 reproducer: asc and desc returned the same set — \
         bug may be fixed. asc_count={} desc_count={}",
        asc.len(),
        desc.len(),
    );
}
