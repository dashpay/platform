//! SH-021 — ADVERSARIAL: nullifier replay after a confirmed spend —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-021. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: capture a note, spend it (confirmed), then rebuild a fresh
//! transition spending the SAME now-spent note (via the build-against-note
//! seam, which skips the local spent-state guard) and re-broadcast. The
//! nullifier is permanently in Drive's spent set, so the replay MUST fail
//! (`NullifierAlreadySpentError`, code 40901) regardless of client state.
//!
//! RED if the replay is accepted (double-spend via replay).

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, broadcast_raw, build_unshield_st_against_notes,
    observe_adv_verdict, shielded_prover, teardown_sweep_shielded, unspent_notes,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee` (see `sh_010`),
// with 1e8 headroom.
const FUNDING_CREDITS: u64 = 2_382_851_200;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_021_nullifier_replay_after_restart() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_021",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    // Capture the note BEFORE spending so the replay can rebuild against
    // it after it's confirmed-spent.
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(!notes.is_empty(), "expected one synced note");
    let captured = vec![notes[0].clone()];

    // First spend through the real wallet path (confirmed).
    let dst = s.test_wallet.next_unused_address().await.expect("dst");
    let dst_b32 = dst.to_bech32m_string(s.ctx.bank().network());
    s.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &dst_b32,
            UNSHIELD_AMOUNT,
            prover,
        )
        .await
        .expect("first unshield must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("first unshield destination never observed");

    // Replay: rebuild a fresh transition against the now-spent captured
    // note and broadcast. The witness still resolves (the commitment is
    // in the tree), but the nullifier is already spent on-chain.
    let exact_fee = compute_minimum_shielded_fee(1, PlatformVersion::latest())
        .expect("compute_minimum_shielded_fee");
    let dst2 = s.test_wallet.next_unused_address().await.expect("dst2");
    let replay_st = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst2,
        UNSHIELD_AMOUNT,
        exact_fee,
        &captured,
    )
    .await
    .expect("rebuild replay against spent note");
    let replay = broadcast_raw(s.ctx.sdk(), &replay_st).await;
    // Observe the TRUE verdict (consensus, not just check_tx) for Marvin.
    observe_adv_verdict(s.ctx.sdk(), "SH-021", &replay, &replay_st, COMMIT_TIMEOUT).await;
    assert!(
        replay.is_err(),
        "SH-021 FINDING (CRITICAL): replay of a confirmed-spent note was ACCEPTED — \
         double-spend via replay. result={replay:?}"
    );
    let err_s = format!("{replay:?}").to_lowercase();
    assert!(
        err_s.contains("nullifier")
            || err_s.contains("alreadyspent")
            || err_s.contains("already spent"),
        "SH-021: replay must fail nullifier-already-spent (code 40901); observed {replay:?}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
