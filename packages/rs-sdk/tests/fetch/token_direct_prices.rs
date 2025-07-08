use dash_sdk::platform::FetchMany;
use dpp::{prelude::Identifier, tokens::token_pricing_schedule::TokenPricingSchedule};

use crate::fetch::{
    config::Config,
    generated_data::{TOKEN_ID_0, TOKEN_ID_1, TOKEN_ID_2},
};

/// Given some dummy token ID, when I fetch token prices, I get proof of non-existence.
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

/// Given some existing token IDs, when I fetch token prices, I get correct results.
///
/// See [Platform::create_data_for_token_direct_prices](drive_abci::platform_types::platform::Platform::create_data_for_token_direct_prices) for token definitions.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_tokens_ok() {
    super::common::setup_logs();
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_tokens_ok").await;

    let ids = [*TOKEN_ID_0, *TOKEN_ID_1, *TOKEN_ID_2];

    let result = TokenPricingSchedule::fetch_many(&sdk, &ids[..])
        .await
        .expect("query should succeed");

    assert_eq!(
        result.len(),
        ids.len(),
        "each queried token should be present"
    );

    assert!(matches!(result.get(&*TOKEN_ID_0), Some(None)));
    assert!(matches!(
        result.get(&*TOKEN_ID_1),
        Some(Some(TokenPricingSchedule::SinglePrice(25)))
    ));

    let Some(Some(TokenPricingSchedule::SetPrices(schedule))) = result.get(&*TOKEN_ID_2) else {
        panic!("expected token pricing schedule");
    };

    assert_eq!(schedule.len(), 10);
    for (amount, price) in schedule {
        assert_eq!(*price, 1000 - amount);
    }
}

/// Given two identical existing token IDs,
/// when we fetch direct purchase prices,
/// we receive only one in response.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_direct_prices_duplicate_token() {
    super::common::setup_logs();
    let cfg = Config::new();
    let sdk = cfg.setup_api("test_direct_prices_duplicate_token").await;

    let ids = [*TOKEN_ID_2, *TOKEN_ID_2];

    let result = TokenPricingSchedule::fetch_many(&sdk, &ids[..])
        .await
        .expect("query should succeed");

    assert_eq!(result.len(), 1, "only one token should be present");
    assert!(
        result
            .get(&*TOKEN_ID_2)
            .expect("result should contain token")
            .is_some(),
        "pricing info expected"
    );
}
