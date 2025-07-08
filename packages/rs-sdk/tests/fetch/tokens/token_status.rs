use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use assert_matches::assert_matches;
use dash_sdk::platform::FetchMany;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::tokens::status::TokenStatus;

/// Fetches multiple token statuses, including the one, which does not exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_token_status() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_token_status").await;

    let token_statuses = TokenStatus::fetch_many(
        &sdk,
        vec![*TOKEN_ID_0, *TOKEN_ID_1, *TOKEN_ID_2, UNKNOWN_TOKEN_ID],
    )
    .await
    .expect("fetch identity token infos");

    assert_eq!(token_statuses.len(), 4);

    assert_matches!(token_statuses.get(&*TOKEN_ID_0), Some(None));
    assert_matches!(token_statuses.get(&*TOKEN_ID_1), Some(Some(status)) if status.paused());
    assert_matches!(token_statuses.get(&*TOKEN_ID_2), Some(None));
    assert_matches!(token_statuses.get(&UNKNOWN_TOKEN_ID), Some(None));
}
