//! Test GetPrefundedSpecializedBalanceRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::Fetch;
use dpp::identifier::Identifier;
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
