//! TK-007 — Freeze identity for token (admin action).
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §TK-007. Pins the contract owner's
//! `token_freeze_with_signer` admin path: after a successful freeze
//! the target identity's full token balance is unspendable, the
//! frozen-balance accessor reports the locked amount, and the freeze
//! transition itself charges identity credits.
//!
//! Gated behind the `e2e` cargo feature per Wave 2 conventions — needs the
//! operator `tests/.env` plus live testnet access. Run with
//! `cargo test --test e2e -- --ignored --nocapture`.
//!
//! Self-contained: stands up its own two-identity token contract via
//! [`setup_with_token_and_two_identities`] rather than chaining onto
//! a sibling test's frozen state. The cross-test dependency note in
//! `TEST_SPEC.md` is editorial — TK-008 / TK-009 each redo this same
//! setup so a single failure is localised.
//!
//! `actual_fee` is not surfaced on `FreezeResult` (the SDK enum has
//! no fee field), so the "freeze charged credits" assertion is made
//! against the owner's identity balance pre vs. post the transition.

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

/// Token amount the owner mints to itself before transferring some
/// to the peer. Sized well above `TRANSFER_TO_PEER` so the owner's
/// post-transfer balance is unambiguously non-zero.
const MINT_TO_OWNER: TokenAmount = 1_000;

/// Token amount the owner transfers to the peer pre-freeze.
/// Matches the spec's pinned `200` so frozen-balance assertions
/// align with TK-009's destroy step.
const TRANSFER_TO_PEER: TokenAmount = 200;

/// Per-step timeout for token-balance polls.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_007_token_freeze() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_007") {
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

    // Owner transfers TRANSFER_TO_PEER to peer.
    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch contract")
        .expect("contract present");
    let data_contract = std::sync::Arc::new(data_contract);

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

    // Capture owner's identity-credit balance before the freeze
    // transition so we can assert the freeze charged a non-zero fee
    // — `FreezeResult` itself does not expose `actual_fee`.
    let owner_credits_pre = IdentityBalance::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner credits pre-freeze")
        .expect("owner identity present");

    // Owner freezes peer.
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

    // Marvin TK-007 forensics (v30): `IdentityBalance::fetch` may
    // round-robin onto a DAPI replica that hasn't yet applied the
    // freeze block and return the pre-freeze value, even though the
    // SDK's `broadcast_and_wait` confirmed apply on the serving node.
    // Poll the chain side until it surfaces a balance distinct from
    // the snapshot — the freeze always charges credits, so any
    // change clears the gate.
    let owner_credits_post =
        wait_for_identity_balance_change(ctx.sdk(), owner.id, owner_credits_pre, STEP_TIMEOUT)
            .await
            .expect("owner credit balance never changed after freeze");

    let frozen_balance = token_frozen_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("frozen balance fetch");
    assert_eq!(
        frozen_balance, TRANSFER_TO_PEER,
        "frozen balance must equal the locked amount \
         (peer was credited {TRANSFER_TO_PEER}, observed frozen={frozen_balance})"
    );

    // Peer attempts to transfer 50 back to owner — must fail with a
    // typed "frozen" error class. We assert error semantics via
    // string match: the SDK funnels DPP consensus errors as opaque
    // strings here, and the variant
    // `IdentityTokenAccountFrozenError`'s formatter contains the
    // word "frozen" (see rs-dpp consensus state-error 40702).
    let half_back = TRANSFER_TO_PEER / 4;
    let attempt = test_wallet
        .platform_wallet()
        .identity()
        .token_transfer_with_signer(
            data_contract,
            position,
            peer.id,
            owner.id,
            half_back,
            &peer.critical_key,
            peer.signer.as_ref(),
            None,
            None,
        )
        .await;
    let err = match attempt {
        Ok(_) => panic!("frozen peer transfer must fail, but it succeeded"),
        Err(e) => e,
    };
    let err_text = format!("{err:?}").to_lowercase();
    assert!(
        err_text.contains("frozen") || err_text.contains("freeze"),
        "expected 'frozen' / 'freeze' marker in error, got: {err:?}"
    );

    // Peer's token balance unchanged after the failed transfer.
    let peer_balance = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer balance fetch");
    assert_eq!(
        peer_balance, TRANSFER_TO_PEER,
        "frozen peer balance must be unchanged after rejected transfer \
         (expected {TRANSFER_TO_PEER}, observed {peer_balance})"
    );

    assert!(
        owner_credits_post < owner_credits_pre,
        "freeze must charge identity credits \
         (pre={owner_credits_pre} post={owner_credits_post})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_007",
        owner_id = ?owner.id,
        peer_id = ?peer.id,
        ?contract_id,
        position,
        peer_balance,
        frozen_balance,
        fee_charged = owner_credits_pre - owner_credits_post,
        "TK-007 post-freeze snapshot"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
