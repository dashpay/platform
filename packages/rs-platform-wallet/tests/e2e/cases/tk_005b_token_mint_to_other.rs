//! TK-005b — Mint with `recipient_id != self`.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Tokens (TK) → TK-005b).
//! Pinned status: BLOCKED until run on a live testnet.
//!
//! Drives `Sdk::token_mint` with an explicit cross-identity
//! `issued_to_identity_id` recipient on a permissive contract that
//! sets `mintingAllowChoosingDestination = true`. Pins:
//! - The recipient (`peer`) gains the minted balance, not the owner.
//! - The owner's balance stays at `0` after the mint.
//! - Total supply equals the mint amount.

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, token_supply_of,
    TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING,
};

/// Single cross-identity mint amount — sized small (the spec reads
/// `100`) since the assertion is on direction, not magnitude.
const MINT_AMOUNT: u64 = 100;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_005b_token_mint_to_other() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e ctx init");
    if ctx.skip_if_bank_floor_unmet("tk_005b") {
        return;
    }
    let two = setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING)
        .await
        .expect("setup_with_token_and_two_identities");

    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;
    let owner_id = two.setup.owner.id;
    let peer_id = two.peer.id;

    // Owner mints to peer — `mint_to` calls the builder with
    // `issued_to_identity_id(peer_id)` so this exercises the
    // cross-identity destination branch the contract gates on
    // `mintingAllowChoosingDestination = true`.
    mint_to(
        ctx,
        contract_id,
        position,
        MINT_AMOUNT,
        &two.peer,
        &two.setup.owner,
    )
    .await
    .expect("mint to peer");

    let supply = token_supply_of(ctx, contract_id, position)
        .await
        .expect("post-mint supply");
    assert_eq!(
        supply, MINT_AMOUNT,
        "supply must equal mint amount ({MINT_AMOUNT}); got {supply}"
    );

    let owner_balance = token_balance_of(ctx, contract_id, position, owner_id)
        .await
        .expect("owner balance");
    assert_eq!(
        owner_balance, 0,
        "owner balance must remain 0 — mint went to the recipient; got {owner_balance}"
    );

    let peer_balance = token_balance_of(ctx, contract_id, position, peer_id)
        .await
        .expect("peer balance");
    assert_eq!(
        peer_balance, MINT_AMOUNT,
        "peer balance must equal mint amount ({MINT_AMOUNT}); got {peer_balance}"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_005b",
        %contract_id,
        %owner_id,
        %peer_id,
        supply,
        owner_balance,
        peer_balance,
        "TK-005b cross-identity mint snapshot"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
