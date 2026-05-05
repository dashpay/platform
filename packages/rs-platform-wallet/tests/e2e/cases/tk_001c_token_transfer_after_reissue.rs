//! TK-001c — Token transfer after sender's signing key has been rotated.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-001c. Depends on ID-004
//! (identity-update — add + disable a key). The harness's
//! `SeedBackedIdentitySigner` only pre-derives keys for `key_index ∈
//! 0..DEFAULT_GAP_LIMIT`; rotating in a freshly-issued key needs a
//! `derive_identity_key`-driven cache-injection helper that does not
//! exist on the Wave 1 baseline (see `TEST_SPEC.md` § ID-004 STUB).
//!
//! Wave 2-α writes the body up to the rotation step and panics there
//! with a TODO so Wave 3+ can wire in the new helper without rewriting
//! the surrounding setup. Once ID-004 lands, replace the panic with:
//!   1. `update_identity` (add new HIGH key) signed by `master_key`,
//!   2. `update_identity` (disable old HIGH key) signed by master,
//!   3. transfer signed by the **new** key,
//!   4. (sub-case) transfer signed by the disabled key → typed error.

use std::time::Duration;

use crate::framework::harness::E2eContext;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, wait_for_token_balance,
    DEFAULT_TK_FUNDING,
};

/// Tokens minted to the sender so it has stock for the post-rotation
/// transfer.
const MINT_AMOUNT: u64 = 100;

/// Per-step deadline for token-balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with cargo test -- --ignored"]
async fn tk_001c_token_transfer_after_key_rotation() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");

    let two = setup_with_token_and_two_identities(ctx, DEFAULT_TK_FUNDING)
        .await
        .expect("setup token + 2 identities");
    let contract_id = two.setup.contract_id;
    let position = two.setup.token_position;
    let owner = &two.setup.owner;
    let _peer = &two.peer;

    // Mint stock so the post-rotation transfer has something to move.
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
    assert_eq!(
        owner_tok_pre, MINT_AMOUNT,
        "owner must hold the just-minted balance pre-rotation \
         (observed={owner_tok_pre} expected={MINT_AMOUNT})"
    );

    // ---- key rotation step: requires ID-004 helper -----------------
    //
    // Two pieces are missing:
    //  - a `derive_identity_key(identity_index, key_index, purpose,
    //    security_level)` helper that hands back a fresh
    //    `IdentityPublicKey` outside the gap window; AND
    //  - a way to inject the matching private bytes into the test's
    //    `SeedBackedIdentitySigner` so subsequent transfers sign with
    //    the new key.
    //
    // Both are tracked under TEST_SPEC.md § ID-004 (STUB). Once they
    // land, replace this panic with the rotate + transfer + sub-case
    // sequence outlined in the module docs.
    panic!(
        "TK-001c: requires ID-004 key-rotation helper \
         (derive_identity_key + signer cache injection) — see TEST_SPEC.md § ID-004"
    );

    // Unreachable until ID-004 lands; left in place so the eventual
    // implementor sees the assertion shape the spec asks for.
    #[allow(unreachable_code)]
    {
        two.setup.setup_guard.teardown().await.expect("teardown");
    }
}
