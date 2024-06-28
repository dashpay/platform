//! Tests of ContestedResource object

use crate::fetch::{common::setup_logs, config::Config};
use core::panic;
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
use std::panic::catch_unwind;

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
    check_mn_voting_prerequisities(&cfg)
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
    feature = "network-testing",
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
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resources_fields() {
    setup_logs();

    type MutFn = fn(&mut VotePollsByDocumentTypeQuery);
    struct TestCase {
        name: &'static str,
        query_mut_fn: MutFn,
        expect: Result<&'static str, &'static str>,
    }

    let test_cases: Vec<TestCase> = vec![
        TestCase {
            name: "unmodified base query is Ok",
            query_mut_fn: |_q| {},
            expect: Ok("ContestedResources([Value(Text("),
        },
        TestCase {
            name: "index value empty string is Ok",
            query_mut_fn: |q| q.start_index_values = vec![Value::Text("".to_string())],
            expect: Ok(""),
        },
        TestCase {
            name: "non existing document type returns InvalidArgument",
            query_mut_fn: |q| q.document_type_name = "some random non-existing name".to_string(),
            expect: Err(
                r#"code: InvalidArgument, message: "document type some random non-existing name not found"#,
            ),
        },
        TestCase {
            name: "non existing index returns InvalidArgument",
            query_mut_fn: |q| q.index_name = "nx index".to_string(),
            expect: Err(
                r#"code: InvalidArgument, message: "index with name nx index is not the contested index"#,
            ),
        },
        TestCase {
            name: "existing non-contested index returns InvalidArgument",
            query_mut_fn: |q| q.index_name = "dashIdentityId".to_string(),
            expect: Err(
                r#"code: InvalidArgument, message: "index with name dashIdentityId is not the contested index"#,
            ),
        },
        TestCase {
            name: "start_at_value wrong index type returns InvalidArgument PLAN-653",
            query_mut_fn: |q| q.start_at_value = Some((Value::Array(vec![]), true)),
            expect: Err(r#"code: InvalidArgument"#),
        },
        TestCase {
            name: "start_index_values empty vec returns top-level keys",
            query_mut_fn: |q| q.start_index_values = vec![],
            expect: Ok(r#"ContestedResources([Value(Text("dash"))])"#),
        },
        TestCase {
            name: "start_index_values empty string returns zero results",
            query_mut_fn: |q| q.start_index_values = vec![Value::Text("".to_string())],
            expect: Ok(r#"ContestedResources([])"#),
        },
        TestCase {
            name: "start_index_values with two values returns error",
            query_mut_fn: |q| {
                q.start_index_values = vec![
                    Value::Text("dash".to_string()),
                    Value::Text("dada".to_string()),
                ]
            },
            expect: Err("incorrect index values error: too many start index values were provided, since no end index values were provided, the start index values must be less than the amount of properties in the contested index"),
        },
        TestCase {
            name: "end_index_values one value with empty start_index_values returns 'dash'",
            query_mut_fn: |q| {
                q.start_index_values = vec![];
                q.end_index_values = vec![Value::Text("dada".to_string())];
            },
            expect:Ok(r#"ContestedResources([Value(Text("dash"))])"#),
        },
        TestCase {
            name: "end_index_values two values (1 nx) with empty start_index_values returns error",
            query_mut_fn: |q| {
                q.start_index_values = vec![];
                q.end_index_values = vec![Value::Text("dada".to_string()), Value::Text("non existing".to_string())];
            },
            expect:Err("too many end index values were provided"),
        },
        TestCase {
            name: "end_index_values with 1 nx value 'aaa*' and empty start_index_values returns zero objects",
            query_mut_fn: |q| {
                q.start_index_values = vec![];
                q.end_index_values = vec![Value::Text("aaa non existing".to_string())];
            },
            expect:Ok(r#"ContestedResources([])"#),
        },
        TestCase {
            name: "end_index_values with 1 nx value 'zzz*' and empty start_index_values returns zero objects",
            query_mut_fn: |q| {
                q.start_index_values = vec![];
                q.end_index_values = vec![Value::Text("zzz non existing".to_string())];
            },
            expect:Ok(r#"ContestedResources([])"#),
        },
        TestCase {
            // fails due to PLAN-662
            name: "too many items in start_index_values returns error",
            query_mut_fn: |q| {
                q.start_index_values = vec![
                    Value::Text("dash".to_string()),
                    Value::Text("dada".to_string()),
                    Value::Text("eee".to_string()),
                ]
            },
            expect: Err("incorrect index values error: too many start index values were provided, since no end index values were provided, the start index values must be less than the amount of properties in the contested index"),
        },
        TestCase {
            name: "Both start_ and end_index_values returns error",
            query_mut_fn: |q| {
                q.end_index_values = vec![Value::Text("zzz non existing".to_string())]
            },
            expect: Err("incorrect index values error: too many end index values were provided"),
        },
        TestCase {
            name: "Non-existing end_index_values returns error",
            query_mut_fn: |q| {
                q.start_index_values = vec![];
                q.end_index_values = vec![Value::Text("zzz non existing".to_string())]
            },
            expect:Ok("ContestedResources([])"),
        },
        TestCase {
            name: "wrong type of end_index_values should return InvalidArgument",
            query_mut_fn: |q| q.end_index_values = vec![Value::Array(vec![0.into(), 1.into()])],
            expect: Err("incorrect index values error: too many end index values were provided"),
        },
        TestCase {
            name: "limit 0 returns InvalidArgument",
            query_mut_fn: |q| q.limit = Some(0),
            expect: Err(r#"code: InvalidArgument"#),
        },
        TestCase {
            name: "limit std::u16::MAX returns InvalidArgument",
            query_mut_fn: |q| q.limit = Some(std::u16::MAX),
            expect: Err(r#"code: InvalidArgument"#),
        },
        TestCase{
            name: "exact match query returns one object PLAN-656",
            query_mut_fn: |q| {
                q.start_index_values = vec![Value::Text("dash".to_string())];
                q.start_at_value = Some((Value::Text("dada".to_string()), true));
                q.limit = Some(1);
            },
            expect: Ok(r#"ContestedResources([Value(Text("dada"))])"#),

        }
        // start index + start at + limit 1
    ];

    let cfg = Config::new();

    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let base_query = VotePollsByDocumentTypeQuery {
        contract_id: cfg.existing_data_contract_id,
        document_type_name: cfg.existing_document_type_name.clone(),
        index_name: "parentNameAndLabel".to_string(),
        start_at_value: None,
        start_index_values: vec![Value::Text("dash".to_string())],
        end_index_values: vec![],
        limit: None,
        order_ascending: false,
    };

    // check if the base query works
    let base_query_sdk = cfg.setup_api("contested_resources_fields_base_query").await;
    let result = ContestedResource::fetch_many(&base_query_sdk, base_query.clone()).await;
    assert!(
        result.is_ok_and(|v| !v.0.is_empty()),
        "base query should return some results"
    );

    let mut failures: Vec<(&'static str, String)> = Default::default();

    for test_case in test_cases {
        tracing::debug!("Running test case: {}", test_case.name);
        // handle panics to not stop other test cases from running
        let unwinded = catch_unwind(|| {
            {
                pollster::block_on(async {
                    // create new sdk to ensure that test cases don't interfere with each other
                    let sdk = cfg
                        .setup_api(&format!("contested_resources_fields_{}", test_case.name))
                        .await;

                    let mut query = base_query.clone();
                    (test_case.query_mut_fn)(&mut query);

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

                failures.push((test_case.name, format!("PANIC: {}", msg)));
                continue; // continue to next test case
            }
        };

        match test_case.expect {
            Ok(expected) if result.is_ok() => {
                let result_string = format!("{:?}", result.as_ref().expect("result"));
                if !result_string.contains(expected) {
                    failures.push((
                        test_case.name,
                        format!("EXPECTED: {} GOT: {:?}\n", expected, result),
                    ));
                }
            }
            Err(expected) if result.is_err() => {
                let result = result.expect_err("error");
                if !result.to_string().contains(expected) {
                    failures.push((
                        test_case.name,
                        format!("EXPECTED: {} GOT: {:?}\n", expected, result),
                    ));
                }
            }
            expected => {
                failures.push((
                    test_case.name,
                    format!("EXPECTED: {:?} GOT: {:?}\n", expected, result),
                ));
            }
        }
    }
    if !failures.is_empty() {
        for failure in &failures {
            tracing::error!(?failure, "Failed: {}", failure.0);
        }
        let failed_cases = failures
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<String>>()
            .join("\n* ");

        panic!(
            "{} test cases failed:\n* {}\n\n{}\n",
            failures.len(),
            failed_cases,
            failures
                .iter()
                .map(|(name, msg)| format!("===========================\n{}:\n\n{:?}", name, msg))
                .collect::<Vec<String>>()
                .join("\n")
        );
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
                Value::Text(INDEX_VALUE.to_string()),
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
