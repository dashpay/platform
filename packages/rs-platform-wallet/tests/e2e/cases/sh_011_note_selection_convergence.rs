//! SH-011 — `select_notes_with_fee` convergence + overflow protection on
//! a real funded note set.
//!
//! **RED-by-design (Found-033)**: the `for _ in 0..NUM_NOTES` shield loop fails on
//! the third iteration. After two successful shields the shielded nonce cache for
//! the P2PKH fee-bearing input (`DeductFromInput(0)`) is not invalidated, so the
//! third call submits stale nonce=1; the server expects nonce=2 and rejects with
//! `"Invalid address nonce for P2PKH(...): expected 2, got 1"`. The server is
//! correct. See TEST_SPEC.md Found-033.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-011.
//! Priority: P2. (A unit test covers overflow at `note_selection.rs:187`;
//! this is the e2e-adjacent variant on a real funded note set.)
//!
//! Shields several small notes, then unshields an amount that forces
//! multi-note selection so the fee grows with the action count and the
//! convergence loop iterates. Also probes the `checked_add` overflow
//! guard with a degenerate `u64::MAX` request.
//!
//! Expected outcome: PASS.

use std::time::Duration;

use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// `select_shield_inputs` claims greedily from the smallest-key address,
// so all NUM_NOTES sequential shields concentrate on one funded address.
// Size every funded address to survive all of them:
// `NUM_NOTES × (SHIELD_EACH + ~1.63e8 fee) + 1e9 reserve` (see `sh_010`).
const FUNDING_CREDITS: u64 = 3_288_553_600;
const SHIELD_EACH: u64 = 600_000_000;
const NUM_NOTES: u64 = 3;
/// Above any single note (600M) yet `+ fee` below the 3-note sum (1.8e9) —
/// the raw amount alone forces multi-note selection (fee-independent), so the
/// convergence loop iterates (>1 pass) regardless of the exact shielded fee.
const MULTI_NOTE_UNSHIELD: u64 = 650_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_011_note_selection_convergence() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

    for _ in 0..NUM_NOTES {
        let addr = s
            .test_wallet
            .next_unused_address()
            .await
            .expect("derive funding addr");
        s.ctx
            .bank()
            .fund_address(&addr, FUNDING_CREDITS)
            .await
            .expect("bank.fund_address");
        wait_for_address_balance_chain_confirmed_n(
            s.ctx.sdk(),
            &addr,
            FUNDING_CREDITS,
            CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
            STEP_TIMEOUT,
        )
        .await
        .expect("funding never observed");
        s.test_wallet
            .sync_balances()
            .await
            .expect("pre-shield sync");
    }

    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    for _ in 0..NUM_NOTES {
        s.test_wallet
            .platform_wallet()
            .shielded_shield_from_account(
                &handle.coordinator,
                0,
                0,
                SHIELD_EACH,
                s.test_wallet.address_signer(),
                prover,
            )
            .await
            .expect(
                "Found-033 (RED-by-design): shielded nonce cache not invalidated after \
                 a successful broadcast — sequential shields on the same P2PKH \
                 fee-bearing input reuse the pre-broadcast stale nonce; server \
                 rejects the third call with 'expected 2, got 1'. \
                 See TEST_SPEC.md Found-033.",
            );
    }
    wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        0,
        SHIELD_EACH * NUM_NOTES,
        STEP_TIMEOUT,
    )
    .await
    .expect("shielded balance never reached all notes");

    let pw = s.test_wallet.platform_wallet();
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());

    // Overflow arm: a degenerate u64::MAX request must hit the
    // `checked_add` guard rather than wrapping.
    let overflow = pw
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            u64::MAX,
            prover,
        )
        .await;
    match overflow {
        Err(PlatformWalletError::ShieldedBuildError(msg)) => assert!(
            msg.contains("overflow"),
            "u64::MAX request must surface an overflow build error; observed {msg:?}"
        ),
        Err(PlatformWalletError::ShieldedInsufficientBalance { .. }) => {
            // Acceptable: the requirement overflow guard may live behind
            // the balance check depending on the version; either way it
            // did NOT wrap. The overflow build error is the tighter pin.
        }
        other => panic!("u64::MAX request must not wrap; observed {other:?}"),
    }

    // Convergence arm: multi-note selection succeeds and the balance
    // drops by at least the requested amount.
    let before = handle
        .balances(&s.test_wallet)
        .await
        .expect("pre-spend shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    pw.shielded_unshield_to(
        &handle.coordinator,
        &s.test_wallet.seed_bytes(),
        0,
        &addr_dst_bech32m,
        MULTI_NOTE_UNSHIELD,
        prover,
    )
    .await
    .expect("multi-note unshield must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        MULTI_NOTE_UNSHIELD,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("multi-note unshield destination never observed");
    handle.sync().await;
    let after = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-spend shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    assert!(
        before.saturating_sub(after) >= MULTI_NOTE_UNSHIELD,
        "shielded balance must drop by at least the unshield amount; before={before} after={after}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
