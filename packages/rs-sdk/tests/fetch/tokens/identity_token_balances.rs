use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use dash_sdk::platform::tokens::identity_token_balances::{
    IdentitiesTokenBalancesQuery, IdentityTokenBalancesQuery,
};
use dash_sdk::platform::FetchMany;
use dpp::balances::credits::TokenAmount;
use dpp::tokens::calculate_token_id;
use drive_proof_verifier::types::identity_token_balance::{
    IdentitiesTokenBalances, IdentityTokenBalances,
};
use drive_proof_verifier::Length;

/// Fetches token balances of multiple identities, including the one, which does not exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_multiple_identity_token_balances() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_multiple_identity_token_balances").await;

    let query = IdentityTokenBalancesQuery {
        identity_id: cfg.existing_identity_id,
        token_ids: vec![*TOKEN_ID_0, *TOKEN_ID_1, *TOKEN_ID_2, UNKNOWN_TOKEN_ID],
    };

    let balances: IdentityTokenBalances = TokenAmount::fetch_many(&sdk, query)
        .await
        .expect("fetch identity token balances");

    assert_eq!(balances.count(), 4);

    assert_eq!(balances.get(&*TOKEN_ID_0), Some(&Some(100100)));
    assert_eq!(balances.get(&*TOKEN_ID_1), Some(&Some(100200)));
    assert_eq!(balances.get(&*TOKEN_ID_2), Some(&Some(100300)));
    assert_eq!(balances.get(&UNKNOWN_TOKEN_ID), Some(&None::<TokenAmount>));
}

/// Fetches unknown token balances of multiple identities, including the one, which does not exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_multiple_identities_with_unknown_token_balance() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_multiple_identities_with_unknown_token_balance")
        .await;

    let query = IdentitiesTokenBalancesQuery {
        identity_ids: vec![
            IDENTITY_ID_1,
            IDENTITY_ID_2,
            IDENTITY_ID_3,
            UNKNOWN_IDENTITY_ID,
        ],
        token_id: UNKNOWN_TOKEN_ID,
    };

    let balances: IdentitiesTokenBalances = TokenAmount::fetch_many(&sdk, query)
        .await
        .expect("fetch identities token balances");

    assert_eq!(balances.count(), 4);

    assert_eq!(balances.get(&IDENTITY_ID_1), Some(&None::<TokenAmount>));
    assert_eq!(balances.get(&IDENTITY_ID_2), Some(&None::<TokenAmount>));
    assert_eq!(balances.get(&IDENTITY_ID_3), Some(&None::<TokenAmount>));
    assert_eq!(
        balances.get(&UNKNOWN_IDENTITY_ID),
        Some(&None::<TokenAmount>)
    );
}

/// Fetches token balances of multiple identities, including the one, which does not exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_multiple_identities_token_balances() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_multiple_identities_token_balances")
        .await;

    let token_id_0 = calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 0).into();

    let query = IdentitiesTokenBalancesQuery {
        identity_ids: vec![
            IDENTITY_ID_1,
            IDENTITY_ID_2,
            IDENTITY_ID_3,
            UNKNOWN_IDENTITY_ID,
        ],
        token_id: token_id_0,
    };

    let balances: IdentitiesTokenBalances = TokenAmount::fetch_many(&sdk, query)
        .await
        .expect("fetch identities token balances");

    assert_eq!(balances.count(), 4);

    assert_eq!(balances.get(&IDENTITY_ID_1), Some(&Some(100100)));
    assert_eq!(balances.get(&IDENTITY_ID_2), Some(&Some(100)));
    assert_eq!(balances.get(&IDENTITY_ID_3), Some(&None::<TokenAmount>));
    assert_eq!(
        balances.get(&UNKNOWN_IDENTITY_ID),
        Some(&None::<TokenAmount>)
    );
}
