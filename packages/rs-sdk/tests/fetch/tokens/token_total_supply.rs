use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use assert_matches::assert_matches;
use dash_sdk::platform::Fetch;
use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;

/// Fetches total supply of a single token
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_token_total_supply() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_token_total_supply").await;

    let balance = TotalSingleTokenBalance::fetch(&sdk, *TOKEN_ID_0)
        .await
        .expect("fetch identity token infos");

    assert_matches!(
        balance,
        Some(TotalSingleTokenBalance {
            token_supply: 100200,
            aggregated_token_account_balances: 100200,
        })
    );
}
