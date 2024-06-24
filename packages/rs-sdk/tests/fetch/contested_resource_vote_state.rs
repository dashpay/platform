//! Tests for SDK requests that return one or more [Contender] objects.
use crate::fetch::{
    common::setup_logs, config::Config, contested_resource::check_mn_voting_prerequisities,
};
use dash_sdk::platform::{Fetch, FetchMany};
use dpp::{
    data_contract::{accessors::v0::DataContractV0Getters, DataContract},
    document::{
        serialization_traits::DocumentPlatformConversionMethodsV0, Document, DocumentV0Getters,
    },
    identifier::Identifier,
    platform_value::Value,
    util::strings::convert_to_homograph_safe_chars,
    voting::{
        contender_structs::ContenderWithSerializedDocument,
        vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
    },
};
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};

/// Ensure we get proof of non-existence when querying for a non-existing index value.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_vote_states_not_found")
        .await;
    // Given some existing data contract ID and non-existing label
    let data_contract_id = cfg.existing_data_contract_id;
    let label = "non existing name";

    // When I query for vote poll states
    let query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![label.into()],
            document_type_name: cfg.existing_document_type_name,
            contract_id: data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query)
        .await
        .expect("fetch many contenders");
    // Then I get no contenders
    assert!(
        contenders.contenders.is_empty(),
        "no contenders expected for this query"
    );
}

/// Asking for non-existing contract should return error.
///
/// Note: due to the way error handling is implemented, this test will not work
/// correctly in offline mode.
#[cfg_attr(
    feature = "offline-testing",
    ignore = "offline mode does not support this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_nx_contract() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_vote_states_nx_contract")
        .await;

    // Given some non-existing contract ID
    let data_contract_id = Identifier::new([0xff; 32]);

    // When I query for votes referring this contract ID
    let query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec!["dash".into()],
            document_type_name: cfg.existing_document_type_name,
            contract_id: data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        // TODO test other result types
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    // Then I get an error
    let result = if let Err(e) = ContenderWithSerializedDocument::fetch_many(&sdk, query).await {
        e
    } else {
        panic!("asking for non-existing contract should return error.")
    };

    if let dash_sdk::error::Error::DapiClientError(e) = result {
        assert!(
            e.contains(
                "Transport(Status { code: InvalidArgument, message: \"contract not found error"
            ),
            "we should get contract not found error"
        );
    } else {
        panic!("expected 'contract not found' transport error");
    };
}

/// Ensure we can successfully query for existing index values.
///
/// ## Preconditions
///
/// 1. There must be at least one contender for name "dash" and value "dada".
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_vote_states_ok").await;
    // Given some existing data contract and existing label
    let data_contract_id = cfg.existing_data_contract_id;
    let label = Value::Text(convert_to_homograph_safe_chars("dada"));
    let document_type_name = "domain".to_string();

    let data_contract = DataContract::fetch_by_identifier(&sdk, data_contract_id)
        .await
        .expect("fetch data contract")
        .expect("found data contract");
    let document_type = data_contract
        .document_type_for_name(&document_type_name)
        .expect("found document type");

    // When I query for vote poll states with existing index values
    let query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![Value::Text("dash".into()), label],
            document_type_name,
            contract_id: data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query)
        .await
        .expect("fetch many contenders");
    tracing::debug!(contenders=?contenders, "Contenders");
    // Then I get contenders
    assert!(
        !contenders.contenders.is_empty(),
        "contenders expected for this query"
    );

    // verify that the contenders have the expected properties and we don't have duplicates
    let mut seen = std::collections::BTreeSet::new();
    for contender in contenders.contenders {
        let serialized_document = contender
            .1
            .serialized_document()
            .as_ref()
            .expect("serialized doc");

        let doc = Document::from_bytes(serialized_document, document_type, sdk.version())
            .expect("doc from bytes");
        assert!(seen.insert(doc.id()), "duplicate contender");
        let properties = doc.properties();
        assert_eq!(properties["parentDomainName"], Value::Text("dash".into()));
        assert_eq!(properties["label"], Value::Text("dada".into()));
        tracing::debug!(?properties, "document properties");
    }
}

/// Ensure we can limit the number of returned contenders.
///
/// ## Preconditions
///
/// 1. There must be at least 3 condenders for name "dash" and value "dada".
///
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_with_limit() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_vote_states_with_limit")
        .await;
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisites not met");

    // Given more contenders for some `label` than the limit
    let data_contract_id = cfg.existing_data_contract_id;
    let limit: u16 = 2;
    let label = Value::Text("dada".into());

    // ensure we have enough contenders
    let query_all = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![Value::Text("dash".into()), label.clone()],
            document_type_name: cfg.existing_document_type_name,
            contract_id: data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let all_contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch many contenders")
        .contenders
        .len();
    assert!(
        all_contenders > limit as usize,
        "we need more than {} contenders for this test",
        limit
    );

    // When I query for vote poll states with a limit
    let query = ContestedDocumentVotePollDriveQuery {
        limit: Some(limit),
        ..query_all
    };

    let contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query)
        .await
        .expect("fetch many contenders");
    // Then I get no more than the limit of contenders
    tracing::debug!(contenders=?contenders, "Contenders");

    assert_eq!(
        contenders.contenders.len(),
        limit as usize,
        "number of contenders for {:?} should must be at least {}",
        label,
        limit
    );
}

/// Check various queries for [ContenderWithSerializedDocument] that contain invalid field values
///
/// ## Preconditions
///
/// None
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_fields() {
    setup_logs();

    type MutFn = fn(&mut ContestedDocumentVotePollDriveQuery);
    struct TestCase {
        name: &'static str,
        query_mut_fn: MutFn,
        expect: Result<&'static str, &'static str>,
    }

    let test_cases: Vec<TestCase> = vec![
        TestCase {
            name: "limit 0 PLAN-664",
            query_mut_fn: |q| q.limit = Some(0),
            expect: Ok("..."),
        },
        TestCase {
            name: "limit std::u16::MAX PLAN-664",
            query_mut_fn: |q| q.limit = Some(std::u16::MAX),
            expect: Ok("..."),
        },
        TestCase {
            name: "offset not None",
            query_mut_fn: |q| q.offset = Some(1),
            expect: Err(
                r#"Generic("ContestedDocumentVotePollDriveQuery.offset field is internal and must be set to None")"#,
            ),
        },
        TestCase {
            //  TODO: pagination test
            name: "start_at does not exist",
            query_mut_fn: |q| q.start_at = Some(([0x11; 32], true)),
            expect: Ok("Contenders { contenders: {Identifier("),
        },
        TestCase {
            name: "start_at 0xff;32",
            query_mut_fn: |q| q.start_at = Some(([0xff; 32], true)),
            expect: Ok("Contenders { contenders: {Identifier("),
        },
        TestCase {
            name: "non existing document type returns InvalidArgument",
            query_mut_fn: |q| q.vote_poll.document_type_name = "nx doctype".to_string(),
            expect: Err(r#"code: InvalidArgument, message: "document type nx doctype not found"#),
        },
        TestCase {
            name: "non existing index returns InvalidArgument",
            query_mut_fn: |q| q.vote_poll.index_name = "nx index".to_string(),
            expect: Err(
                r#"code: InvalidArgument, message: "index with name nx index is not the contested index"#,
            ),
        },
        TestCase {
            name: "existing non-contested index returns InvalidArgument",
            query_mut_fn: |q| q.vote_poll.index_name = "dashIdentityId".to_string(),
            expect: Err(
                r#"code: InvalidArgument, message: "index with name dashIdentityId is not the contested index"#,
            ),
        },
        TestCase {
            // todo maybe this should fail? or return everything?
            name: "index_values empty vec returns error PLAN-665",
            query_mut_fn: |q| q.vote_poll.index_values = vec![],
            expect: Ok(r#"TODO error"#),
        },
        TestCase {
            name: "index_values empty string returns error PLAN-665",
            query_mut_fn: |q| q.vote_poll.index_values = vec![Value::Text("".to_string())],
            expect: Ok("TODO error"),
        },
        TestCase {
            name: "index_values with one value returns error PLAN-665",
            query_mut_fn: |q| q.vote_poll.index_values = vec![Value::Text("dash".to_string())],
            expect: Ok("TODO error"),
        },
        TestCase {
            name: "index_values with two values returns contenders",
            query_mut_fn: |q| {
                q.vote_poll.index_values = vec![
                    Value::Text("dash".to_string()),
                    Value::Text("dada".to_string()),
                ]
            },
            expect: Ok("contenders: {Identifier("),
        },
        TestCase {
            name: "index_values too many items should return error PLAN-665",
            query_mut_fn: |q| {
                q.vote_poll.index_values = vec![
                    Value::Text("dash".to_string()),
                    Value::Text("dada".to_string()),
                    Value::Text("eee".to_string()),
                ]
            },
            expect: Ok(
                r#"code: InvalidArgument, message: "incorrect index values error: the start index values and the end index"#,
            ),
        },
        TestCase {
            name: "invalid contract id should cause InvalidArgument error",
            query_mut_fn: |q| q.vote_poll.contract_id = Identifier::from([0xff; 32]),
            expect: Err(r#"InvalidArgument, message: "contract not found error"#),
        },
        TestCase {
            name:
                "allow_include_locked_and_abstaining_vote_tally false should return some contenders",
            query_mut_fn: |q| q.allow_include_locked_and_abstaining_vote_tally = false,
            expect: Ok(r#"contenders: {Identifier(IdentifierBytes32"#),
        },
        TestCase {
            name: "result_type Documents",
            query_mut_fn: |q| {
                q.result_type = ContestedDocumentVotePollDriveQueryResultType::Documents
            },
            expect: Ok(r#"]), vote_tally: None })"#),
        },
        TestCase {
            name: "result_type DocumentsAndVoteTally",
            query_mut_fn: |q| {
                q.result_type = ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
            },
            expect: Ok(r#"]), vote_tally: Some("#),
        },
        TestCase {
            name: "result_type VoteTally",
            query_mut_fn: |q| {
                q.result_type = ContestedDocumentVotePollDriveQueryResultType::VoteTally
            },
            expect: Ok(r#"serialized_document: None, vote_tally: Some"#),
        },
    ];

    let cfg = Config::new();
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let base_query = ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![Value::Text("dash".into()), Value::Text("dada".into())],
            document_type_name: cfg.existing_document_type_name.clone(),
            contract_id: cfg.existing_data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    // check if the base query works
    let base_query_sdk = cfg
        .setup_api("contested_resource_vote_states_fields_base_query")
        .await;
    let result =
        ContenderWithSerializedDocument::fetch_many(&base_query_sdk, base_query.clone()).await;
    assert!(
        result.is_ok_and(|v| !v.contenders.is_empty()),
        "base query should return some results"
    );

    let mut failures: Vec<(&'static str, String)> = Default::default();

    for test_case in test_cases {
        tracing::debug!(
            test_case = test_case.name,
            "Running test case: {}",
            test_case.name
        );
        // create new sdk to ensure that test cases don't interfere with each other
        let sdk = cfg
            .setup_api(&format!(
                "contested_resources_vote_states_fields_{}",
                test_case.name
            ))
            .await;

        let mut query = base_query.clone();
        (test_case.query_mut_fn)(&mut query);

        let result = ContenderWithSerializedDocument::fetch_many(&sdk, query).await;
        tracing::debug!(
            test_case = test_case.name,
            ?result,
            "Result of test case {}",
            test_case.name
        );
        match test_case.expect {
            Ok(expected) if result.is_ok() => {
                let result_string = format!("{:?}", result.as_ref().expect("result"));
                if !result_string.contains(expected) {
                    failures.push((
                        test_case.name,
                        format!("expected: {:#?}\ngot: {:?}\n", expected, result),
                    ));
                }
            }
            Err(expected) if result.is_err() => {
                let result = result.expect_err("error");
                if !result.to_string().contains(expected) {
                    failures.push((
                        test_case.name,
                        format!("expected: {:#?}\ngot {:?}\n", expected, result),
                    ));
                }
            }
            expected => {
                failures.push((
                    test_case.name,
                    format!("expected: {:#?}\ngot: {:?}\n", expected, result),
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
