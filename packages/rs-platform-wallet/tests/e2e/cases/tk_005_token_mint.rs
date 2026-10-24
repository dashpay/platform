//! TK-005 — Token mint + total-supply assertion.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Tokens (TK) → TK-005).
//! Pinned status: BLOCKED until run on a live testnet.
//!
//! Drives `Sdk::token_mint` (via the framework `mint_to` helper) end
//! to end on a freshly-deployed permissive owner-only token contract.
//! Pins:
//! - Two consecutive mints to the owner accumulate in both the
//!   per-identity balance and the contract-wide total supply.
//! - Pre-mint supply is `0` (matches `DEFAULT_BASE_SUPPLY`).
//! - Post-mint supply equals the sum of both mint amounts.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;
use dash_sdk::platform::Fetch;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_contract_with_step_timeout, token_balance_of, token_supply_of,
    TK_OWNER_FUNDING_SIMPLE,
};

/// Per-step propagation budget for TK-005's bootstrap (QA-V28-403). The
/// default 60 s framework timeout is too tight when this test funds 35 B
/// credits in a single hop while seven sibling guards compete for the
/// bank under `--test-threads=8`: the funding broadcast lands but
/// `wait_for_balance`'s chain-confirmed gate doesn't clear inside the
/// deadline. 120 s is plenty without softening the global default — the
/// rest of the suite keeps the tight 60 s budget so a genuinely-stuck
/// test still surfaces fast.
const SETUP_STEP_TIMEOUT: Duration = Duration::from_secs(120);

/// First mint amount — owner mints to self with implicit recipient.
const MINT_AMOUNT_A: u64 = 500_000;

/// Second mint amount — owner mints to self with the explicit
/// `recipient_id = owner_id` branch (the `mint_to` helper always
/// passes a recipient via `issued_to_identity_id`, which is the
/// branch this case pins).
const MINT_AMOUNT_B: u64 = 50_000;

/// Total expected supply / owner balance after both mints.
const EXPECTED_TOTAL: u64 = MINT_AMOUNT_A + MINT_AMOUNT_B;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_005_token_mint() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_005") {
        return;
    }
    let setup = setup_with_token_contract_with_step_timeout(
        ctx,
        TK_OWNER_FUNDING_SIMPLE,
        SETUP_STEP_TIMEOUT,
    )
    .await
    .expect("setup_with_token_contract");

    let contract_id = setup.contract_id;
    let position = setup.token_position;
    let owner_id = setup.owner.id;

    // Pre-mint supply is the contract's `baseSupply` — `0` for the
    // permissive owner-only template (`DEFAULT_BASE_SUPPLY`).
    let pre_supply = token_supply_of(ctx, contract_id, position)
        .await
        .expect("pre-mint supply");
    assert_eq!(
        pre_supply, 0,
        "pre-mint supply must equal DEFAULT_BASE_SUPPLY (0); got {pre_supply}"
    );

    let pre_balance = token_balance_of(ctx, contract_id, position, owner_id)
        .await
        .expect("pre-mint owner balance");
    assert_eq!(
        pre_balance, 0,
        "pre-mint owner balance must be 0; got {pre_balance}"
    );

    // Mint #1 — owner → (implicit recipient via `recipient_id: None`).
    // The framework `mint_to` always sets `issued_to_identity_id`, so
    // we drive the SDK builder directly here to keep the
    // `recipient_id: None` (default-to-owner) branch covered. The
    // contract's `mintingAllowChoosingDestination` is true and
    // `newTokensDestinationIdentity` is the owner, so the protocol
    // routes the mint to the owner anyway.
    let data_contract = Arc::new(
        DataContract::fetch(ctx.sdk(), contract_id)
            .await
            .expect("fetch data contract")
            .expect("contract present"),
    );
    let builder_implicit = TokenMintTransitionBuilder::new(
        Arc::clone(&data_contract),
        position,
        owner_id,
        MINT_AMOUNT_A,
    );
    ctx.sdk()
        .token_mint(
            builder_implicit,
            &setup.owner.critical_key,
            setup.owner.signer.as_ref(),
        )
        .await
        .expect("first mint (implicit recipient)");

    // Mint #2 — owner → owner (explicit recipient via the framework
    // `mint_to` helper, which sets `issued_to_identity_id`).
    mint_to(
        ctx,
        contract_id,
        position,
        MINT_AMOUNT_B,
        &setup.owner,
        &setup.owner,
    )
    .await
    .expect("second mint to owner");

    let post_supply = token_supply_of(ctx, contract_id, position)
        .await
        .expect("post-mint supply");
    assert_eq!(
        post_supply, EXPECTED_TOTAL,
        "post-mint supply must equal MINT_AMOUNT_A + MINT_AMOUNT_B ({EXPECTED_TOTAL}); got {post_supply}"
    );

    let post_balance = token_balance_of(ctx, contract_id, position, owner_id)
        .await
        .expect("post-mint owner balance");
    assert_eq!(
        post_balance, EXPECTED_TOTAL,
        "post-mint owner balance must equal mint total ({EXPECTED_TOTAL}); got {post_balance}"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_005",
        %contract_id,
        %owner_id,
        pre_supply,
        post_supply,
        post_balance,
        "TK-005 mint snapshot"
    );

    setup.setup_guard.teardown().await.expect("teardown");
}
