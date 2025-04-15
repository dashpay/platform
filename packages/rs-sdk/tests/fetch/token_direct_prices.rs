use dash_sdk::platform::FetchMany;
use dpp::{prelude::Identifier, tokens::token_pricing_schedule::TokenPricingSchedule};

use crate::fetch::config::Config;

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_token_not_found() {
    super::common::setup_logs();

    pub const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_token_not_found").await;

    let result = TokenPricingSchedule::fetch_many(&sdk, &[id][..])
        .await
        .expect("query should succeed");

    assert!(result.is_empty(), "result should be empty");
}
