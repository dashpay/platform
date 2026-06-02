//! SH-020 — ADVERSARIAL: double-spend the same note across two
//! transitions (Type 17) — backend MUST reject the second [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-020. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: build two distinct, individually-valid unshield transitions
//! that both spend the SAME shielded note (same nullifier), bypassing the
//! wallet's `reserve_unspent_notes` via the build-against-note seam, and
//! broadcast both. Exactly ONE must be accepted; the second must be
//! rejected because its Orchard nullifier is already in Drive's spent set
//! (`NullifierAlreadySpentError`, code 40901).
//!
//! RED if the backend accepts both (double-spend — CRITICAL fund forgery)
//! or accepts neither (liveness bug).

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, broadcast_raw, build_unshield_st_against_notes,
    shielded_prover, teardown_sweep_shielded, unspent_notes, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_020_double_spend_two_transitions() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_020",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
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
        .shielded_shield_from_account(0, 0, SHIELD_AMOUNT, s.test_wallet.address_signer(), prover)
        .await
        .expect("shield_from_account");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    // Capture the single synced note; build TWO unshields against it.
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(
        !notes.is_empty(),
        "expected one synced note to double-spend"
    );
    let one_note = vec![notes[0].clone()];

    let exact_fee = compute_minimum_shielded_fee(1, PlatformVersion::latest());
    let dst_a = s.test_wallet.next_unused_address().await.expect("dst_a");
    let dst_b = s.test_wallet.next_unused_address().await.expect("dst_b");

    let st_a = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst_a,
        UNSHIELD_AMOUNT,
        exact_fee,
        &one_note,
    )
    .await
    .expect("build first unshield against note");
    let st_b = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst_b,
        UNSHIELD_AMOUNT,
        exact_fee,
        &one_note,
    )
    .await
    .expect("build second unshield against the SAME note");

    let r_a = broadcast_raw(s.ctx.sdk(), &st_a).await;
    let r_b = broadcast_raw(s.ctx.sdk(), &st_b).await;

    let accepted = [r_a.is_ok(), r_b.is_ok()].iter().filter(|ok| **ok).count();
    assert_eq!(
        accepted,
        1,
        "SH-020 FINDING (CRITICAL): exactly ONE of two same-note spends must be accepted; \
         observed {accepted} accepted (a==Ok:{}, b==Ok:{}). Both-accepted = double-spend / \
         fund forgery; neither = liveness bug. r_a={r_a:?} r_b={r_b:?}",
        r_a.is_ok(),
        r_b.is_ok()
    );
    // The rejected one must fail with a nullifier-already-spent class
    // error, not a generic failure.
    let rejected_err = if r_a.is_err() { r_a } else { r_b };
    let err_s = format!("{rejected_err:?}").to_lowercase();
    assert!(
        err_s.contains("nullifier") || err_s.contains("alreadyspent") || err_s.contains("already spent"),
        "SH-020: the rejected spend must fail nullifier-already-spent (code 40901); observed {rejected_err:?}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
