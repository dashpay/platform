//! Test GetPrefundedSpecializedBalanceRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::{Fetch, FetchMany};
use dpp::{identifier::Identifier, voting::vote_polls::VotePoll};
use drive::query::VotePollsByEndDateDriveQuery;
use drive_proof_verifier::types::PrefundedSpecializedBalance;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_prefunded_specialized_balance_not_found() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg
        .setup_api("test_prefunded_specialized_balance_not_found")
        .await;

    let query = Identifier::from_bytes(&[1u8; 32]).expect("create identifier");

    let rss = PrefundedSpecializedBalance::fetch(&sdk, query)
        .await
        .expect("fetch prefunded specialized balance");

    assert!(rss.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn test_prefunded_specialized_balance_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_prefunded_specialized_balance_ok").await;

    // Given some vote poll
    let query = VotePollsByEndDateDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_time: None,
        end_time: None,
    };

    let polls = VotePoll::fetch_many(&sdk, query)
        .await
        .expect("fetch vote polls");
    tracing::debug!("vote polls retrieved: {:?}", polls);

    let poll = polls
        .0
        .first()
        .expect("need at least one vote poll timestamp")
        .1
        .first()
        .expect("need at least one vote poll");

    // Vote poll specialized balance ID
    let balance_id = poll
        .specialized_balance_id()
        .expect("vote poll specialized balance ID")
        .expect("must have specialized balance ID");

    let balance = PrefundedSpecializedBalance::fetch(&sdk, balance_id)
        .await
        .expect("fetch prefunded specialized balance")
        .expect("prefunded specialized balance expected for this query");

    tracing::debug!(balance=?balance, "Prefunded specialized balance");

    assert!(
        balance.to_credits() > 0,
        "prefunded specialized balance expected for this query"
    );
}
