//! Test GetContestedResourceIdentityVotesRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::voting::votes::resource_vote::ResourceVote;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;

/// Given some data contract ID, document type and document ID, when I fetch it, then I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_identity_votes_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_identity_votes_not_found")
        .await;

    let query = ContestedResourceVotesGivenByIdentityQuery {
        identity_id: cfg.existing_identity_id,
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
    };

    let votes = ResourceVote::fetch_many(&sdk, query)
        .await
        .expect("fetch votes for identity");

    assert!(votes.is_empty(), "no votes expected for this query");
}
