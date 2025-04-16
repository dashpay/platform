use dash_sdk::platform::FetchMany;
use dpp::{prelude::Identifier, tokens::token_pricing_schedule::TokenPricingSchedule};

use crate::fetch::{
    config::Config,
    generated_data::{TOKEN_ID_0, TOKEN_ID_1},
};

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_token_not_found() {
    super::common::setup_logs();

    pub const TOKEN_ID_BYTES: [u8; 32] = [1; 32];
    let id = Identifier::from_bytes(&TOKEN_ID_BYTES).expect("parse token id");

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_token_not_found").await;

    let result = TokenPricingSchedule::fetch_many(&sdk, &[id][..])
        .await
        .expect("query should succeed");

    println!("result: {:?}", result);
    assert_eq!(result.len(), 1);
    let first = result.first().expect("result should have first");
    assert_eq!(first.0, &id);
    assert!(first.1.is_none(), "proof of non-existence expected");
}

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_tokens_found_no_prices() {
    super::common::setup_logs();
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_token_not_found").await;

    let ids = [*TOKEN_ID_0, *TOKEN_ID_1];

    let result = TokenPricingSchedule::fetch_many(&sdk, &ids[..])
        .await
        .expect("query should succeed");

    assert_eq!(
        result.len(),
        ids.len(),
        "each queried token should be present"
    );
    for id in &ids {
        let item = result.get(id).expect("result should contain token");
        assert!(item.is_none(), "proof of non-existence expected");
    }
}

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_duplicate_token() {
    super::common::setup_logs();
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_token_not_found").await;

    let ids = [*TOKEN_ID_0, *TOKEN_ID_0];

    let result = TokenPricingSchedule::fetch_many(&sdk, &ids[..])
        .await
        .expect("query should succeed");

    assert_eq!(
        result
            .get(&*TOKEN_ID_0)
            .expect("result should contain token"),
        &None,
        "proof of non-existence expected"
    );
}
