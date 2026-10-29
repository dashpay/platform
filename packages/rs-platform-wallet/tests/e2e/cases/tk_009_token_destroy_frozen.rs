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
//! chaining onto TK-007's state. Gated behind the `e2e` cargo feature.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    setup_with_token_and_two_identities, token_balance_of, token_frozen_balance_of,
    wait_for_token_balance, wait_for_token_supply, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING,
};
use crate::framework::wait::{wait_for_token_predicate, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES};

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::DataContract;

const MINT_TO_OWNER: TokenAmount = 1_000;
const TRANSFER_TO_PEER: TokenAmount = 200;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_009_token_destroy_frozen() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_009") {
        return;
    }
    let two = setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING)
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

    // Streak-confirm the post-mint total supply rather than a bare
    // one-shot read: the mint's supply-total update round-robins
    // independently of the already-confirmed balances, so a lagging
    // replica here would set an unreachable post-destroy target and
    // false-red the poll below. Base supply is DEFAULT_BASE_SUPPLY (0)
    // and the pre-freeze transfer moves tokens without changing supply,
    // so the total is exactly MINT_TO_OWNER; a drifted value reds on
    // timeout, surfacing a fixture change instead of masking it.
    let supply_pre_destroy =
        wait_for_token_supply(ctx, contract_id, position, MINT_TO_OWNER, STEP_TIMEOUT)
            .await
            .expect("post-mint total supply did not settle to MINT_TO_OWNER");

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

    // The fee debit lands on whichever replica served the destroy
    // broadcast, so a bare post-fetch can round-robin onto a sibling still
    // serving the pre-debit balance (Marvin TK-007/008). Poll until credits
    // drop below the pre snapshot across a consecutive-success streak — a
    // stale `== pre` read fails the `< pre` gate and keeps waiting, and a
    // timeout still reds (credits genuinely never dropped).
    let owner_credits_post = wait_for_token_predicate(
        "owner credits < pre (post-destroy fee debit)",
        || async {
            match IdentityBalance::fetch(ctx.sdk(), owner.id).await {
                Ok(Some(post)) if post < owner_credits_pre => Ok(Some(post)),
                Ok(_) => Ok(None),
                Err(err) => Err(FrameworkError::Sdk(format!(
                    "fetch owner credits post-destroy: {err}"
                ))),
            }
        },
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("owner credits did not drop below the pre-destroy snapshot");

    // Post-destroy reads round-robin across DAPI replicas; one that hasn't
    // applied the burn yet still serves stale pre-destroy state. Gate each
    // read on a consecutive-success streak so a lagging sibling can't red
    // the run (see `wait_for_token_predicate`).
    let peer_balance = wait_for_token_predicate(
        "peer token_balance_of == 0 (post-destroy)",
        || async {
            match token_balance_of(ctx, contract_id, position, peer.id).await {
                Ok(b) if b == 0 => Ok(Some(b)),
                Ok(_) => Ok(None),
                Err(err) => Err(err),
            }
        },
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("peer balance did not settle to 0 post-destroy");
    assert_eq!(
        peer_balance, 0,
        "peer balance must be 0 after destroy_frozen_funds; observed {peer_balance}"
    );

    let supply_post_destroy = wait_for_token_supply(
        ctx,
        contract_id,
        position,
        supply_pre_destroy - TRANSFER_TO_PEER,
        STEP_TIMEOUT,
    )
    .await
    .expect("total supply did not settle to the post-destroy target");
    assert_eq!(
        supply_post_destroy,
        supply_pre_destroy - TRANSFER_TO_PEER,
        "total supply must decrease by exactly the destroyed amount \
         (pre={supply_pre_destroy} post={supply_post_destroy} destroyed={TRANSFER_TO_PEER})"
    );

    // Frozen-balance helper: with the peer's balance now zero, the
    // helper returns 0 even though the `IdentityTokenInfo.frozen`
    // flag may still be set (full balance × frozen-flag = 0).
    let frozen_balance = wait_for_token_predicate(
        "peer token_frozen_balance_of == 0 (post-destroy)",
        || async {
            match token_frozen_balance_of(ctx, contract_id, position, peer.id).await {
                Ok(f) if f == 0 => Ok(Some(f)),
                Ok(_) => Ok(None),
                Err(err) => Err(err),
            }
        },
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("frozen balance did not settle to 0 post-destroy");
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
