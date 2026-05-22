//! SH-026 — ADVERSARIAL: wrong/random anchor — backend MUST reject
//! AnchorMismatch [INJECT] (Found-030 dynamic probe).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-026. Priority: P1. HIGH-if-fails.
//!
//! Attack: capture a VALID Type-17 unshield, overwrite
//! `SerializedBundle.anchor` with random 32 bytes (a root Drive never
//! recorded) while the witness paths authenticate against the real root,
//! then broadcast raw. Drive accepts only anchors it has recorded, so a
//! wrong anchor must fail.
//!
//! Found-030 dynamic probe: whichever anchor the backend accepts resolves
//! the doc ambiguity between `operations.rs:601-611` ("most recent
//! checkpoint") and `file_store.rs:162-165` ("current tree state"). A
//! wrong-anchor acceptance is a soundness break (RED).

#![cfg(feature = "shielded")]

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, broadcast_raw, capture_unshield_st,
    mutate_serialized_bundle, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
    BundleField, BundleMutation,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 90_000_000;
const SHIELD_AMOUNT: u64 = 50_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_026_anchor_mismatch() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_026",
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

    let dst = s.test_wallet.next_unused_address().await.expect("dst");

    // Overwrite the anchor with a root the chain never recorded.
    let mut st = capture_unshield_st(&s.test_wallet, &handle, 0, &dst, UNSHIELD_AMOUNT)
        .await
        .expect("capture valid unshield ST");
    mutate_serialized_bundle(
        &mut st,
        BundleField::Anchor,
        &BundleMutation::Overwrite(vec![0xAB; 32]),
    )
    .expect("tamper anchor");
    let result = broadcast_raw(s.ctx.sdk(), &st).await;
    assert!(
        result.is_err(),
        "SH-026 FINDING: backend ACCEPTED a wrong/random anchor — soundness break (and resolves \
         Found-030 against any documented depth). result={result:?}"
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_026",
        "wrong anchor correctly rejected by backend (Found-030 probe: rejected as expected)"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
