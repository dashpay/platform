//! Tests for SDK requests that return one or more [Contender] objects.
use crate::fetch::{
    common::{setup_logs, setup_sdk_for_test_case, TEST_DPNS_NAME},
    config::Config,
    contested_resource::check_mn_voting_prerequisities,
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
use test_case::test_case;

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
            index_values: vec!["nx".into(), label.into()],
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
/// 1. There must be at least one contender for name "dash" and value "[TEST_DPNS_NAME]".
///
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_vote_states_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_vote_states_ok").await;
    // Given some existing data contract and existing label

    let query = base_query(&cfg);

    let data_contract_id = query.vote_poll.contract_id;
    let document_type_name = &query.vote_poll.document_type_name;

    let data_contract = DataContract::fetch_by_identifier(&sdk, data_contract_id)
        .await
        .expect("fetch data contract")
        .expect("found data contract");
    let document_type = data_contract
        .document_type_for_name(document_type_name)
        .expect("found document type");

    // When I query for vote poll states with existing index values

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
        assert_eq!(properties["label"], Value::Text(TEST_DPNS_NAME.into()));
        tracing::debug!(?properties, "document properties");
    }
}

fn base_query(cfg: &Config) -> ContestedDocumentVotePollDriveQuery {
    let index_value_2 = Value::Text(convert_to_homograph_safe_chars(TEST_DPNS_NAME));

    ContestedDocumentVotePollDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        vote_poll: ContestedDocumentResourceVotePoll {
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![Value::Text("dash".into()), index_value_2],
            document_type_name: cfg.existing_document_type_name.clone(),
            contract_id: cfg.existing_data_contract_id,
        },
        allow_include_locked_and_abstaining_vote_tally: true,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    }
}

/// Ensure we can limit the number of returned contenders.
///
/// ## Preconditions
///
/// 1. There must be at least 3 condenders for name "dash" and value [TEST_DPNS_NAME].
///
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
#[allow(non_snake_case)]
async fn contested_resource_vote_states_with_limit_PLAN_674() {
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
    let label = Value::Text(TEST_DPNS_NAME.into());

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
        .contenders;

    tracing::debug!(?all_contenders, "All contenders");

    assert!(
        all_contenders.len() > limit as usize,
        "we need more than {} contenders for this test",
        limit
    );

    // When I query for vote poll states with a limit
    let query = ContestedDocumentVotePollDriveQuery {
        limit: Some(limit),
        ..query_all
    };

    let contenders = ContenderWithSerializedDocument::fetch_many(&sdk, query.clone())
        .await
        .expect("fetch many contenders");
    // Then I get no more than the limit of contenders
    tracing::debug!(contenders=?contenders, ?query, "Contenders");

    assert_eq!(
        contenders.contenders.len(),
        limit as usize,
        "number of contenders for {:?} should must be at least {}",
        label,
        limit
    );
}

type MutFn = fn(&mut ContestedDocumentVotePollDriveQuery);

#[test_case(|q| q.limit = Some(0), Err("limit 0 out of bounds of [1, 100]"); "limit 0")]
#[test_case(|q| q.limit = Some(std::u16::MAX), Err("limit 65535 out of bounds of [1, 100]"); "limit std::u16::MAX")]
#[test_case(|q| q.start_at = Some(([0x11; 32], true)), Ok("Contenders { contenders: {Identifier("); "start_at does not exist should return next contenders")]
#[test_case(|q| q.start_at = Some(([0xff; 32], true)), Ok("Contenders { contenders: {}, abstain_vote_tally: None, lock_vote_tally: None }"); "start_at 0xff;32 should return zero contenders")]
#[test_case(|q| q.vote_poll.document_type_name = "nx doctype".to_string(), Err(r#"code: InvalidArgument, message: "document type nx doctype not found"#); "non existing document type returns InvalidArgument")]
#[test_case(|q| q.vote_poll.index_name = "nx index".to_string(), Err(r#"code: InvalidArgument, message: "index with name nx index is not the contested index"#); "non existing index returns InvalidArgument")]
#[test_case(|q| q.vote_poll.index_name = "dashIdentityId".to_string(), Err(r#"code: InvalidArgument, message: "index with name dashIdentityId is not the contested index"#); "existing non-contested index returns InvalidArgument")]
#[test_case(|q| q.vote_poll.index_values = vec![], Err("query uses index parentNameAndLabel, this index has 2 properties, but the query provided 0 index values instead"); "index_values empty vec returns error")]
#[test_case(|q| q.vote_poll.index_values = vec![Value::Text("".to_string())], Err("query uses index parentNameAndLabel, this index has 2 properties, but the query provided 1 index values instead"); "index_values empty string returns error")]
#[test_case(|q| q.vote_poll.index_values = vec![Value::Text("dash".to_string())], Err("query uses index parentNameAndLabel, this index has 2 properties, but the query provided 1 index values instead"); "index_values with one value returns error")]
#[test_case(|q| {
    q.vote_poll.index_values = vec![
        Value::Text("dash".to_string()),
        Value::Text(TEST_DPNS_NAME.to_string()),
    ]
}, Ok("contenders: {Identifier("); "index_values with two values returns contenders")]
#[test_case(|q| {
    q.vote_poll.index_values = vec![
        Value::Text("dash".to_string()),
        Value::Text(TEST_DPNS_NAME.to_string()),
        Value::Text("eee".to_string()),
    ]
}, Err("query uses index parentNameAndLabel, this index has 2 properties, but the query provided 3 index values instead"); "index_values too many items should return error")]
#[test_case(|q| q.vote_poll.contract_id = Identifier::from([0xff; 32]), Err(r#"InvalidArgument, message: "contract not found error"#); "invalid contract id should cause InvalidArgument error")]
#[test_case(|q| q.allow_include_locked_and_abstaining_vote_tally = false, Ok(r#"contenders: {Identifier(IdentifierBytes32"#); "allow_include_locked_and_abstaining_vote_tally false should return some contenders")]
#[test_case(|q| {
    q.result_type = ContestedDocumentVotePollDriveQueryResultType::Documents
}, Ok(r#"]), vote_tally: None })"#); "result_type Documents")]
#[test_case(|q| {
    q.result_type = ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
}, Ok(r#"]), vote_tally: Some("#); "result_type DocumentsAndVoteTally")]
#[test_case(|q| {
    q.result_type = ContestedDocumentVotePollDriveQueryResultType::VoteTally
}, Ok(r#"serialized_document: None, vote_tally: Some"#); "result_type VoteTally")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn contested_rss_vote_state_fields(
    query_mut_fn: MutFn,
    expect: Result<&'static str, &'static str>,
) -> Result<(), String> {
    setup_logs();

    let cfg = Config::new();
    check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let mut query = base_query(&cfg);
    query_mut_fn(&mut query);
    let (test_case_id, sdk) =
        setup_sdk_for_test_case(cfg, query.clone(), "contested_rss_vote_state_fields_").await;

    tracing::debug!(test_case_id, ?query, "Executing test case query");

    let result = ContenderWithSerializedDocument::fetch_many(&sdk, query).await;
    tracing::debug!(?result, "Result of test case");
    match expect {
        Ok(expected) if result.is_ok() => {
            let result_string = format!("{:?}", result.as_ref().expect("result"));
            if !result_string.contains(expected) {
                Err(format!("expected: {:#?}\ngot: {:?}\n", expected, result))
            } else {
                Ok(())
            }
        }
        Err(expected) if result.is_err() => {
            let result = result.expect_err("error");
            if !result.to_string().contains(expected) {
                Err(format!("expected: {:#?}\ngot {:?}\n", expected, result))
            } else {
                Ok(())
            }
        }
        expected => Err(format!("expected: {:#?}\ngot: {:?}\n", expected, result)),
    }
}
