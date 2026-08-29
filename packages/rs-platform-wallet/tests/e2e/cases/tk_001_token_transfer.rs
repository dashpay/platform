//! TK-001 — Token transfer between two identities (happy path).
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-001. Owner mints 100 tokens to
//! itself, then transfers 50 to a peer. Pins:
//!   - sender token balance drops by exactly the transferred amount,
//!   - peer token balance grows by exactly the transferred amount,
//!   - sender's identity credit balance drops by `> 0` (token transfer
//!     pays its fee in credits, not tokens).
//!
//! Gated behind the `e2e` cargo feature so a stock workspace `cargo test` stays
//! green for contributors that lack the bank mnemonic and live testnet
//! access. Operator setup mirrors `cases/transfer.rs`.

use std::sync::Arc;
use std::time::Duration;

use dash_platform_macros::stack_size;
use dash_sdk::platform::Fetch;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::harness::E2eContext;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, wait_for_token_balance,
    TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING,
};

/// Tokens minted to the sender before the transfer. Sized comfortably
/// above `TRANSFER_AMOUNT` so the post-transfer assertion can pin the
/// residual without being sensitive to mint-side rounding.
const MINT_AMOUNT: u64 = 100;

/// Tokens moved from owner → peer. Picked to leave a non-zero residual
/// on the sender so we can pin "balance decreased by exactly N" rather
/// than "balance is now zero".
const TRANSFER_AMOUNT: u64 = 50;

/// Per-step deadline for token-balance observations. Longer than the
/// PA-side `wait_for_balance` budget because token reads round-trip
/// the SDK + proof verifier rather than a wallet-cached map.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

// 16MB stack (driving thread + tokio worker threads) — the deep SDK GroveDB
// proof/quorum verification recursion overflows libtest's ~2MB thread stack.
// The macro strips `async` and drives the body on a multi-threaded tokio
// runtime whose workers also get the 16MB stack, so this test's concurrent
// work has the same recursion budget on every thread.
#[stack_size(16 * 1024 * 1024, multi_thread, worker_threads = 12)]
#[test]
async fn tk_001_token_transfer_between_identities() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_001") {
        return;
    }

    let two = setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING)
        .await
        .expect("setup token + 2 identities");
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;
    let owner = &two.setup.owner;
    let peer = &two.peer;

    // --- mint to owner so it has stock to transfer -------------------
    mint_to(ctx, contract_id, position, MINT_AMOUNT, owner, owner)
        .await
        .expect("mint to owner");
    wait_for_token_balance(
        ctx,
        owner.id,
        contract_id,
        position,
        MINT_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("mint never observed on owner");

    let owner_tok_pre = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner token balance pre");
    let peer_tok_pre = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer token balance pre");
    let owner_credits_pre = Identity::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner identity pre")
        .expect("owner identity must exist after registration")
        .balance();

    assert_eq!(
        owner_tok_pre, MINT_AMOUNT,
        "owner must hold the just-minted balance (observed={owner_tok_pre} expected={MINT_AMOUNT})"
    );
    assert_eq!(
        peer_tok_pre, 0,
        "peer must start with zero token balance (observed={peer_tok_pre})"
    );

    // --- transfer ----------------------------------------------------
    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch data contract")
        .expect("contract not found on chain");

    two.setup
        .setup_guard
        .base
        .test_wallet
        .platform_wallet()
        .identity()
        .token_transfer_with_signer(
            Arc::new(data_contract),
            position,
            owner.id,
            peer.id,
            TRANSFER_AMOUNT,
            &owner.critical_key,
            owner.signer.as_ref(),
            None,
            None,
        )
        .await
        .expect("token_transfer_with_signer");

    // Wait for the proof-verified peer balance to hit the target.
    wait_for_token_balance(
        ctx,
        peer.id,
        contract_id,
        position,
        TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("peer balance never observed");

    // --- post-transfer reads ----------------------------------------
    let owner_tok_post = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner token balance post");
    let peer_tok_post = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer token balance post");
    let owner_credits_post = Identity::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner identity post")
        .expect("owner identity must still exist post-transfer")
        .balance();

    let credit_fee = owner_credits_pre.saturating_sub(owner_credits_post);

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_001",
        owner = ?owner.id,
        peer = ?peer.id,
        owner_tok_pre,
        owner_tok_post,
        peer_tok_pre,
        peer_tok_post,
        owner_credits_pre,
        owner_credits_post,
        credit_fee,
        "post-transfer snapshot"
    );

    assert_eq!(
        owner_tok_post,
        owner_tok_pre - TRANSFER_AMOUNT,
        "owner token balance must drop by exactly TRANSFER_AMOUNT (observed={owner_tok_post})",
    );
    assert_eq!(
        peer_tok_post,
        peer_tok_pre + TRANSFER_AMOUNT,
        "peer token balance must rise by exactly TRANSFER_AMOUNT (observed={peer_tok_post})",
    );
    assert!(
        credit_fee > 0,
        "token transfer must charge a non-zero credit fee \
         (pre={owner_credits_pre} post={owner_credits_post})"
    );
    assert!(
        credit_fee < owner_credits_pre,
        "credit fee implausibly large: {credit_fee} >= owner_credits_pre {owner_credits_pre}"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
