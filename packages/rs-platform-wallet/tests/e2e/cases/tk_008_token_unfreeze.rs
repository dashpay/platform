//! TK-008 — Unfreeze identity for token.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §TK-008. Round-trip pin: freeze
//! followed by unfreeze must restore the pre-freeze invariant — a
//! peer that was rejected mid-freeze can transfer once the freeze is
//! released. After unfreeze, `token_frozen_balance_of` must return
//! `0` (per Wave G's editorial note that the helper returns `0`
//! once the `IdentityTokenInfo.frozen` flag is cleared).
//!
//! Self-contained: redoes TK-007's freeze setup inline rather than
//! sharing state across test functions, matching the harness's
//! "self-contained tests" convention. Gated behind the `e2e` cargo feature.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    setup_with_token_and_two_identities, token_balance_of, token_frozen_balance_of,
    wait_for_token_balance, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING_ACTIVE,
};
use crate::framework::wait::wait_for_identity_balance_change;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::DataContract;

const MINT_TO_OWNER: TokenAmount = 1_000;
const TRANSFER_TO_PEER: TokenAmount = 200;
const PEER_RETURN: TokenAmount = 50;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_008_token_unfreeze() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_008") {
        return;
    }
    let two =
        setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING_ACTIVE)
            .await
            .expect("two-identity token setup");
    let test_wallet = &two.setup.setup_guard.base.test_wallet;
    let owner = &two.setup.owner;
    let peer = &two.peer;
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;

    // Mint to owner.
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

    // Freeze peer (TK-007 precondition replay).
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

    // Snapshot owner credits before unfreeze so we can assert it
    // charged a non-zero fee — `UnfreezeResult` carries no
    // `actual_fee` field.
    let owner_credits_pre = IdentityBalance::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner credits pre-unfreeze")
        .expect("owner identity present");

    // Unfreeze.
    test_wallet
        .platform_wallet()
        .identity()
        .token_unfreeze_with_signer(
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
        .expect("token unfreeze");

    // Marvin TK-008 forensics (v30): same stale-read mechanism as
    // TK-007. `IdentityBalance::fetch` may round-robin onto a DAPI
    // replica that hasn't yet applied the unfreeze block; poll until
    // the chain surfaces a distinct balance.
    let owner_credits_post =
        wait_for_identity_balance_change(ctx.sdk(), owner.id, owner_credits_pre, STEP_TIMEOUT)
            .await
            .expect("owner credit balance never changed after unfreeze");

    // Frozen-balance helper: returns the identity's full token
    // balance while frozen, `0` once the `frozen` flag is cleared.
    let frozen_balance = token_frozen_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("frozen balance fetch");
    assert_eq!(
        frozen_balance, 0,
        "post-unfreeze frozen-balance helper must return 0 \
         (the IdentityTokenInfo.frozen flag is cleared); observed {frozen_balance}"
    );

    // Peer retries the transfer that was blocked while frozen.
    let owner_balance_pre_return = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner balance pre-return");

    test_wallet
        .platform_wallet()
        .identity()
        .token_transfer_with_signer(
            data_contract,
            position,
            peer.id,
            owner.id,
            PEER_RETURN,
            &peer.critical_key,
            peer.signer.as_ref(),
            None,
            None,
        )
        .await
        .expect("post-unfreeze peer transfer");

    let expected_owner_balance = owner_balance_pre_return + PEER_RETURN;
    wait_for_token_balance(
        ctx,
        owner.id,
        contract_id,
        position,
        expected_owner_balance,
        STEP_TIMEOUT,
    )
    .await
    .expect("owner balance increment not observed");

    let peer_balance = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer balance fetch");
    assert_eq!(
        peer_balance,
        TRANSFER_TO_PEER - PEER_RETURN,
        "peer balance must decrement by PEER_RETURN \
         (expected {}, observed {peer_balance})",
        TRANSFER_TO_PEER - PEER_RETURN
    );

    assert!(
        owner_credits_post < owner_credits_pre,
        "unfreeze must charge identity credits \
         (pre={owner_credits_pre} post={owner_credits_post})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_008",
        owner_id = ?owner.id,
        peer_id = ?peer.id,
        ?contract_id,
        position,
        peer_balance,
        fee_charged = owner_credits_pre - owner_credits_post,
        "TK-008 post-unfreeze snapshot"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
