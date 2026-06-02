//! SH-024 — ADVERSARIAL: u64 value-boundary overflow — backend MUST
//! reject safely [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-024. Priority: P1. HIGH-if-fails.
//!
//! Attack: capture a VALID Type-17 unshield, overwrite `unshielding_amount`
//! to `u64::MAX` (and `u64::MAX - 1`), and broadcast raw. The arithmetic
//! must be checked on the BACKEND — no wraparound, no validator panic, no
//! boundary value silently accepted. The client `checked_add` guard alone
//! is not the line of defense; a direct gRPC submitter bypasses it.
//!
//! RED if the backend wraps, panics, or accepts a boundary value.

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

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_024_value_boundary_overflow() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_024",
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

    for boundary in [u64::MAX, u64::MAX - 1] {
        let mut st = capture_unshield_st(&s.test_wallet, &handle, 0, &dst, UNSHIELD_AMOUNT)
            .await
            .expect("capture valid unshield ST");
        mutate_serialized_bundle(
            &mut st,
            BundleField::ValueBalance,
            &BundleMutation::Overwrite(boundary.to_le_bytes().to_vec()),
        )
        .expect("set boundary amount");
        let result = broadcast_raw(s.ctx.sdk(), &st).await;
        assert!(
            result.is_err(),
            "SH-024 FINDING: backend ACCEPTED a boundary unshielding_amount ({boundary}) — \
             missing backend arithmetic check (wrap/overflow/accept). result={result:?}"
        );
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_024",
            boundary,
            "boundary amount correctly rejected by backend"
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
