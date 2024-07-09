//! Tests of ContestedResource object

use crate::fetch::{
    common::{setup_logs, setup_sdk_for_test_case, TEST_DPNS_NAME},
    config::Config,
};
use dash_sdk::{platform::FetchMany, Error};
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
use std::panic::catch_unwind;

/// Test that we can fetch contested resources
///
/// ## Preconditions
///
/// 1. At least one contested resource (DPNS name) exists
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn test_contested_resources_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_contested_resources_ok").await;
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let query = base_query(&cfg);

    let rss = ContestedResource::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");
    tracing::debug!(contested_resources=?rss, "Contested resources");
    assert!(!rss.0.is_empty());
}

fn base_query(cfg: &Config) -> VotePollsByDocumentTypeQuery {
    VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: "parentNameAndLabel".to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".to_string())],
        end_index_values: vec![],
        limit: None,
        order_ascending: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
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
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    // Given all contested resources sorted ascending
    let index_name = "parentNameAndLabel";
    for order_ascending in [true, false] {
        let query_all = VotePollsByDocumentTypeQuery {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name.clone(),
            index_name: index_name.to_string(),
            start_at_value: None,
            start_index_values: vec![Value::Text("dash".into())],
            end_index_values: vec![],
            limit: Some(50),
            order_ascending,
        };

        let all = ContestedResource::fetch_many(&sdk, query_all.clone())
            .await
            .expect("fetch contested resources");

        tracing::debug!(contested_resources=?all, order_ascending, "All contested resources");
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
                "when starting with {:?} order ascending {} with inclusive {}, fetched element {} ({:?}) must equal all element {} ({:?})",
                start,
                order_ascending,
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
}

/// Test that we can fetch contested resources with a limit
///
/// ## Preconditions
///
/// 1. At least 3 contested resources (eg. different DPNS names) exist
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
#[allow(non_snake_case)]
async fn contested_resources_limit_PLAN_656() {
    // TODO: fails due to PLAN-656, not tested enough so it can be faulty
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resources_limit").await;
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    const LIMIT: u16 = 2;
    const LIMIT_ALL: u16 = 100;
    let index_name = "parentNameAndLabel";

    for order_ascending in [true, false] {
        let query_all = VotePollsByDocumentTypeQuery {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name.clone(),
            index_name: index_name.to_string(),
            start_at_value: None,
            start_index_values: vec![Value::Text("dash".into())],
            end_index_values: vec![],
            limit: Some(LIMIT_ALL),
            order_ascending,
        };
        let all = ContestedResource::fetch_many(&sdk, query_all.clone())
            .await
            .expect("fetch contested resources");
        let count_all = all.0.len() as u16;

        // When we query for 2 contested values at a time, we get all of them
        let mut i = 0;
        let mut start_at_value = None;
        while i < count_all && i < LIMIT_ALL {
            let query = VotePollsByDocumentTypeQuery {
                limit: Some(LIMIT),
                start_at_value,
                order_ascending,
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

            for (j, fetched) in rss.0.iter().enumerate() {
                let all_index = i + j as u16;
                assert_eq!(
                    fetched,
                    &(all.0[all_index as usize]),
                    "fetched element {} ({:?}) must equal all element {} ({:?}) when ascending {}",
                    j,
                    fetched,
                    all_index,
                    all.0[all_index as usize],
                    order_ascending,
                );
            }

            let ContestedResource::Value(last) =
                rss.0.into_iter().last().expect("last contested resource");
            start_at_value = Some((last, false));

            i += length as u16;
        }
        assert_eq!(i, count_all, "all contested resources fetched");
    }
}
/// Check various queries for [ContestedResource] that contain invalid field values
///
/// ## Preconditions
///
/// None
#[test_case::test_case(|_q| {}, Ok("ContestedResources([Value(Text(".into()); "unmodified base query is Ok")]
#[test_case::test_case(|q| q.start_index_values = vec![Value::Text("".to_string())], Ok("".into()); "index value empty string is Ok")]
#[test_case::test_case(|q| q.document_type_name = "some random non-existing name".to_string(), Err(r#"code: InvalidArgument, message: "document type some random non-existing name not found"#); "non existing document type returns InvalidArgument")]
#[test_case::test_case(|q| q.index_name = "nx index".to_string(), Err(r#"code: InvalidArgument, message: "index with name nx index is not the contested index"#); "non existing index returns InvalidArgument")]
#[test_case::test_case(|q| q.index_name = "dashIdentityId".to_string(), Err(r#"code: InvalidArgument, message: "index with name dashIdentityId is not the contested index"#); "existing non-contested index returns InvalidArgument")]
// Disabled due to bug PLAN-653
// #[test_case::test_case(|q| q.start_at_value = Some((Value::Array(vec![]), true)), Err(r#"code: InvalidArgument"#); "start_at_value wrong index type returns InvalidArgument PLAN-653")]
#[test_case::test_case(|q| q.start_index_values = vec![], Ok(r#"ContestedResources([Value(Text("dash"))])"#.into()); "start_index_values empty vec returns top-level keys")]
#[test_case::test_case(|q| q.start_index_values = vec![Value::Text("".to_string())], Ok(r#"ContestedResources([])"#.into()); "start_index_values empty string returns zero results")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![
        Value::Text("dash".to_string()),
        Value::Text(TEST_DPNS_NAME.to_string()),
    ]
}, Err("incorrect index values error: too many start index values were provided, since no end index values were provided, the start index values must be less than the amount of properties in the contested index"); "start_index_values with two values returns error")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![];
    q.end_index_values = vec![Value::Text(TEST_DPNS_NAME.to_string())];
}, Ok(r#"ContestedResources([Value(Text("dash"))])"#.into()); "end_index_values one value with empty start_index_values returns 'dash'")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![];
    q.end_index_values = vec![Value::Text(TEST_DPNS_NAME.to_string()), Value::Text("non existing".to_string())];
}, Err("too many end index values were provided"); "end_index_values two values (1 nx) with empty start_index_values returns error")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![];
    q.end_index_values = vec![Value::Text("aaa non existing".to_string())];
}, Ok(r#"ContestedResources([])"#.into()); "end_index_values with 1 nx value 'aaa*' and empty start_index_values returns zero objects")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![];
    q.end_index_values = vec![Value::Text("zzz non existing".to_string())];
}, Ok(r#"ContestedResources([])"#.into()); "end_index_values with 1 nx value 'zzz*' and empty start_index_values returns zero objects")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![
        Value::Text("dash".to_string()),
        Value::Text(TEST_DPNS_NAME.to_string()),
        Value::Text("eee".to_string()),
    ]
}, Err("incorrect index values error: too many start index values were provided, since no end index values were provided, the start index values must be less than the amount of properties in the contested index"); "too many items in start_index_values returns error")]
#[test_case::test_case(|q| {
    q.end_index_values = vec![Value::Text("zzz non existing".to_string())]
}, Err("incorrect index values error: too many end index values were provided"); "Both start_ and end_index_values returns error")]
#[test_case::test_case(|q| {
    q.start_index_values = vec![];
    q.end_index_values = vec![Value::Text("zzz non existing".to_string())]
}, Ok("ContestedResources([])".into()); "Non-existing end_index_values returns error")]
#[test_case::test_case(|q| q.end_index_values = vec![Value::Array(vec![0.into(), 1.into()])], Err("incorrect index values error: too many end index values were provided"); "wrong type of end_index_values should return InvalidArgument")]
#[test_case::test_case(|q| q.limit = Some(0), Err(r#"code: InvalidArgument"#); "limit 0 returns InvalidArgument")]
#[test_case::test_case(|q| q.limit = Some(std::u16::MAX), Err(r#"code: InvalidArgument"#); "limit std::u16::MAX returns InvalidArgument")]
// Disabled due to bug PLAN-656
// #[test_case::test_case(|q| {
//     q.start_index_values = vec![Value::Text("dash".to_string())];
//     q.start_at_value = Some((Value::Text(TEST_DPNS_NAME.to_string()), true));
//     q.limit = Some(1);
// }, Ok(format!(r#"ContestedResources([Value(Text({}))])"#, TEST_DPNS_NAME)); "exact match query returns one object PLAN-656")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn contested_resources_fields(
    query_mut_fn: fn(&mut VotePollsByDocumentTypeQuery),
    expect: Result<String, &'static str>,
) -> Result<(), String> {
    setup_logs();

    let cfg = Config::new();

    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    tracing::debug!(?expect, "Running test case");
    // handle panics to not stop other test cases from running
    let unwinded = catch_unwind(|| {
        {
            pollster::block_on(async {
                let mut query = base_query(&cfg);
                query_mut_fn(&mut query);

                let (test_case_id, sdk) =
                    setup_sdk_for_test_case(cfg, query.clone(), "contested_resources_fields").await;
                tracing::debug!(test_case_id, ?query, "Executing query");

                ContestedResource::fetch_many(&sdk, query).await
            })
        }
    });
    let result = match unwinded {
        Ok(r) => r,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.to_string()
            } else {
                format!("unknown panic type: {:?}", std::any::type_name_of_val(&e))
            };

            tracing::error!("PANIC: {}", msg);
            Err(Error::Generic(msg))
        }
    };

    match expect {
        Ok(expected) if result.is_ok() => {
            let result_string = format!("{:?}", result.as_ref().expect("result"));
            if !result_string.contains(&expected) {
                Err(format!("EXPECTED: {} GOT: {:?}\n", expected, result))
            } else {
                Ok(())
            }
        }
        Err(expected) if result.is_err() => {
            let result = result.expect_err("error");
            if !result.to_string().contains(expected) {
                Err(format!("EXPECTED: {} GOT: {:?}\n", expected, result))
            } else {
                Ok(())
            }
        }
        expected => Err(format!("EXPECTED: {:?} GOT: {:?}\n", expected, result)),
    }
}

/// Ensure prerequsities for masternode voting tests are met
pub async fn check_mn_voting_prerequisities(cfg: &Config) -> Result<(), Vec<String>> {
    let sdk = cfg.setup_api("check_mn_voting_prerequisities").await;
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

    // Check if we have enough contested resources; this implies that we have
    // at least 1 vote poll for each of them
    let contested_resources = ContestedResource::fetch_many(&sdk, query_contested_resources)
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
                Value::Text(TEST_DPNS_NAME.to_string()),
            ],
            document_type_name: cfg.existing_document_type_name.clone(),
            contract_id: cfg.existing_data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let all_contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch many contenders");
    if all_contenders.contenders.len() < 3 {
        errors.push(format!(
            "Please create 3 identities and create DPNS name `{}` for each of them, found {}",
            TEST_DPNS_NAME,
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
