// //! Tests for `getTokenPerpetualDistributionLastClaim` query
// use crate::fetch::common::setup_logs;
// use crate::fetch::config::Config;
// use crate::fetch::generated_data::*; // TOKEN_ID_*, IDENTITY_ID_*, etc.

// use assert_matches::assert_matches;
// use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::ContractTokenInfo;
// use dash_sdk::platform::query::TokenLastClaimQuery;
// use dash_sdk::platform::Fetch;
// use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
// use dpp::tokens::calculate_token_id;

// /// Fetches the last‑claim moment (raw value, no proof)
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_last_claim_direct_value() {
//     setup_logs();

//     let cfg = Config::new();
//     let sdk = cfg.setup_api("test_last_claim_direct_value").await;

//     // ────────────────────────────────────────────────────────────────
//     // Test vector: IDENTITY_ID_2 claimed TOKEN_ID_0 at time X (ms)
//     // ────────────────────────────────────────────────────────────────
//     let query = TokenLastClaimQuery {
//         token_id: *TOKEN_ID_0,
//         identity_id: IDENTITY_ID_2,
//     };

//     let last_claim: Option<RewardDistributionMoment> = RewardDistributionMoment::fetch(&sdk, query)
//         .await
//         .expect("fetch last claim (raw)");

//     assert_matches!(
//         last_claim,
//         Some(RewardDistributionMoment::TimeBasedMoment(ts)) if ts == TODO_LAST_CLAIM_TS_MS
//     );
// }

// /// Fetches the last‑claim moment **with GroveDB proof** and verifies it
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_last_claim_with_proof() {
//     setup_logs();

//     let cfg = Config::new();
//     let sdk = cfg.setup_api("test_last_claim_with_proof").await;

//     // We’ll request TOKEN_ID_1, position 1 in DATA_CONTRACT_ID
//     let contract_id_buf = DATA_CONTRACT_ID.to_buffer();
//     let token_id_1 = calculate_token_id(&contract_id_buf, 1).into();

//     let contract_info = ContractTokenInfo {
//         contract_id: DATA_CONTRACT_ID.to_vec(),
//         token_contract_position: 1,
//     };

//     let query = TokenLastClaimQuery {
//         token_id: token_id_1,
//         identity_id: IDENTITY_ID_2,
//     };

//     let last_claim: Option<RewardDistributionMoment> = RewardDistributionMoment::fetch(&sdk, query)
//         .await
//         .expect("fetch last claim (with proof)");

//     // This particular token uses **block‑based** distribution in the generated fixture
//     assert_matches!(
//         last_claim,
//         Some(RewardDistributionMoment::BlockBasedMoment(h)) if h == TODO_LAST_CLAIM_HEIGHT
//     );
// }

// /// Fetches last‑claim for an identity that never claimed → expect `None`
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_last_claim_absent() {
//     setup_logs();

//     let cfg = Config::new();
//     let sdk = cfg.setup_api("test_last_claim_absent").await;

//     let query = TokenLastClaimQuery {
//         token_id: *TOKEN_ID_2,
//         identity_id: UNKNOWN_IDENTITY_ID, // no such identity / never claimed
//     };

//     let last_claim: Option<RewardDistributionMoment> = RewardDistributionMoment::fetch(&sdk, query)
//         .await
//         .expect("fetch last claim (absent)");

//     assert_matches!(last_claim, None);
// }
