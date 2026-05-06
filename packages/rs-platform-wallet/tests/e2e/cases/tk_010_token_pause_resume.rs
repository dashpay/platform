//! TK-010 — Pause and resume token (emergency action).
//!
//! Two-identity setup (owner + peer). The owner pauses the token,
//! attempts a transfer (must be rejected with a "token is paused"
//! consensus error), then resumes and retries the transfer.
//!
//! Wave 2 stub: `#[ignore]`d so a stock `cargo test` stays green.
//! Wave 4 runs it against a live testnet.
//!
//! Spec drift note: TEST_SPEC.md asks for a positive `actual_fee` on
//! both pause and resume `EmergencyActionResult`s, but the bare SDK
//! `EmergencyActionResult` enum (rs-sdk/src/platform/tokens/
//! transitions/emergency_action.rs) does not surface a fee field —
//! that lives in DET's task wrapper. Wave 4 will either fold a fee
//! accessor into the SDK result or read fees from credit-balance
//! deltas; until then the `actual_fee` assertion is a TODO.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::tokens::builders::emergency_action::TokenEmergencyActionTransitionBuilder;
use dash_sdk::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;
use dash_sdk::platform::Fetch;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, token_is_paused_of,
    DEFAULT_TK_FUNDING, DEFAULT_TOKEN_POSITION,
};

const MINT_AMOUNT: u64 = 1_000;
/// Initial peer seed (owner mints this amount to peer pre-pause) so
/// the spec's "both identities have a non-zero token balance"
/// pre-condition holds.
const PEER_SEED: u64 = 25;
const SEED_TRANSFER: u64 = 100;
const POST_RESUME_TRANSFER: u64 = 50;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_010_token_pause_blocks_transfers_then_resume_restores() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    let s = setup_with_token_and_two_identities(ctx, DEFAULT_TK_FUNDING)
        .await
        .expect("token + two identities setup");

    let owner = &s.setup.owner;
    let peer = &s.peer;
    let contract_id = s.setup.contract_id;
    let position = s.setup.token_position;

    // Step 1: owner mints to self and seeds peer with a small
    // balance. Spec § TK-010 precondition asks for "two identities;
    // both have a non-zero token balance" — the pre-pause peer mint
    // makes the regulatory case (pause must block transfers from a
    // funded peer too) actually reachable.
    mint_to(ctx, contract_id, position, MINT_AMOUNT, owner, owner)
        .await
        .expect("owner mint to self");
    mint_to(ctx, contract_id, position, PEER_SEED, peer, owner)
        .await
        .expect("owner mint to peer (precondition seed)");

    // Pre-pause sanity: balances are exactly the minted amounts on a
    // fresh contract (no historical seed possible because the
    // contract was freshly deployed in setup).
    let owner_pre = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner balance pre-pause");
    assert_eq!(
        owner_pre, MINT_AMOUNT,
        "owner balance must equal the freshly-minted amount (got {owner_pre})"
    );
    let peer_pre = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer balance pre-pause");
    assert_eq!(
        peer_pre, PEER_SEED,
        "peer balance must equal PEER_SEED post seed-mint (got {peer_pre})"
    );
    let paused_before = token_is_paused_of(ctx, contract_id, position)
        .await
        .expect("paused flag pre-pause");
    assert!(!paused_before, "token must start unpaused");

    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch data contract for pause builder")
        .expect("contract on chain");
    let data_contract = Arc::new(data_contract);

    // Step 2: owner pauses.
    let pause_builder =
        TokenEmergencyActionTransitionBuilder::pause(data_contract.clone(), position, owner.id);
    ctx.sdk()
        .token_emergency_action(pause_builder, &owner.critical_key, owner.signer.as_ref())
        .await
        .expect("pause emergency action");

    // Wave G's `token_is_paused_of` must flip to true.
    let paused_after = token_is_paused_of(ctx, contract_id, position)
        .await
        .expect("paused flag post-pause");
    assert!(paused_after, "token must report paused after pause action");

    // Step 3: owner transfer must be rejected with a "token is paused"
    // typed error. We match on the consensus-error error display string;
    // the upstream type is `dpp::...::TokenIsPausedError`.
    let transfer_builder = TokenTransferTransitionBuilder::new(
        data_contract.clone(),
        position,
        owner.id,
        peer.id,
        SEED_TRANSFER,
    );
    let result = ctx
        .sdk()
        .token_transfer(transfer_builder, &owner.critical_key, owner.signer.as_ref())
        .await;
    // `TransferResult` doesn't impl `Debug`, so unpack with `match` rather than
    // `expect_err`.
    let err_str = match result {
        Ok(_) => panic!("transfer must fail while paused"),
        Err(err) => err.to_string(),
    };
    assert!(
        err_str.contains("paused") || err_str.contains("TokenIsPaused"),
        "expected a 'token paused' typed error class, got: {err_str}"
    );

    // Step 4: owner resumes.
    let resume_builder =
        TokenEmergencyActionTransitionBuilder::resume(data_contract.clone(), position, owner.id);
    ctx.sdk()
        .token_emergency_action(resume_builder, &owner.critical_key, owner.signer.as_ref())
        .await
        .expect("resume emergency action");

    let paused_resumed = token_is_paused_of(ctx, contract_id, position)
        .await
        .expect("paused flag post-resume");
    assert!(
        !paused_resumed,
        "token must report not-paused after resume action"
    );

    // Step 5: owner retries the transfer; succeeds.
    let retry_builder = TokenTransferTransitionBuilder::new(
        data_contract,
        position,
        owner.id,
        peer.id,
        POST_RESUME_TRANSFER,
    );
    ctx.sdk()
        .token_transfer(retry_builder, &owner.critical_key, owner.signer.as_ref())
        .await
        .expect("post-resume transfer");

    let peer_post = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer balance post-resume");
    // Spec § TK-010 step 5: "succeeds" — peer balance grows by
    // exactly POST_RESUME_TRANSFER from its pre-pause value (the
    // mid-pause attempt was rejected, so it had no effect).
    assert_eq!(
        peer_post,
        peer_pre + POST_RESUME_TRANSFER,
        "peer balance must equal the seed plus the post-resume transfer \
         (pre={peer_pre} post={peer_post} expected_delta={POST_RESUME_TRANSFER})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_010",
        ?contract_id,
        owner_pre,
        peer_post,
        "TK-010 pause/resume round-trip complete"
    );

    // TODO(spec-drift): once SDK's EmergencyActionResult exposes
    // actual_fee, assert pause_fee > 0 and resume_fee > 0 per
    // TEST_SPEC.md TK-010.

    let _ = STEP_TIMEOUT; // currently unused — kept for future wait_for_token_balance hooks.

    s.setup.setup_guard.teardown().await.expect("teardown");
}
