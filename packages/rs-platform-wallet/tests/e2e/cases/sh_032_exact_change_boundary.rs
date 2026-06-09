//! SH-032 — ADVERSARIAL: boundary balance `== amount + fee` + off-by-one
//! below — exact-change correctness.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-032. Priority: P1. MEDIUM-if-fails.
//!
//! Attack: fund a single note to EXACTLY `amount + compute_minimum_shielded_fee(1)`,
//! spend `amount` (exact change → ZERO change, value conserved); then
//! off-by-one: a note of `amount + fee - 1` must be rejected
//! (`ShieldedInsufficientBalance`).
//!
//! Achievable through the public API (precise shield + public
//! `compute_minimum_shielded_fee`) — the spend reaches the backend so the
//! BACKEND's fee/value check is exercised, not just the client's. The
//! backend off-by-one INJECT arm needs the raw seam (flagged elsewhere);
//! the client off-by-one arm is asserted here.

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;
use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// The shield funds a single note of `UNSHIELD_AMOUNT + compute_minimum_shielded_fee(1)`
// (~1e9); funding must cover that note PLUS the shield's own fee, so ~2.3e9.
// UNSHIELD_AMOUNT stays modest — the boundary note size is derived from the
// REAL fee at runtime, so this case is already fee-floor-correct by construction.
const FUNDING_CREDITS: u64 = 2_300_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_032_exact_change_boundary() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_032",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    // A single-spend unshield is 1 action; the exact fee the wallet folds
    // into the requirement is `compute_minimum_shielded_fee(1)`.
    let version = PlatformVersion::latest();
    let exact_fee = compute_minimum_shielded_fee(1, version).expect("compute_minimum_shielded_fee");
    let exact_note = UNSHIELD_AMOUNT + exact_fee;

    // ---- Exact-change arm ----
    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let pw = s.test_wallet.platform_wallet();

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

    // Shield EXACTLY amount+fee into one note.
    pw.shielded_shield_from_account(0, 0, exact_note, s.test_wallet.address_signer(), prover)
        .await
        .expect("exact-note shield");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, exact_note, STEP_TIMEOUT)
        .await
        .expect("exact note never synced");

    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());
    pw.shielded_unshield_to(
        &handle.coordinator,
        0,
        &addr_dst_bech32m,
        UNSHIELD_AMOUNT,
        prover,
    )
    .await
    .expect("exact-change unshield must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("exact-change unshield destination never observed");

    // ZERO change: the note was consumed exactly, no dust change note.
    handle.sync().await;
    let change = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-unshield shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        change, 0,
        "SH-032 FINDING: exact-change unshield (note == amount+fee) left {change} change — \
         expected ZERO (no phantom dust note, fee == {exact_fee} exact)"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown exact arm");

    // ---- Off-by-one-below arm (client rejection) ----
    let s2 = setup().await.expect("e2e setup (off-by-one arm)");
    let handle2 = bind_shielded(&s2.test_wallet, &[0], &s2.ctx.workdir)
        .await
        .expect("bind_shielded off-by-one");
    let pw2 = s2.test_wallet.platform_wallet();
    let under_note = exact_note - 1;

    let addr2 = s2
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr2");
    s2.ctx
        .bank()
        .fund_address(&addr2, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address off-by-one");
    wait_for_address_balance_chain_confirmed_n(
        s2.ctx.sdk(),
        &addr2,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr2 funding never observed");
    s2.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync 2");
    pw2.shielded_shield_from_account(0, 0, under_note, s2.test_wallet.address_signer(), prover)
        .await
        .expect("under-note shield");
    wait_for_shielded_balance(&s2.test_wallet, &handle2, 0, under_note, STEP_TIMEOUT)
        .await
        .expect("under note never synced");

    let addr_dst2 = s2
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst2");
    let addr_dst2_bech32m = addr_dst2.to_bech32m_string(s2.ctx.bank().network());
    let off_by_one = pw2
        .shielded_unshield_to(
            &handle2.coordinator,
            0,
            &addr_dst2_bech32m,
            UNSHIELD_AMOUNT,
            prover,
        )
        .await;
    assert!(
        matches!(
            off_by_one,
            Err(PlatformWalletError::ShieldedInsufficientBalance { .. })
        ),
        "SH-032 FINDING: a note of amount+fee-1 ({under_note}) underpays the fee by 1 and must be \
         rejected with ShieldedInsufficientBalance; observed {off_by_one:?}"
    );

    teardown_sweep_shielded(&s2.test_wallet, &handle2, &bank_addr).await;
    s2.teardown().await.expect("teardown off-by-one arm");
}
