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

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_contract, token_balance_of, token_supply_of, DEFAULT_TK_FUNDING,
};

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
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_005_token_mint() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    let setup = setup_with_token_contract(ctx, DEFAULT_TK_FUNDING)
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

    // Mint #1 — owner → owner.
    mint_to(
        ctx,
        contract_id,
        position,
        MINT_AMOUNT_A,
        &setup.owner,
        &setup.owner,
    )
    .await
    .expect("first mint to owner");

    // Mint #2 — owner → owner (explicit recipient via builder).
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
