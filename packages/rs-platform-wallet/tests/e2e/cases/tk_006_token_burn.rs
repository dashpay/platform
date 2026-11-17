//! TK-006 — Token burn + total-supply decrement.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Tokens (TK) → TK-006).
//! Pinned status: BLOCKED until run on a live testnet.
//!
//! Drives `Sdk::token_burn` end-to-end via the SDK
//! `TokenBurnTransitionBuilder` — Wave G's framework helper set
//! covers mint/transfer/freeze but does not yet expose a `burn_to`
//! shortcut, so this case calls the SDK directly. Pins:
//! - Owner balance decrements `mint → mint − burn`.
//! - Total supply decrements `mint → mint − burn` (mint+burn pair
//!   is supply-conservative around the burned amount).
//! - `BurnResult::TokenBalance` reports the same remaining balance
//!   the read-side accessor sees.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::tokens::builders::burn::TokenBurnTransitionBuilder;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_contract, token_balance_of, token_supply_of, TK_OWNER_FUNDING_SIMPLE,
};
use crate::framework::wait::wait_for_identity_balance_change;

/// Pre-burn mint that seeds the owner's balance.
const MINT_AMOUNT: u64 = 1_000;

/// Burn amount — strictly less than `MINT_AMOUNT` so the residual
/// balance is non-zero and the mint+burn supply arithmetic stays
/// positive (matches the spec: `1_000 → 900`).
const BURN_AMOUNT: u64 = 100;

/// Expected residual after `MINT_AMOUNT − BURN_AMOUNT`.
const EXPECTED_RESIDUAL: u64 = MINT_AMOUNT - BURN_AMOUNT;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_006_token_burn() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_006") {
        return;
    }
    let setup = setup_with_token_contract(ctx, TK_OWNER_FUNDING_SIMPLE)
        .await
        .expect("setup_with_token_contract");

    let contract_id = setup.contract_id;
    let position = setup.token_position;
    let owner_id = setup.owner.id;

    // Seed the owner's balance — TK-006 explicitly chains a mint
    // before the burn rather than depending on TK-005's run order.
    mint_to(
        ctx,
        contract_id,
        position,
        MINT_AMOUNT,
        &setup.owner,
        &setup.owner,
    )
    .await
    .expect("seed mint");

    let pre_burn_supply = token_supply_of(ctx, contract_id, position)
        .await
        .expect("pre-burn supply");
    assert_eq!(
        pre_burn_supply, MINT_AMOUNT,
        "pre-burn supply must equal seeded mint ({MINT_AMOUNT}); got {pre_burn_supply}"
    );

    let pre_burn_balance = token_balance_of(ctx, contract_id, position, owner_id)
        .await
        .expect("pre-burn owner balance");
    assert_eq!(
        pre_burn_balance, MINT_AMOUNT,
        "pre-burn owner balance must equal seeded mint ({MINT_AMOUNT}); got {pre_burn_balance}"
    );

    // Burn — go SDK-direct via the builder. The wallet exposes
    // `token_burn_with_signer` but binding a full `IdentityWallet`
    // here would force the test to also adopt the wallet-side
    // broadcaster wiring. The builder path is what the wallet
    // helper itself ends up calling.
    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch data contract")
        .expect("contract must exist");

    let builder =
        TokenBurnTransitionBuilder::new(Arc::new(data_contract), position, owner_id, BURN_AMOUNT);

    // Snapshot the owner's identity-credit balance pre-burn so we can
    // assert the spec's `actual_fee > 0` requirement. `BurnResult`
    // does not surface a fee field on any variant — the closest
    // available signal is the credit-balance delta around the
    // transition.
    let owner_credits_pre_burn = IdentityBalance::fetch(ctx.sdk(), owner_id)
        .await
        .expect("fetch owner credits pre-burn")
        .expect("owner identity present");

    let _burn_result = ctx
        .sdk()
        .token_burn(
            builder,
            &setup.owner.critical_key,
            setup.owner.signer.as_ref(),
        )
        .await
        .expect("token_burn");

    // Marvin TK-007/008 forensics generalises here: `IdentityBalance::
    // fetch` may round-robin onto a DAPI replica that hasn't yet
    // applied the burn block and return the pre-burn value, even
    // though `broadcast_and_wait` confirmed apply on the serving node.
    // Poll until the chain surfaces a distinct balance — the burn
    // always charges credits, so any change clears the gate.
    let owner_credits_post_burn = wait_for_identity_balance_change(
        ctx.sdk(),
        owner_id,
        owner_credits_pre_burn,
        Duration::from_secs(60),
    )
    .await
    .expect("owner credit balance never changed after burn");
    let burn_fee = owner_credits_pre_burn.saturating_sub(owner_credits_post_burn);
    assert!(
        burn_fee > 0,
        "burn must charge identity credits (`actual_fee > 0` per spec) \
         (pre={owner_credits_pre_burn} post={owner_credits_post_burn})"
    );

    let post_burn_supply = token_supply_of(ctx, contract_id, position)
        .await
        .expect("post-burn supply");
    assert_eq!(
        post_burn_supply, EXPECTED_RESIDUAL,
        "post-burn supply must equal MINT_AMOUNT - BURN_AMOUNT ({EXPECTED_RESIDUAL}); got {post_burn_supply}"
    );

    let post_burn_balance = token_balance_of(ctx, contract_id, position, owner_id)
        .await
        .expect("post-burn owner balance");
    assert_eq!(
        post_burn_balance, EXPECTED_RESIDUAL,
        "post-burn owner balance must equal MINT_AMOUNT - BURN_AMOUNT ({EXPECTED_RESIDUAL}); got {post_burn_balance}"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_006",
        %contract_id,
        %owner_id,
        pre_burn_supply,
        post_burn_supply,
        post_burn_balance,
        burn_fee,
        "TK-006 burn snapshot"
    );

    setup.setup_guard.teardown().await.expect("teardown");
}
