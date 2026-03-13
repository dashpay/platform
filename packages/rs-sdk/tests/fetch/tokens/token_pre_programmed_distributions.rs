// TODO: Generate test vectors by running against a devnet:
//   yarn reset && SDK_TEST_DATA=true yarn start
//   ./packages/rs-sdk/scripts/generate_test_vectors.sh test_token_pre_programmed_distributions

use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use dash_sdk::platform::tokens::token_pre_programmed_distributions::{
    TokenPreProgrammedDistributions, TokenPreProgrammedDistributionsQuery,
};
use dash_sdk::platform::Fetch;

/// TOKEN_ID_2 has pre-programmed distributions configured with 3 timestamps.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_token_pre_programmed_distributions_present() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_token_pre_programmed_distributions_present")
        .await;

    let query = TokenPreProgrammedDistributionsQuery {
        token_id: *TOKEN_ID_2,
        start_at_info: None,
        limit: None,
    };

    let distributions = TokenPreProgrammedDistributions::fetch(&sdk, query)
        .await
        .expect("fetch token pre-programmed distributions");

    let distributions = distributions.expect("TOKEN_ID_2 should have pre-programmed distributions");
    assert_eq!(
        distributions.0.len(),
        3,
        "expected 3 distribution timestamps"
    );
}

/// TOKEN_ID_0 has no pre-programmed distributions; query should return None.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_token_pre_programmed_distributions_absent() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_token_pre_programmed_distributions_absent")
        .await;

    let query = TokenPreProgrammedDistributionsQuery {
        token_id: *TOKEN_ID_0,
        start_at_info: None,
        limit: None,
    };

    let distributions = TokenPreProgrammedDistributions::fetch(&sdk, query)
        .await
        .expect("fetch token pre-programmed distributions");

    assert!(
        distributions.is_none(),
        "expected no pre-programmed distributions for TOKEN_ID_0"
    );
}
