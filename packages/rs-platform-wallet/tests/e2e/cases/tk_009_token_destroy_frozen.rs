//! TK-009 — Destroy frozen funds.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §TK-009. Pins the irreversible
//! "burn the rule-breaker's bag" admin action: after a freeze, the
//! owner can call `token_destroy_frozen_funds_with_signer` (which
//! takes no `amount` — the call always destroys the full frozen
//! balance) to drop the peer's balance to `0`. Total supply
//! decreases by the destroyed amount, and a follow-up frozen-balance
//! read returns `0` (no balance left to be frozen).
//!
//! Self-contained: stages its own freeze precondition rather than
//! chaining onto TK-007's state. Gated behind `#[ignore]`.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    setup_with_token_and_two_identities, token_balance_of, token_frozen_balance_of,
    token_supply_of, wait_for_token_balance, DEFAULT_TK_FUNDING,
};

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::DataContract;

const TK_FUNDING_PER: dpp::fee::Credits = DEFAULT_TK_FUNDING;
const MINT_TO_OWNER: TokenAmount = 1_000;
const TRANSFER_TO_PEER: TokenAmount = 200;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_009_token_destroy_frozen() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    let two = setup_with_token_and_two_identities(ctx, TK_FUNDING_PER)
        .await
        .expect("two-identity token setup");
    let test_wallet = &two.setup.setup_guard.base.test_wallet;
    let owner = &two.setup.owner;
    let peer = &two.peer;
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;

    // Mint to owner so we have a balance to fund the peer with.
    crate::framework::tokens::mint_to(ctx, contract_id, position, MINT_TO_OWNER, owner, owner)
        .await
        .expect("mint to owner");
    wait_for_token_balance(
        ctx,
        owner.id,
        contract_id,
        position,
        MINT_TO_OWNER,
        STEP_TIMEOUT,
    )
    .await
    .expect("owner mint not observed");

    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch contract")
        .expect("contract present");
    let data_contract = std::sync::Arc::new(data_contract);

    // Owner -> peer pre-freeze transfer.
    test_wallet
        .platform_wallet()
        .identity()
        .token_transfer_with_signer(
            data_contract.clone(),
            position,
            owner.id,
            peer.id,
            TRANSFER_TO_PEER,
            &owner.critical_key,
            owner.signer.as_ref(),
            None,
            None,
        )
        .await
        .expect("token transfer pre-freeze");
    wait_for_token_balance(
        ctx,
        peer.id,
        contract_id,
        position,
        TRANSFER_TO_PEER,
        STEP_TIMEOUT,
    )
    .await
    .expect("peer pre-freeze balance not observed");

    // Snapshot the post-mint total supply. With no burns yet, this
    // equals MINT_TO_OWNER; we capture the live value rather than
    // pinning the constant so a future change to the helper's
    // base-supply default doesn't drift this assertion.
    let supply_pre_destroy = token_supply_of(ctx, contract_id, position)
        .await
        .expect("supply pre-destroy");

    // Freeze peer (TK-007 precondition).
    test_wallet
        .platform_wallet()
        .identity()
        .token_freeze_with_signer(
            data_contract.clone(),
            position,
            owner.id,
            peer.id,
            &owner.critical_key,
            owner.signer.as_ref(),
            None,
            None,
            None,
        )
        .await
        .expect("token freeze");

    // Snapshot owner credits before destroy so we can assert it
    // charged a non-zero fee — `DestroyFrozenFundsResult` carries no
    // `actual_fee` field.
    let owner_credits_pre = IdentityBalance::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner credits pre-destroy")
        .expect("owner identity present");

    // Destroy frozen funds (no amount param — always full balance).
    test_wallet
        .platform_wallet()
        .identity()
        .token_destroy_frozen_funds_with_signer(
            data_contract,
            position,
            owner.id,
            peer.id,
            &owner.critical_key,
            owner.signer.as_ref(),
            None,
            None,
            None,
        )
        .await
        .expect("destroy frozen funds");

    let owner_credits_post = IdentityBalance::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner credits post-destroy")
        .expect("owner identity present");

    let peer_balance = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer balance post-destroy");
    assert_eq!(
        peer_balance, 0,
        "peer balance must be 0 after destroy_frozen_funds; observed {peer_balance}"
    );

    let supply_post_destroy = token_supply_of(ctx, contract_id, position)
        .await
        .expect("supply post-destroy");
    assert_eq!(
        supply_post_destroy,
        supply_pre_destroy - TRANSFER_TO_PEER,
        "total supply must decrease by exactly the destroyed amount \
         (pre={supply_pre_destroy} post={supply_post_destroy} destroyed={TRANSFER_TO_PEER})"
    );

    // Frozen-balance helper: with the peer's balance now zero, the
    // helper returns 0 even though the `IdentityTokenInfo.frozen`
    // flag may still be set (full balance × frozen-flag = 0).
    let frozen_balance = token_frozen_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("frozen balance fetch post-destroy");
    assert_eq!(
        frozen_balance, 0,
        "post-destroy frozen-balance must be 0 (nothing left to freeze); observed {frozen_balance}"
    );

    assert!(
        owner_credits_post < owner_credits_pre,
        "destroy_frozen_funds must charge identity credits \
         (pre={owner_credits_pre} post={owner_credits_post})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_009",
        owner_id = ?owner.id,
        peer_id = ?peer.id,
        ?contract_id,
        position,
        peer_balance,
        supply_pre_destroy,
        supply_post_destroy,
        fee_charged = owner_credits_pre - owner_credits_post,
        "TK-009 post-destroy snapshot"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
