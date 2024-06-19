//! Tests of ContestedResource object
use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::{
    platform_value::Value,
    voting::{
        contender_structs::ContenderWithSerializedDocument,
        vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
    },
};
use drive::query::{
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
    vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery,
};
use drive_proof_verifier::types::ContestedResource;

pub(crate) const INDEX_VALUE: &str = "dada";

/// Test that we can fetch contested resources
///
/// ## Preconditions
///
/// 1. At least one contested resource (DPNS name) exists
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing",
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn test_contested_resources_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_contested_resources_ok").await;
    check_mn_voting_prerequisities(&sdk, &cfg)
        .await
        .expect("prerequisities");

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
    feature = "network-testing",
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]

/// Test [ContestedResource] start index (`start_at_value`)
///
/// ## Preconditions
///
/// 1. At least 2 contested resources (eg. different DPNS names) exist
async fn contested_resources_start_at_value() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("contested_resources_start_at_value").await;
    check_mn_voting_prerequisities(&sdk, &cfg)
        .await
        .expect("prerequisities");

    // Given all contested resources sorted ascending
    let index_name = "parentNameAndLabel";

    let query_all = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: Some(50),
        order_ascending: true,
    };

    let all = ContestedResource::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch contested resources");

    tracing::debug!(contested_resources=?all, "All contested resources");
    for inclusive in [true, false] {
        // when I set start_at_value to some value,
        for (i, start) in all.0.iter().enumerate() {
            let ContestedResource::Value(start_value) = start.clone();

            let query = VotePollsByDocumentTypeQuery {
                start_at_value: Some((start_value, inclusive)),
                ..query_all.clone()
            };

            let rss = ContestedResource::fetch_many(&sdk, query)
                .await
                .expect("fetch contested resources");
            tracing::debug!(?start, contested_resources=?rss, "Contested resources");

            for (j, fetched) in rss.0.into_iter().enumerate() {
                let all_index = if inclusive { i + j } else { i + j + 1 };

                assert_eq!(
                fetched,
                (all.0[all_index]),
                "when starting with {:?} with inclusive {}, fetched element {} ({:?}) must equal all element {} ({:?})",
                start,
                inclusive,
                j,
                fetched,
                all_index,
                all.0[all_index]
            );
            }
        }
    }
}

/// Test that we can fetch contested resources with a limit
///
/// ## Preconditions
///
/// 1. At least 3 contested resources (eg. different DPNS names) exist
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    feature = "network-testing",
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn contested_resources_limit() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resources_limit").await;
    check_mn_voting_prerequisities(&sdk, &cfg)
        .await
        .expect("prerequisities");

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
        tracing::debug!(contested_resources=?rss, i, "Contested resources");

        let ContestedResource::Value(last) =
            rss.0.into_iter().last().expect("last contested resource");
        start_at_value = Some((last, false));

        i += length as u16;
    }

    assert_eq!(i, count_all, "all contested resources fetched");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]

/// Empty string in start_index_values should not cause an error
///
/// ## Preconditions
///
/// None
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

/// Ensure prerequsities for masternode voting tests are met
pub async fn check_mn_voting_prerequisities(
    sdk: &dash_sdk::Sdk,
    cfg: &Config,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    let index_name = "parentNameAndLabel".to_string();

    let query_contested_resources = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: index_name.to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".into())],
        end_index_values: vec![],
        limit: None,
        order_ascending: true,
    };

    // Check if we have enough contested resources; this implies that we have at least 1 vote poll
    let contested_resources = ContestedResource::fetch_many(sdk, query_contested_resources)
        .await
        .expect("fetch contested resources");
    if contested_resources.0.len() < 3 {
        errors.push(format!(
            "Please create at least 3 different DPNS names for masternode voting tests, found {}",
            contested_resources.0.len()
        ));
    }

    // ensure we have enough contenders
    let query_all = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".into()),
                Value::Text(INDEX_VALUE.to_string()),
            ],
            document_type_name: cfg.existing_document_type_name.clone(),
            contract_id: cfg.existing_data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let all_contenders = ContenderWithSerializedDocument::fetch_many(sdk, query_all.clone())
        .await
        .expect("fetch many contenders");
    if all_contenders.contenders.len() < 3 {
        errors.push(format!(
            "Please create 3 identities and create DPNS name `{}` for each of them, found {}",
            INDEX_VALUE,
            all_contenders.contenders.len()
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        tracing::error!(?errors, "Prerequisities for masternode voting tests not met, please configure the network accordingly");
        Err(errors)
    }
}
