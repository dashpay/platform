//! SH-034 — ADVERSARIAL: tampered binding signature — backend MUST
//! reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-034. Priority: P1.
//! CRITICAL-if-it-fails (value-balance binding bypass).
//!
//! Attack: capture a VALID Type-17 unshield, flip bytes in
//! `SerializedBundle.binding_signature` (64 bytes), broadcast raw. The
//! binding signature commits to the value balance; a tampered signature
//! must fail Orchard bundle verification at the backend.
//!
//! RED if the backend accepts a tampered binding signature.

#![cfg(feature = "shielded")]

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, assert_adv_rejected, bind_shielded, broadcast_raw, capture_unshield_st,
    mutate_serialized_bundle, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
    BundleField, BundleMutation,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Two binding-signature mutations each consume one note via
// `capture_unshield_st` (which reserves and never releases), so fund +
// shield one note PER probe: `2 × (SHIELD_AMOUNT + 1.63e8) + 1e9`
// (see `sh_024`).
const FUNDING_CREDITS: u64 = 1_725_702_400;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const NUM_PROBES: u64 = 2;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_034_tampered_binding_signature() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_034",
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
    // One note per probe: `capture_unshield_st` reserves a note and never
    // releases it, so a single note would starve the second probe.
    for _ in 0..NUM_PROBES {
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
    }
    wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        0,
        SHIELD_AMOUNT * NUM_PROBES,
        STEP_TIMEOUT,
    )
    .await
    .expect("shielded balance never reached the probe-note total");

    let dst = s.test_wallet.next_unused_address().await.expect("dst");

    for mutation in [BundleMutation::FlipByte(0), BundleMutation::Zero] {
        let mut st = capture_unshield_st(&s.test_wallet, &handle, 0, &dst, UNSHIELD_AMOUNT)
            .await
            .expect("capture valid unshield ST");
        mutate_serialized_bundle(&mut st, BundleField::BindingSignature, &mutation)
            .expect("tamper binding signature");
        let result = broadcast_raw(s.ctx.sdk(), &st).await;
        // Verdict is load-bearing: the TRUE consensus result (not just check_tx
        // admission) gates PASS/FAIL, and the bundle-verification reason pins the
        // rejection so a transport drop can't read as "binding enforced". FAILS
        // only if the tampered binding signature actually committed at consensus.
        let probe = format!("SH-034/{mutation:?}");
        assert_adv_rejected(
            s.ctx.sdk(),
            &probe,
            &result,
            &st,
            COMMIT_TIMEOUT,
            &[
                "binding",
                "signature",
                "proof",
                "bundle",
                "verification",
                "invalid",
            ],
        )
        .await;
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_034",
            ?mutation,
            "tampered binding signature correctly rejected by backend"
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
