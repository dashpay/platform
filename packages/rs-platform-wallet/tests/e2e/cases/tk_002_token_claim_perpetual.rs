//! TK-002 — Token claim against a live perpetual distribution.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-002 (long-runtime, nightly only).
//!
//! Owner deploys a token with a `BlockBasedDistribution` perpetual
//! rule (interval = 5 blocks, function = `FixedAmount { amount }`,
//! recipient = `ContractOwner` — the testnet floor for block
//! interval is 5; smaller intervals trip
//! `InvalidTokenDistributionBlockIntervalTooShortError` at chain
//! validation). After the contract registers, the test waits long
//! enough for the platform block height to advance past one
//! interval boundary and issues
//! `token_claim` with `TokenDistributionType::Perpetual`. Asserts
//! the owner's balance increased by at least one `amount` payout.
//!
//! Why a wall-clock sleep instead of a height-poll: the e2e harness
//! doesn't expose a "platform block height" probe today, and TK-002
//! only needs *some* boundary to have elapsed. ~3 s/block on testnet
//! puts a 5-block interval at ~15 s; the wait below adds generous
//! headroom. The test is `#[ignore]` (nightly only) so the long wall
//! clock doesn't impact CI.
//!
//! Gated behind `#[ignore]` — same operator-env reasoning as the
//! transfer case (`PLATFORM_WALLET_E2E_BANK_MNEMONIC` + live testnet
//! DAPI access).

use std::sync::Arc;
use std::time::Duration;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::DataContract;

use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ClaimResult;
use dash_sdk::platform::Fetch;

use crate::framework::harness::E2eContext;
use crate::framework::tokens::{
    setup_with_token_perpetual_distribution, token_balance_of, PerpetualDistribution,
    DEFAULT_TK_FUNDING, DEFAULT_TOKEN_POSITION,
};

/// Per-interval payout. Small enough that a multi-credit regression
/// (double-pay, off-by-one cycle) shows up as an unmistakable balance
/// mismatch — but the assert below accepts ≥ PAYOUT to tolerate
/// multiple intervals having elapsed before the claim lands.
const PAYOUT: TokenAmount = 100;

/// Perpetual block interval. Testnet floor is 5 (see
/// `RewardDistributionType::validate_structure_interval_v0`). Anything
/// smaller trips `InvalidTokenDistributionBlockIntervalTooShortError`
/// at chain validation.
const INTERVAL_BLOCKS: u64 = 5;

/// Wait window for at least one interval boundary to elapse. Testnet
/// produces a platform block roughly every 3 s; 5 blocks ≈ 15 s.
/// Multiplied by 4× plus a 30 s floor for transient block-time
/// stretching and DAPI propagation lag.
const PERPETUAL_WAIT: Duration = Duration::from_secs(90);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "long-runtime perpetual claim (≈90 s wall-clock); requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_002_token_claim_perpetual_distribution() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");

    let setup = setup_with_token_perpetual_distribution(
        ctx,
        DEFAULT_TK_FUNDING,
        PerpetualDistribution {
            interval_blocks: INTERVAL_BLOCKS,
            amount_per_interval: PAYOUT,
        },
    )
    .await
    .expect("deploy token with perpetual distribution");

    let contract_id = setup.contract_id;
    let owner_id = setup.owner.id;

    // Snapshot pre-claim balance — strict diff, mirrors TK-013.
    let balance_before = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, owner_id)
        .await
        .expect("pre-claim balance");

    // Wait for at least one interval boundary to advance past the
    // contract-creation block height. No height-poll helper exists in
    // the e2e harness today, so we sleep — the test is `#[ignore]`d
    // (nightly only), so the wall-clock cost stays out of CI.
    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_002",
        ?contract_id,
        ?owner_id,
        interval_blocks = INTERVAL_BLOCKS,
        wait_secs = PERPETUAL_WAIT.as_secs(),
        "TK-002 waiting for perpetual interval boundary"
    );
    tokio::time::sleep(PERPETUAL_WAIT).await;

    // Build + broadcast the perpetual claim. Mirrors TK-013's direct
    // SDK-builder path (the wallet's `token_claim_with_signer` is a
    // thin forward to `Sdk::token_claim`).
    let data_contract = Arc::new(
        DataContract::fetch(ctx.sdk(), contract_id)
            .await
            .expect("fetch token data contract")
            .expect("token data contract present on chain"),
    );
    let builder = TokenClaimTransitionBuilder::new(
        Arc::clone(&data_contract),
        DEFAULT_TOKEN_POSITION,
        owner_id,
        TokenDistributionType::Perpetual,
    );
    let claim_result = ctx
        .sdk()
        .token_claim(
            builder,
            &setup.owner.critical_key,
            setup.owner.signer.as_ref(),
        )
        .await
        .expect("token_claim broadcast");

    match &claim_result {
        ClaimResult::Document(_) | ClaimResult::GroupActionWithDocument(_, _) => {}
    }

    let balance_after = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, owner_id)
        .await
        .expect("post-claim balance");

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_002",
        ?contract_id,
        ?owner_id,
        balance_before,
        balance_after,
        payout = PAYOUT,
        "TK-002 post-claim balance snapshot"
    );

    // Use ≥ rather than == because more than one interval may have
    // elapsed by the time the claim lands (testnet block time can
    // tighten well below 3 s under load). The contract is fresh —
    // any balance growth at all is attributable to this claim.
    assert!(
        balance_after >= balance_before + PAYOUT,
        "post-claim balance must grow by at least one payout \
         (claim from perpetual distribution silently fails — balance just doesn't move). \
         observed before={balance_before} after={balance_after} expected_min_delta={PAYOUT}"
    );

    setup.setup_guard.teardown().await.expect("teardown");
}
