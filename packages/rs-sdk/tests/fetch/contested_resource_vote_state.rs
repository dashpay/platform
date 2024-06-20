//! Tests for SDK requests that return one or more [Contender] objects.
use crate::fetch::{
    common::setup_logs, config::Config, contested_resource::check_mn_voting_prerequisities,
};
use dash_sdk::platform::FetchMany;
use dpp::{
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
        // TODO test other result types
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
    // Given some existing data contract ID and existing label
    let data_contract_id = cfg.existing_data_contract_id;
    // TODO: Lookup the label instead of hardcoding it
    let label = Value::Text(convert_to_homograph_safe_chars("dada"));
    let document_type_name = "domain".into();

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
    check_mn_voting_prerequisities(&sdk, &cfg)
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
