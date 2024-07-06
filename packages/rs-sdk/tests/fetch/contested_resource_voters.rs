//! Test GetContestedResourceVotersForIdentityRequest

use dash_sdk::platform::FetchMany;
use dpp::{
    identifier::Identifier,
    platform_value::{string_encoding::Encoding, Value},
    voting::contender_structs::ContenderWithSerializedDocument,
};
use drive::query::{
    vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery,
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
};
use drive_proof_verifier::types::Voter;

use crate::fetch::{
    common::{setup_logs, TEST_DPNS_NAME},
    config::Config,
};

/// When we request votes for a non-existing identity, we should get no votes.
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see docs of contested_resource_identity_votes_ok()"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_contested_resource_voters_for_identity_not_found() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg
        .setup_api("test_contested_resource_voters_for_identity_not_found")
        .await;

    let contestant_id = Identifier::new([0xff; 32]);
    let index_name = "parentNameAndLabel";

    let query = ContestedDocumentVotePollVotesDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
        vote_poll: dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name,
            index_name: index_name.to_string(),
            index_values: vec!["dash".into()],
        },
          contestant_id,
    };

    let rss = Voter::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");

    assert!(rss.0.is_empty());
}

/// When we request votes for an existing contestant, we should get some votes.
///
/// ## Preconditions
///
/// 1. Votes exist for DPNS name [TEST_DPNS_NAME].
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn contested_resource_voters_for_existing_contestant() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_voters_for_existing_contestant")
        .await;

    super::contested_resource::check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisites");

    let index_name = "parentNameAndLabel".to_string();
    let index_value = Value::Text(TEST_DPNS_NAME.to_string());

    // fetch contestant
    let contestants_query = ContestedDocumentVotePollDriveQuery {
        vote_poll: dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name.clone(),
            index_name:index_name.clone(),
            index_values: vec![Value::Text("dash".into()),index_value.clone()],
        },
        limit: None, // TODO: Change to Some(1) when PLAN-656 is fixed
        offset:None,
        allow_include_locked_and_abstaining_vote_tally:true,
        start_at: None,
        result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
    };

    let contenders = ContenderWithSerializedDocument::fetch_many(&sdk, contestants_query)
        .await
        .expect("fetch contenders");
    let contender_ids = contenders
        .contenders
        .keys()
        .map(|id| id.to_string(Encoding::Base58))
        .collect::<Vec<String>>();
    tracing::debug!(
        contenders = ?contender_ids,
        "contenders for {}",
        &index_value
    );

    let mut votes = 0;

    for contestant in contenders.contenders.keys() {
        let query = ContestedDocumentVotePollVotesDriveQuery {
        limit: None,
        offset: None,
        start_at: None,
        order_ascending: true,
        vote_poll: dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name.clone(),
            index_name: index_name.to_string(),
            index_values: vec!["dash".into(), index_value.clone()],
        },
        contestant_id:*contestant,
    };

        let rss = Voter::fetch_many(&sdk, query)
            .await
            .expect("fetch contested resources");

        tracing::debug!(
            ?rss,
            contender = contestant.to_string(Encoding::Base58),
            "votes retrieved"
        );
        votes += rss.0.len();
    }

    // We expect to find votes for the known contestant
    assert_ne!(
        votes, 0,
        "Expected to find at least one vote for any of the contestants"
    );
}
