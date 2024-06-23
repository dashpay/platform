//! Test GetContestedResourceVotersForIdentityRequest

use dash_sdk::platform::{Fetch, FetchMany};
use dpp::{identifier::Identifier, identity::Identity, platform_value::Value};
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive_proof_verifier::types::Voter;

use crate::fetch::{common::setup_logs, config::Config};

/// When we request votes for a non-existing identity, we should get no votes.
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
/// 1. Votes exist for the given contestant.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_voters_for_existing_contestant() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_voters_for_existing_contestant")
        .await;

    // Given a known contestant ID that has votes
    // TODO: lookup contestant ID
    let contestant_id = Identifier::from_string(
        "D63rWKSagCgEE53XPkouP3swN9n87jHvjesFEZEh1cLr",
        dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .expect("valid contestant ID");

    let index_name = "parentNameAndLabel";
    let index_value = Value::Text("dada".to_string());
    // double-check that the contestant identity exist
    let _contestant_identity = Identity::fetch(&sdk, contestant_id)
        .await
        .expect("fetch identity")
        .expect("contestant identity must exist");

    // When I query for votes given to this contestant
    let query = ContestedDocumentVotePollVotesDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
        vote_poll: dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll {
            contract_id: cfg.existing_data_contract_id,
            document_type_name: cfg.existing_document_type_name,
            index_name: index_name.to_string(),
            index_values: vec!["dash".into(), index_value],
        },
        contestant_id,
    };

    let rss = Voter::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");

    // We expect to find votes for the known contestant
    assert!(
        !rss.0.is_empty(),
        "Expected to find votes for the existing contestant"
    );
}
