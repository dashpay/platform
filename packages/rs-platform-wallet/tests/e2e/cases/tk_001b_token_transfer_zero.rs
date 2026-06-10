//! TK-001b — Token transfer with `amount = 0`.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-001b. Pins the **(a) Reject**
//! contract: validation surfaces an `InvalidTokenAmountError` (token
//! amount must be `> 0` and `≤ MAX_DISTRIBUTION_PARAM`, see
//! `rs-dpp/.../token_transfer_transition/validate_structure/v0/mod.rs`),
//! no broadcast lands, and both balances stay unchanged.
//!
//! The chain rejects zero-amount before broadcast / proof, so we
//! simply assert the API call returns an error and that the post-call
//! balances match the pre-call ones.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::harness::E2eContext;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, wait_for_token_balance,
    TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING,
};

/// Tokens minted to the sender so the pre-condition (sender holds a
/// non-zero balance) holds. Mirrors TK-001's mint amount.
const MINT_AMOUNT: u64 = 100;

/// Per-step deadline for token-balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_001b_token_transfer_zero_rejected() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_001b") {
        return;
    }

    let two = setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING)
        .await
        .expect("setup token + 2 identities");
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;
    let owner = &two.setup.owner;
    let peer = &two.peer;

    // Mint to owner so it has stock — without this the transfer might
    // fail on insufficient balance instead of the zero-amount guard,
    // which would muddy the assertion.
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
    // Snapshot the owner's identity-credit balance pre-call so we
    // can assert the rejected transition charged zero credits
    // (the spec's "no broadcast, no fee" contract).
    let owner_credits_pre = Identity::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner identity pre")
        .expect("owner identity must exist pre-rejection")
        .balance();

    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch data contract")
        .expect("contract not found on chain");

    let result = two
        .setup
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
            0,
            &owner.critical_key,
            owner.signer.as_ref(),
            None,
            None,
        )
        .await;

    // `TransferResult` doesn't implement `Debug`, so use a manual
    // match instead of `expect_err`.
    let err = match result {
        Ok(_) => panic!("zero-amount transfer must be rejected, but the call returned Ok"),
        Err(e) => e,
    };
    let err_msg = format!("{err}");
    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_001b",
        error = %err_msg,
        "zero-amount transfer rejected (as expected)"
    );

    // Pin the typed error shape: rs-dpp surfaces zero amounts as
    // InvalidTokenAmount; the SDK preserves the variant in its
    // stringified error so a substring match is the cheapest stable
    // contract while we wait for a typed-error accessor in dash-sdk.
    assert!(
        err_msg.contains("InvalidTokenAmount") || err_msg.to_lowercase().contains("amount"),
        "rejection must reference the invalid-amount validator \
         (observed: {err_msg})"
    );

    // Re-read balances; both must be unchanged (no broadcast, no fee).
    let owner_tok_post = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner token balance post");
    let peer_tok_post = token_balance_of(ctx, contract_id, position, peer.id)
        .await
        .expect("peer token balance post");
    let owner_credits_post = Identity::fetch(ctx.sdk(), owner.id)
        .await
        .expect("fetch owner identity post")
        .expect("owner identity must still exist post-rejection")
        .balance();

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_001b",
        owner = ?owner.id,
        peer = ?peer.id,
        owner_tok_pre,
        owner_tok_post,
        peer_tok_pre,
        peer_tok_post,
        owner_credits_pre,
        owner_credits_post,
        "post-rejection snapshot"
    );

    assert_eq!(
        owner_tok_post, owner_tok_pre,
        "rejected transfer must not alter sender token balance"
    );
    assert_eq!(
        peer_tok_post, peer_tok_pre,
        "rejected transfer must not alter recipient token balance"
    );
    // Spec § TK-001b: rejected (no broadcast, no fee). A regression
    // that starts charging credits for client-side-rejected
    // transitions would surface as a non-zero delta here.
    assert_eq!(
        owner_credits_post, owner_credits_pre,
        "rejected transfer must not charge identity credits \
         (pre={owner_credits_pre} post={owner_credits_post})"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
