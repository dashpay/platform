//! Test VotePollsByEndDateDriveQuery

use dash_sdk::platform::FetchMany;
use dpp::voting::vote_polls::VotePoll;
use drive::query::VotePollsByEndDateDriveQuery;

use crate::fetch::{common::setup_logs, config::Config};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_vote_polls_by_ts_not_found() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("test_vote_polls_by_ts_not_found").await;

    let query = VotePollsByEndDateDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_time: None,
        end_time: None,
    };

    let rss = VotePoll::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");

    assert!(rss.0.is_empty());
}
