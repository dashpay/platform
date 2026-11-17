//! TK-001c — Token transfer after sender's signing key has been rotated.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-001c. The test exercises the
//! ID-004 key-rotation helper end-to-end: an identity transfers
//! tokens with its registration-time CRITICAL key, rotates that key
//! out via `IdentityUpdateTransition`, and then transfers more
//! tokens — the second transfer must sign cleanly with the freshly
//! injected key while signing with the rotated-out key would now be
//! rejected on chain.
//!
//! Pins:
//!   - first transfer (pre-rotation, slot-3 CRITICAL key) succeeds,
//!   - rotation injects a new slot-4 CRITICAL key into the signer
//!     and disables slot 3 on chain,
//!   - second transfer (post-rotation, slot-4 CRITICAL key) succeeds
//!     and the peer's token balance reflects the cumulative move.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::data_contract::DataContract;
use dpp::identity::{Purpose, SecurityLevel};

use crate::framework::harness::E2eContext;
use crate::framework::identities::rotate_identity_authentication_key;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, wait_for_token_balance,
    TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING,
};

/// Tokens minted to the sender so it has stock for both transfers.
/// Sized comfortably above `2 * TRANSFER_AMOUNT` to leave a non-zero
/// residual on the sender at the end and let the assertions pin
/// "balance dropped by exactly `2 * TRANSFER_AMOUNT`" rather than
/// "balance is zero".
const MINT_AMOUNT: u64 = 100;

/// Tokens moved per transfer (one pre-rotation, one post-rotation).
/// `2 * TRANSFER_AMOUNT < MINT_AMOUNT` so both transfers complete.
const TRANSFER_AMOUNT: u64 = 25;

/// Per-step deadline for token-balance observations. Matches TK-001;
/// token reads round-trip the SDK + proof verifier so they need a
/// looser budget than PA-side `wait_for_balance`.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Slot index for the rotated-in CRITICAL key. The four keys created
/// by `register_identity_from_addresses` occupy slots 0..=3 (MASTER,
/// HIGH, TRANSFER, CRITICAL); slot 4 is the first free DIP-9
/// identity-key index for the rotation.
const ROTATED_KEY_INDEX: u32 = 4;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_001c_token_transfer_after_key_rotation() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_001c") {
        return;
    }

    let mut two =
        setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING)
            .await
            .expect("setup token + 2 identities");
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;
    let peer_id = two.peer.id;

    // --- mint to owner so it has stock for both transfers ------------
    {
        let owner = &two.setup.owner;
        mint_to(ctx, contract_id, position, MINT_AMOUNT, owner, owner)
            .await
            .expect("mint to owner");
    }
    wait_for_token_balance(
        ctx,
        two.setup.owner.id,
        contract_id,
        position,
        MINT_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("mint never observed on owner");

    let owner_tok_pre = token_balance_of(ctx, contract_id, position, two.setup.owner.id)
        .await
        .expect("owner token balance pre");
    assert_eq!(
        owner_tok_pre, MINT_AMOUNT,
        "owner must hold the just-minted balance pre-rotation \
         (observed={owner_tok_pre} expected={MINT_AMOUNT})"
    );

    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch data contract")
        .expect("contract not found on chain");
    let data_contract = Arc::new(data_contract);

    // --- transfer #1 (pre-rotation, signed by slot-3 CRITICAL) -------
    {
        let owner = &two.setup.owner;
        two.setup
            .setup_guard
            .base
            .test_wallet
            .platform_wallet()
            .identity()
            .token_transfer_with_signer(
                Arc::clone(&data_contract),
                position,
                owner.id,
                peer_id,
                TRANSFER_AMOUNT,
                &owner.critical_key,
                owner.signer.as_ref(),
                None,
                None,
            )
            .await
            .expect("pre-rotation token_transfer_with_signer");
    }
    wait_for_token_balance(
        ctx,
        peer_id,
        contract_id,
        position,
        TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("peer balance never observed pre-rotation");

    // --- rotate the CRITICAL auth key --------------------------------
    let old_critical_key_id =
        dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0::id(
            &two.setup.owner.critical_key,
        );
    let new_critical_key = rotate_identity_authentication_key(
        &two.setup.setup_guard.base.test_wallet,
        &mut two.setup.owner,
        ROTATED_KEY_INDEX,
        Purpose::AUTHENTICATION,
        SecurityLevel::CRITICAL,
        old_critical_key_id,
    )
    .await
    .expect("rotate identity CRITICAL key");

    // The helper updates `RegisteredIdentity::critical_key` to point
    // at the new key — assert that pin so a future helper change
    // that drops the cache update doesn't silently route subsequent
    // transitions through the disabled slot.
    assert_eq!(
        two.setup.owner.critical_key, new_critical_key,
        "rotate_identity_authentication_key must update the cached critical_key"
    );

    // --- transfer #2 (post-rotation, signed by slot-4 CRITICAL) -----
    {
        let owner = &two.setup.owner;
        two.setup
            .setup_guard
            .base
            .test_wallet
            .platform_wallet()
            .identity()
            .token_transfer_with_signer(
                Arc::clone(&data_contract),
                position,
                owner.id,
                peer_id,
                TRANSFER_AMOUNT,
                &owner.critical_key,
                owner.signer.as_ref(),
                None,
                None,
            )
            .await
            .expect("post-rotation token_transfer_with_signer");
    }
    wait_for_token_balance(
        ctx,
        peer_id,
        contract_id,
        position,
        2 * TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("peer balance never observed post-rotation");

    // --- post-transfer reads -----------------------------------------
    let owner_tok_post = token_balance_of(ctx, contract_id, position, two.setup.owner.id)
        .await
        .expect("owner token balance post");
    let peer_tok_post = token_balance_of(ctx, contract_id, position, peer_id)
        .await
        .expect("peer token balance post");

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_001c",
        owner = ?two.setup.owner.id,
        peer = ?peer_id,
        owner_tok_pre,
        owner_tok_post,
        peer_tok_post,
        "post-rotation snapshot"
    );

    assert_eq!(
        owner_tok_post,
        MINT_AMOUNT - 2 * TRANSFER_AMOUNT,
        "owner token balance must drop by exactly 2 * TRANSFER_AMOUNT \
         (observed={owner_tok_post})"
    );
    assert_eq!(
        peer_tok_post,
        2 * TRANSFER_AMOUNT,
        "peer token balance must equal the cumulative transfer amount \
         (observed={peer_tok_post})"
    );

    two.setup.setup_guard.teardown().await.expect("teardown");
}
