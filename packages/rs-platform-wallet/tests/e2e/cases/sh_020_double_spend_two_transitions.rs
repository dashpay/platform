//! SH-020 — ADVERSARIAL: double-spend the same note across two
//! transitions (Type 17) — backend MUST reject the second [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-020. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: build two distinct, individually-valid unshield transitions
//! that both spend the SAME shielded note (same nullifier), bypassing the
//! wallet's `reserve_unspent_notes` via the build-against-note seam, and
//! broadcast both. Exactly ONE must COMMIT; the second must be rejected
//! because its Orchard nullifier is already in Drive's spent set
//! (`NullifierAlreadySpentError`, code 40901).
//!
//! The verdict is read at CONSENSUS, not at `check_tx` (SD-002): both
//! transitions can pass mempool admission, so the case broadcasts both
//! and then waits for each one's COMMIT outcome. A transition counts as
//! committed only if it both passed `check_tx` AND `wait_commit_raw`
//! returned a verified proof result.
//!
//! RED if the backend commits both (double-spend — CRITICAL fund forgery)
//! or commits neither (liveness bug).

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, broadcast_raw, build_unshield_st_against_notes,
    shielded_prover, teardown_sweep_shielded, unspent_notes, wait_commit_raw,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::AddressInfo;
use dpp::address_funds::PlatformAddress;

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than the
/// per-step funding/sync gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

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

    // Capture the single synced note; build TWO unshields against it.
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(
        !notes.is_empty(),
        "expected one synced note to double-spend"
    );
    let one_note = vec![notes[0].clone()];

    let exact_fee = compute_minimum_shielded_fee(1, PlatformVersion::latest())
        .expect("compute_minimum_shielded_fee");
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

    // BEFORE state: both unshield destinations are fresh (0 credits). Read
    // them via the proof-verified on-chain path so the verdict rests on a
    // real before/after delta, not an assumption.
    let before_a = fetch_credits(s.ctx.sdk(), &dst_a).await;
    let before_b = fetch_credits(s.ctx.sdk(), &dst_b).await;

    // Broadcast BOTH first (check_tx / mempool admission) so the two
    // same-nullifier spends are in flight before either is processed.
    let bcast_a = broadcast_raw(s.ctx.sdk(), &st_a).await;
    let bcast_b = broadcast_raw(s.ctx.sdk(), &st_b).await;

    // Drive each admitted spend to its consensus outcome (block inclusion /
    // state apply), not just check_tx. The commit result is secondary
    // evidence + the rejection reason; the authoritative verdict is the
    // post-execution STATE delta below (SD-002). A check_tx-rejected spend
    // never reaches consensus, so its broadcast error IS its verdict.
    let commit_a = match &bcast_a {
        Ok(()) => wait_commit_raw(s.ctx.sdk(), &st_a, COMMIT_TIMEOUT).await,
        Err(e) => Err(crate::framework::FrameworkError::Sdk(format!(
            "check_tx rejected before consensus: {e}"
        ))),
    };
    let commit_b = match &bcast_b {
        Ok(()) => wait_commit_raw(s.ctx.sdk(), &st_b, COMMIT_TIMEOUT).await,
        Err(e) => Err(crate::framework::FrameworkError::Sdk(format!(
            "check_tx rejected before consensus: {e}"
        ))),
    };

    // AFTER state — the AUTHORITATIVE verdict. Each unshield pays its value
    // to a DISTINCT transparent address, so the on-chain economic effect of
    // double-spending one note is unambiguous: BOTH dst_a AND dst_b get
    // credited (~UNSHIELD_AMOUNT each) — one note's value materialised into
    // two outputs. The commit waits above already blocked until execution;
    // give the credited destination(s) a bounded settle on the proof-verified
    // path so the read lands after state-apply, then point-read both. A leg
    // that never credits simply times out (ignored) and reads back 0.
    let settle = Duration::from_secs(30);
    let _ =
        wait_for_address_balance_chain_confirmed_n(s.ctx.sdk(), &dst_a, UNSHIELD_AMOUNT, 1, settle)
            .await;
    let _ =
        wait_for_address_balance_chain_confirmed_n(s.ctx.sdk(), &dst_b, UNSHIELD_AMOUNT, 1, settle)
            .await;
    let after_a = fetch_credits(s.ctx.sdk(), &dst_a).await;
    let after_b = fetch_credits(s.ctx.sdk(), &dst_b).await;

    // A destination is "credited" if its on-chain balance rose toward the
    // unshield value (tolerate fee/rounding by gating at half the amount).
    let credit_threshold = UNSHIELD_AMOUNT / 2;
    let credited_a = after_a.saturating_sub(before_a) >= credit_threshold;
    let credited_b = after_b.saturating_sub(before_b) >= credit_threshold;
    let credited_count = [credited_a, credited_b].iter().filter(|c| **c).count();

    // Authoritative trace: the STATE before/after AND the secondary
    // check_tx/commit signals, so Marvin's trace shows the economic effect
    // and the consensus rejection reason side by side.
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_020",
        before_a, after_a, credited_a,
        before_b, after_b, credited_b,
        credited_count,
        check_tx_a = bcast_a.is_ok(),
        check_tx_b = bcast_b.is_ok(),
        committed_a = commit_a.is_ok(),
        committed_b = commit_b.is_ok(),
        ?commit_a,
        ?commit_b,
        "SH-020 double-spend verdict: post-execution STATE delta (authoritative) + check_tx/commit (secondary)"
    );

    // VERDICT on STATE, not status flags.
    if credited_count == 2 {
        panic!(
            "SH-020 FINDING (CRITICAL DOUBLE-SPEND): one Orchard note's value materialised \
             into TWO transparent outputs — fund forgery. dst_a {before_a}->{after_a}, \
             dst_b {before_b}->{after_b} (each ~{UNSHIELD_AMOUNT}). commit_a={commit_a:?} \
             commit_b={commit_b:?}"
        );
    }
    assert_eq!(
        credited_count,
        1,
        "SH-020 FINDING: exactly ONE same-note spend must materialise on chain; observed \
         {credited_count} credited (dst_a {before_a}->{after_a}, dst_b {before_b}->{after_b}). \
         Two = double-spend / fund forgery; zero = liveness bug (neither unshield's value \
         landed within {COMMIT_TIMEOUT:?}). check_tx[a={},b={}] commit_a={commit_a:?} \
         commit_b={commit_b:?}",
        bcast_a.is_ok(),
        bcast_b.is_ok(),
    );

    // Corroborate: the shielded note's value must have left the pool exactly
    // ONCE. A double-spend would let the same note pay out twice; with one
    // spend committed the residual change note is below SHIELD_AMOUNT.
    handle.sync().await;
    let residual = handle
        .balances(&s.test_wallet)
        .await
        .map(|b| b.get(&0).copied().unwrap_or(0))
        .unwrap_or(0);
    assert!(
        residual < SHIELD_AMOUNT,
        "SH-020: shielded balance must drop after the single committed spend; \
         observed residual {residual} >= SHIELD_AMOUNT {SHIELD_AMOUNT} (the note's value \
         did not leave the pool — investigate)"
    );

    // Secondary corroboration (best-effort): when the chain surfaces a
    // CONSENSUS error for the spend that did NOT materialise, it should be
    // nullifier-already-spent (code 40901) — evidence the replay was caught
    // for the right reason. The STATE delta above is the authoritative
    // verdict; this is skipped when no consensus error surfaced — the
    // duplicate was dropped silently at check_tx, OR (common on a quiet
    // devnet) the rejected tx simply never committed and `wait_commit_raw`
    // returned a timeout rather than a coded rejection. A timeout is NOT a
    // wrong-reason rejection, so it must not fail the test.
    let rejected_err = if !credited_a {
        format!("{commit_a:?}")
    } else {
        format!("{commit_b:?}")
    };
    let err_s = rejected_err.to_lowercase();
    let is_timeout = err_s.contains("timeout") || err_s.contains("elapsed");
    if !is_timeout && (err_s.contains("error") || err_s.contains("err(")) {
        assert!(
            err_s.contains("nullifier")
                || err_s.contains("alreadyspent")
                || err_s.contains("already spent"),
            "SH-020: the rejected spend's consensus error should be nullifier-already-spent \
             (code 40901); observed {rejected_err}"
        );
    }

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}

/// Proof-verified on-chain credit balance for `addr`, the authoritative
/// state read for the double-spend verdict. An address not yet on chain
/// (`Ok(None)`) reads as 0; a fetch error also reads as 0 and is logged —
/// a transient read failure must not be misread as "credited" (which would
/// only ever soften, never fabricate, a double-spend signal).
async fn fetch_credits(sdk: &dash_sdk::Sdk, addr: &PlatformAddress) -> u64 {
    match AddressInfo::fetch(sdk, *addr).await {
        Ok(Some(info)) => info.balance,
        Ok(None) => 0,
        Err(e) => {
            tracing::warn!(
                target: "platform_wallet::e2e::cases::sh_020",
                addr = ?addr,
                error = %e,
                "fetch_credits: AddressInfo::fetch failed; treating as 0 credits"
            );
            0
        }
    }
}
