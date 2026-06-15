//! SH-033 — ADVERSARIAL: duplicate nullifier WITHIN one bundle — backend
//! MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-033. Priority: P1.
//! CRITICAL-if-it-fails (double-spend within one tx).
//!
//! Attack: build one Type-17 unshield whose Orchard bundle spends the
//! same note TWICE (two actions, identical nullifier) by passing
//! `[note, note]` to the build-against-note seam, then broadcast. A
//! duplicate nullifier within one bundle must fail validation before any
//! state write.
//!
//! The build itself may reject the duplicate (a client-side guard), in
//! which case the dup never reaches Drive — acceptable, since no state
//! write occurs. The FINDING (RED) is a SUCCESSFUL broadcast: the backend
//! accepted an intra-bundle double-spend.

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

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
// Below 2× the note value (plus the 2-action fee) so the two duplicated
// spends "cover" it — the point is the duplicate nullifier, not
// insufficient value.
const UNSHIELD_AMOUNT: u64 = 60_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_033_duplicate_nullifier_in_bundle() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_033",
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

    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(!notes.is_empty(), "expected one synced note");
    // The SAME note twice — duplicate nullifier within one bundle.
    let dup = vec![notes[0].clone(), notes[0].clone()];

    let exact_fee = compute_minimum_shielded_fee(2, PlatformVersion::latest())
        .expect("compute_minimum_shielded_fee");
    let dst = s.test_wallet.next_unused_address().await.expect("dst");

    let built = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst,
        UNSHIELD_AMOUNT,
        exact_fee,
        &dup,
    )
    .await;

    match built {
        Ok(st) => {
            let result = broadcast_raw(s.ctx.sdk(), &st).await;
            // Observe the TRUE verdict (consensus, not just check_tx) for Marvin.
            observe_adv_verdict(s.ctx.sdk(), "SH-033", &result, &st, COMMIT_TIMEOUT).await;
            assert!(
                result.is_err(),
                "SH-033 FINDING (CRITICAL): backend ACCEPTED a bundle with a duplicate nullifier \
                 — intra-transaction double-spend. result={result:?}"
            );
            tracing::info!(
                target: "platform_wallet::e2e::cases::sh_033",
                "intra-bundle duplicate nullifier correctly rejected by backend"
            );
        }
        Err(e) => {
            // The build rejected the duplicate before it could reach Drive;
            // no state write occurs. Acceptable (the dup is stopped early),
            // but log it so a reviewer knows the backend arm wasn't exercised.
            // Emit the greppable tag with a build stage so Marvin's one grep
            // still captures SH-033's verdict.
            tracing::info!(
                target: "platform_wallet::e2e::cases::sh_033",
                "ADV-VERDICT probe=SH-033 stage=build result=rejected detail=\"{e}\""
            );
        }
    }

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
