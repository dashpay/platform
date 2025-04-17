use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use assert_matches::assert_matches;
use dash_sdk::platform::tokens::token_info::{IdentitiesTokenInfosQuery, IdentityTokenInfosQuery};
use dash_sdk::platform::FetchMany;
use dpp::tokens::calculate_token_id;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::info::IdentityTokenInfo;
use drive_proof_verifier::types::token_info::{IdentitiesTokenInfos, IdentityTokenInfos};
use drive_proof_verifier::Length;

/// Fetches multiple token infos of specific identity
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_token_info() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_identity_token_info").await;

    let query = IdentityTokenInfosQuery {
        identity_id: IDENTITY_ID_2,
        token_ids: vec![*TOKEN_ID_0, *TOKEN_ID_1, *TOKEN_ID_2, UNKNOWN_TOKEN_ID],
    };

    let token_infos: IdentityTokenInfos = IdentityTokenInfo::fetch_many(&sdk, query)
        .await
        .expect("fetch identity token infos");

    assert_eq!(token_infos.count(), 4);

    assert_matches!(token_infos.get(&*TOKEN_ID_0), Some(Some(info)) if info.frozen());
    assert_matches!(token_infos.get(&*TOKEN_ID_1), Some(None));
    assert_matches!(token_infos.get(&*TOKEN_ID_2), Some(None));
    assert_matches!(token_infos.get(&UNKNOWN_TOKEN_ID), Some(None));
}

/// Fetches unknown token infos of multiple identities
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identities_unknown_token_infos() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_identities_unknown_token_infos").await;

    let query = IdentitiesTokenInfosQuery {
        identity_ids: vec![
            IDENTITY_ID_1,
            IDENTITY_ID_2,
            IDENTITY_ID_3,
            UNKNOWN_IDENTITY_ID,
        ],
        token_id: UNKNOWN_TOKEN_ID,
    };

    let token_infos: IdentitiesTokenInfos = IdentityTokenInfo::fetch_many(&sdk, query)
        .await
        .expect("fetch identity token infos");

    assert_eq!(token_infos.count(), 4);

    assert_matches!(token_infos.get(&IDENTITY_ID_1), Some(None));
    assert_matches!(token_infos.get(&IDENTITY_ID_2), Some(None));
    assert_matches!(token_infos.get(&IDENTITY_ID_3), Some(None));
    assert_matches!(token_infos.get(&UNKNOWN_IDENTITY_ID), Some(None));
}

/// Fetches token infos of multiple identities
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identities_token_infos() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_identities_token_infos").await;

    let token_id_0 = calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 0).into();

    let query = IdentitiesTokenInfosQuery {
        identity_ids: vec![
            IDENTITY_ID_1,
            IDENTITY_ID_2,
            IDENTITY_ID_3,
            UNKNOWN_IDENTITY_ID,
        ],
        token_id: token_id_0,
    };

    let token_infos: IdentitiesTokenInfos = IdentityTokenInfo::fetch_many(&sdk, query)
        .await
        .expect("fetch identity token infos");

    assert_eq!(token_infos.count(), 4);

    assert_matches!(token_infos.get(&IDENTITY_ID_1), Some(None));
    assert_matches!(token_infos.get(&IDENTITY_ID_2), Some(Some(info)) if info.frozen());
    assert_matches!(token_infos.get(&IDENTITY_ID_3), Some(None));
    assert_matches!(token_infos.get(&UNKNOWN_IDENTITY_ID), Some(None));
}
