//! TK-004 — Token transfer round-trip + fee/balance accounting.
//!
//! P0 foundation case. Validates that an A → B → A token round-trip
//! preserves owner balance modulo platform fees, that the
//! intermediate recipient's balance is observable on chain, and
//! that total supply stays untouched across pure transfers (only
//! `mint`/`burn` move the supply needle).
//!
//! Composes Wave G's [`setup_with_token_and_two_identities`] +
//! [`mint_to`] with a direct
//! [`TokenTransferTransitionBuilder`]/[`Sdk::token_transfer`] call —
//! the framework does not (yet) ship a typed `transfer_tokens`
//! helper, and inlining the SDK call here keeps the assertion
//! surface explicit (sender + recipient ids visible at the call
//! site) while we wait on Wave 2 / Wave 4 to decide whether the
//! helper is worth promoting.
//!
//! Editorial note: the owner mint and both transfers sign with
//! [`RegisteredIdentity::critical_key`] (AUTHENTICATION + CRITICAL,
//! KeyID 3), matching `tokens::mint_to`. `TokenBaseTransition`'s
//! `IdentitySignedV0::security_level_requirement` returns only
//! `vec![SecurityLevel::CRITICAL]`; signing with HIGH yields
//! `InvalidSignaturePublicKeySecurityLevelError` at chain validation.
//! See the editorial note in `tokens.rs` for the contract-create
//! case where HIGH is the canonical signing level.
//!
//! Gated behind the `e2e` cargo feature so a stock `cargo test -p platform-wallet`
//! stays green for contributors and CI jobs that lack a funded
//! testnet bank wallet, live DAPI access, and the operator `.env`.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities_with_step_timeout, token_balance_of,
    token_supply_of, wait_for_token_balance, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING_ACTIVE,
};

/// Tokens minted to the owner before the round-trip starts. Picked
/// well above `TRANSFER_AMOUNT` so post-roundtrip the owner's
/// balance is still strictly positive even if a chain-side delta
/// shifts (Wave 4 will pin exact arithmetic).
const MINT_AMOUNT: u64 = 1_000;

/// Tokens A sends to B, then B sends back to A. Same value both
/// directions so the round-trip is symmetric and the owner-balance
/// invariant is the cleanest: pre-roundtrip == post-roundtrip
/// (token transfers do not currently charge a token-side fee — the
/// fee is paid in credits).
const TRANSFER_AMOUNT: u64 = 250;

/// Per-step deadline for balance observations after a broadcast.
/// `mint_to` and `Sdk::token_transfer` both await proof internally,
/// so this is a safety net for the trusted-context-provider warmup
/// rather than an actual sync wait.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Bootstrap-step budget for the two-identity funding hop. 60 s is
/// too tight under `--test-threads=14` when both identities fund
/// 35 150 100 000 duff concurrently — the broadcast lands but
/// `wait_for_balance`'s chain-confirmed gate doesn't clear inside
/// the default deadline. 120 s is plenty without softening the
/// framework-wide default.
const SETUP_STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_004_token_transfer_round_trip() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("e2e context init failed");
    if ctx.skip_if_bank_floor_unmet("tk_004") {
        return;
    }

    // Two identities funded for one contract-create + a handful of
    // token-action broadcasts each. `setup_with_token_and_two_identities`
    // also handles the Wave 1 MASTER-signing surface — if the chain
    // rejects, the failure rolls up here and is caller-visible in
    // the test summary as a fixture build failure.
    let two = setup_with_token_and_two_identities_with_step_timeout(
        ctx,
        TK_OWNER_FUNDING_SIMPLE,
        TK_PEER_FUNDING_ACTIVE,
        SETUP_STEP_TIMEOUT,
    )
    .await
    .expect("TK-004: token + two-identities setup failed");

    let TokenTwoIdentitiesSetup {
        setup,
        peer: identity_b,
    } = two;
    let contract_id = setup.contract_id;
    let position = setup.token_position;
    let identity_a = setup.owner.clone_for_token_setup_local();

    // Snapshot the owner's pre-mint balance so the post-roundtrip
    // assertion can isolate the arithmetic. `token_balance_of` returns
    // 0 for an identity that has never held the token, so an explicit
    // read here doubles as a sanity check that the contract is wired
    // for the right pair of ids.
    let a_pre_mint = token_balance_of(ctx, contract_id, position, identity_a.id)
        .await
        .expect("read A pre-mint balance");
    let supply_pre_mint = token_supply_of(ctx, contract_id, position)
        .await
        .expect("read pre-mint supply");

    assert_eq!(
        a_pre_mint, 0,
        "fresh identity must hold zero tokens before the test mints"
    );
    assert_eq!(
        supply_pre_mint, 0,
        "fresh permissive contract must declare zero supply before the test mints"
    );

    // ------ mint owner-side seed balance -----------------------------
    mint_to(
        ctx,
        contract_id,
        position,
        MINT_AMOUNT,
        &identity_a,
        &identity_a,
    )
    .await
    .expect("mint to owner failed");

    // The mint helper proves on the way out, but the SDK's read-side
    // is a fresh fetch — wait until the proof view sees the new
    // balance before continuing. `wait_for_token_balance` returns
    // the observed value so the next assertion uses live state, not
    // the polled threshold.
    let a_post_mint = wait_for_token_balance(
        ctx,
        identity_a.id,
        contract_id,
        position,
        MINT_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("A post-mint balance never observed");

    assert_eq!(
        a_post_mint, MINT_AMOUNT,
        "owner mint must credit exactly MINT_AMOUNT to the owner"
    );

    let supply_post_mint = token_supply_of(ctx, contract_id, position)
        .await
        .expect("read post-mint supply");
    assert_eq!(
        supply_post_mint, MINT_AMOUNT,
        "total supply must rise by MINT_AMOUNT after the owner mint"
    );

    // ------ A -> B transfer -----------------------------------------
    // Snapshot A's identity-credit balance pre-transfer so we can
    // assert the spec's fee-side requirement (`actual_fee > 0`,
    // credit balance decreased by the transfer fee). Token transfers
    // settle in tokens — the credit-side fee is charged against the
    // sender's identity credits.
    let a_credits_pre_send = IdentityBalance::fetch(ctx.sdk(), identity_a.id)
        .await
        .expect("read A pre-send credits")
        .expect("A identity present");

    transfer_token(
        ctx,
        contract_id,
        position,
        TRANSFER_AMOUNT,
        &identity_a,
        identity_b.id,
    )
    .await
    .expect("transfer A -> B failed");

    let a_credits_post_send = IdentityBalance::fetch(ctx.sdk(), identity_a.id)
        .await
        .expect("read A post-send credits")
        .expect("A identity present");
    let a_send_fee = a_credits_pre_send.saturating_sub(a_credits_post_send);
    assert!(
        a_send_fee > 0,
        "A's credit balance must decrease by a positive transfer fee \
         (pre={a_credits_pre_send} post={a_credits_post_send})"
    );
    assert!(
        a_credits_post_send < a_credits_pre_send,
        "post-send credits must be strictly less than pre-send"
    );

    let b_intermediate = wait_for_token_balance(
        ctx,
        identity_b.id,
        contract_id,
        position,
        TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("B intermediate balance never observed");
    assert_eq!(
        b_intermediate, TRANSFER_AMOUNT,
        "B must observe exactly TRANSFER_AMOUNT after A's send"
    );

    let a_after_send = token_balance_of(ctx, contract_id, position, identity_a.id)
        .await
        .expect("read A post-send balance");
    assert_eq!(
        a_after_send,
        MINT_AMOUNT - TRANSFER_AMOUNT,
        "A must lose exactly TRANSFER_AMOUNT (transfers do not move token supply)"
    );

    let supply_mid = token_supply_of(ctx, contract_id, position)
        .await
        .expect("read mid-roundtrip supply");
    assert_eq!(
        supply_mid, MINT_AMOUNT,
        "total supply must stay flat across a pure A -> B transfer"
    );

    // ------ B -> A transfer (close the loop) ------------------------
    let b_credits_pre_send = IdentityBalance::fetch(ctx.sdk(), identity_b.id)
        .await
        .expect("read B pre-send credits")
        .expect("B identity present");

    transfer_token(
        ctx,
        contract_id,
        position,
        TRANSFER_AMOUNT,
        &identity_b,
        identity_a.id,
    )
    .await
    .expect("transfer B -> A failed");

    let b_credits_post_send = IdentityBalance::fetch(ctx.sdk(), identity_b.id)
        .await
        .expect("read B post-send credits")
        .expect("B identity present");
    let b_send_fee = b_credits_pre_send.saturating_sub(b_credits_post_send);
    assert!(
        b_send_fee > 0,
        "B's credit balance must decrease by a positive transfer fee on the return leg \
         (pre={b_credits_pre_send} post={b_credits_post_send})"
    );

    let a_post_roundtrip = wait_for_token_balance(
        ctx,
        identity_a.id,
        contract_id,
        position,
        MINT_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("A post-roundtrip balance never observed");

    let b_post_roundtrip = token_balance_of(ctx, contract_id, position, identity_b.id)
        .await
        .expect("read B post-roundtrip balance");
    let supply_post_roundtrip = token_supply_of(ctx, contract_id, position)
        .await
        .expect("read post-roundtrip supply");

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_004",
        ?contract_id,
        position,
        a_pre_mint,
        a_post_mint,
        b_intermediate,
        a_after_send,
        a_post_roundtrip,
        b_post_roundtrip,
        supply_post_mint,
        supply_post_roundtrip,
        a_send_fee,
        b_send_fee,
        "TK-004: round-trip balance / supply snapshot"
    );

    // Round-trip identity invariants. Token transfers settle in
    // tokens (no token-side fee) — the credit-side fee for the
    // transfer transition itself is charged against each sender's
    // identity credits, not against the token balance, so on the
    // token axis the round-trip is exact.
    assert_eq!(
        a_post_roundtrip, MINT_AMOUNT,
        "A's post-roundtrip token balance must equal its post-mint balance \
         (transfers do not charge a token-side fee)"
    );
    assert_eq!(
        b_post_roundtrip, 0,
        "B must hold zero tokens after sending the same amount back to A"
    );
    assert_eq!(
        supply_post_roundtrip, MINT_AMOUNT,
        "total supply must equal the minted amount across the entire round-trip \
         (no mint or burn after the initial seed)"
    );

    setup.setup_guard.teardown().await.expect("teardown");
}

/// Local wrapper around [`Sdk::token_transfer`] / the
/// [`TokenTransferTransitionBuilder`] for the round-trip. Lives in
/// the case file (rather than `tokens.rs`) per Wave 2-β scope —
/// framework changes are off-limits here. Promote when a second
/// case needs the same shape.
async fn transfer_token(
    ctx: &'static crate::framework::harness::E2eContext,
    contract_id: dpp::prelude::Identifier,
    position: dpp::data_contract::TokenContractPosition,
    amount: u64,
    sender: &crate::framework::wallet_factory::RegisteredIdentity,
    recipient_id: dpp::prelude::Identifier,
) -> Result<(), String> {
    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .map_err(|err| format!("fetch contract {contract_id} for transfer: {err}"))?
        .ok_or_else(|| format!("contract {contract_id} not found on chain"))?;

    let builder = TokenTransferTransitionBuilder::new(
        Arc::new(data_contract),
        position,
        sender.id,
        recipient_id,
        amount,
    );

    ctx.sdk()
        .token_transfer(builder, &sender.critical_key, sender.signer.as_ref())
        .await
        .map_err(|err| format!("token_transfer {} -> {}: {err}", sender.id, recipient_id))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Local imports — kept at the bottom because they're entirely
// internal to TK-004's tooling. The harness re-exports `RegisteredIdentity`
// only via `tokens::TokenSetup`/`TokenTwoIdentitiesSetup`, so the
// case file pulls them through the explicit framework path.
// ---------------------------------------------------------------------------

use crate::framework::tokens::TokenTwoIdentitiesSetup;

/// Mirror of `tokens::CloneForTokenSetup::clone_for_token_setup`,
/// scoped to the case so we don't reach into framework internals.
/// Wave G's helper is `pub(super)`-ish (defined as a local trait
/// inside `tokens.rs`); we replicate the few lines here rather than
/// widen its visibility.
trait CloneForTokenSetupLocal {
    fn clone_for_token_setup_local(&self) -> Self;
}

impl CloneForTokenSetupLocal for crate::framework::wallet_factory::RegisteredIdentity {
    fn clone_for_token_setup_local(&self) -> Self {
        crate::framework::wallet_factory::RegisteredIdentity {
            id: self.id,
            master_key: self.master_key.clone(),
            high_key: self.high_key.clone(),
            transfer_key: self.transfer_key.clone(),
            critical_key: self.critical_key.clone(),
            signer: Arc::clone(&self.signer),
            identity_index: self.identity_index,
            funding: self.funding,
        }
    }
}
