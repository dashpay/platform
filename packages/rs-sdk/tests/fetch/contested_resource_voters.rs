//! Test GetContestedResourceVotersForIdentityRequest

use dash_sdk::platform::FetchMany;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive_proof_verifier::types::Voter;

use crate::fetch::{common::setup_logs, config::Config};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_contested_resource_voters_for_identity_not_found() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg
        .setup_api("test_contested_resource_voters_for_identity_not_found")
        .await;

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
          contestant_id: cfg.existing_identity_id,
    };

    let rss = Voter::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");

    assert!(rss.0.is_empty());
}
