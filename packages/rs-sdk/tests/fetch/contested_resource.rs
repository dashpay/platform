//! Tests of ContestedResource object
use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::platform_value::Value;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::ContestedResource;

/// Test that we can fetch contested resources
///
/// ## Preconditions
///
/// 1. At least one contested resource exists
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing1",
    ignore = "requires a DPNS name to be registered"
)]
async fn test_contested_resources_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_contested_resources_ok").await;

    let index_name = "parentNameAndLabel";

    let query = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: None,
        order_ascending: false,
    };

    let rss = ContestedResource::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");
    tracing::debug!(contested_resources=?rss, "Contested resources");
    assert!(!rss.0.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing1",
    ignore = "requires a DPNS name to be registered"
)]
/// Test pagination
///
/// ## Preconditions
///
/// 1. Multiple contested resources exists
async fn contested_resources_paginate() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("contested_resources_paginate").await;
    // Given all contested resources sorted ascending
    let index_name = "parentNameAndLabel";

    let query_all = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: None,
        order_ascending: true,
    };

    let all = ContestedResource::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch contested resources");

    tracing::debug!(contested_resources=?all, "All contested resources");
    // when I set start_at_value to some value,

    // for key,val in all
    for (i, start) in all.0.iter().enumerate() {
        if i != 2 {
            continue;
        }
        let start_vec = start
            .encode_to_vec(sdk.version())
            .expect("encode current value");

        let query = VotePollsByDocumentTypeQuery {
            start_at_value: Some((start_vec, false)),
            ..query_all.clone()
        };

        let rss = ContestedResource::fetch_many(&sdk, query)
            .await
            .expect("fetch contested resources");
        tracing::debug!(?start, contested_resources=?rss, "Contested resources");
        assert!(!rss.0.is_empty());

        for (j, fetched) in rss.0.into_iter().enumerate() {
            let all_index = i + j; // we fetch exclusive

            assert_eq!(
                fetched,
                (all.0[all_index]),
                "when starting with {:?}, fetched element {} ({:?}) must equal all element {} ({:?})",
                start,
                j,
                fetched,
                all_index,
                all.0[all_index]
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing1",
    ignore = "requires at least 3 DPNS name contests"
)]

async fn contested_resources_limit() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("contested_resources_limit").await;

    // Given index with more than 2 contested resources
    const LIMIT: u16 = 2;
    const LIMIT_ALL: u16 = 100;
    let index_name = "parentNameAndLabel";

    // ... and number of all contested resources
    let query_all = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: Some(LIMIT_ALL),
        order_ascending: false,
    };
    let count_all = ContestedResource::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch contested resources")
        .0
        .len() as u16;

    // When we query for 2 contested values at a time, we get all of them
    let mut i = 0;
    let mut start_at_value = None;
    while i < count_all && i < LIMIT_ALL {
        let query = VotePollsByDocumentTypeQuery {
            limit: Some(LIMIT),
            start_at_value,
            ..query_all.clone()
        };

        let rss = ContestedResource::fetch_many(&sdk, query)
            .await
            .expect("fetch contested resources");
        tracing::debug!(contested_resources=?rss, "Contested resources");
        let length = rss.0.len();
        let expected = if i + LIMIT > count_all {
            count_all - i
        } else {
            LIMIT
        };
        assert_eq!(length, expected as usize);

        let ContestedResource::Value(last) = rss.0.last().expect("last contested resource");
        let last_value = dpp::bincode::encode_to_vec(last, dpp::bincode::config::standard())
            .expect("encode last value");
        start_at_value = Some((last_value, false));
        tracing::debug!(contested_resources=?rss, i, "Contested resources");

        i += length as u16;
    }

    assert_eq!(i, count_all, "all contested resources fetched");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]

/// Empty string in start_index_values should not cause an error
async fn test_contested_resources_idx_value_empty_string() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg
        .setup_api("test_contested_resources_idx_value_empty_string")
        .await;

    // Given an empty string as index value
    let index_name = "parentNameAndLabel";
    let index_value = Value::Text("".to_string());

    // When I send a VotePollsByDocumentTypeQuery with this index value
    let query = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        // start_index_values: vec![], // Value(Text("dash")), Value(Text(""))])
        start_index_values: vec![index_value],
        end_index_values: vec![],
        limit: None,
        order_ascending: false,
    };
    let result = ContestedResource::fetch_many(&sdk, query).await;
    tracing::debug!(contested_resources=?result, "Contested resources");

    // Then I don't get an error
    assert!(result.is_ok(), "empty index value shall not cause an error");
}
