//! Test GetContestedResourceIdentityVotesRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::{
    dashcore::ProTxHash, identifier::Identifier, voting::votes::resource_vote::ResourceVote,
};
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;

/// When we request votes for a non-existing identity, we should get no votes.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_identity_votes_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_identity_votes_not_found")
        .await;

    // Given some non-existing identity ID
    let identity_id = Identifier::new([0xff; 32]);

    // When I query for votes given by this identity
    let query = ContestedResourceVotesGivenByIdentityQuery {
        identity_id,
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
    };
    let votes = ResourceVote::fetch_many(&sdk, query)
        .await
        .expect("fetch votes for identity");

    // Then I get no votes
    assert!(votes.is_empty(), "no votes expected for this query");
}

/// When we request votes for an existing identity, we should get some votes.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_identity_votes_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_identity_votes_ok").await;

    // Given some existing identity ID, that is, proTxHash of some Validator
    // TODO: Fetch proTxHash from the network
    let protx_hex = "7624E7D0D7C8837D4D02A19700F4116091A8AD145352420193DE8828F6D00BBF";
    let protx = ProTxHash::from_hex(protx_hex).expect("ProTxHash from hex");

    // When I query for votes given by this identity
    let votes = ResourceVote::fetch_many(&sdk, protx)
        .await
        .expect("fetch votes for identity");

    // Then I get some votes
    assert!(!votes.is_empty(), "votes expected for this query");
}
