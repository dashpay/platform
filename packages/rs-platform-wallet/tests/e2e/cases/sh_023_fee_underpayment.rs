//! SH-023 — ADVERSARIAL: unshield amount below the minimum shielded fee —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-023. Priority: P1. HIGH-if-fails.
//!
//! The DPP unshield builder derives its own fee, so no correctly signed
//! transition can carry an under-floor amount. Instead, capture a valid
//! funded Type-17 unshield and overwrite `unshielding_amount` (the gross
//! leaving the pool, which must cover the fee) with
//! `compute_shielded_unshield_fee(actions) - 1`.
//!
//! Drive runs `validate_minimum_shielded_fee` before Orchard proof
//! verification at both CheckTx and block processing, so the expected
//! rejection is `InsufficientShieldedFeeError` (40904). The broken signature
//! does not decide the verdict: with the fee floor removed, the same probe
//! would be rejected by proof verification (40902) and this case would fail.
//!
//! RED if accepted, or rejected by anything other than the fee floor.

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_shielded_unshield_fee;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, assert_adv_rejected, bind_shielded, broadcast_raw, capture_unshield_st,
    mutate_serialized_bundle, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
    BundleField, BundleMutation,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Same budget as SH-022: the 200M note covers the 20M unshield plus the
// ~169M unshield fee, so the valid transition builds before mutation.
const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_023_fee_underpayment() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_023",
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

    let dst = s.test_wallet.next_unused_address().await.expect("dst");
    let mut st = capture_unshield_st(&s.test_wallet, &handle, 0, &dst, UNSHIELD_AMOUNT)
        .await
        .expect("capture valid unshield ST");
    let StateTransition::Unshield(UnshieldTransition::V0(valid)) = &st else {
        panic!("capture_unshield_st must return a Type-17 unshield")
    };
    let floor = compute_shielded_unshield_fee(valid.actions.len(), s.ctx.sdk().version())
        .expect("compute_shielded_unshield_fee");
    let under_floor = floor - 1;
    assert!(
        valid.unshielding_amount > under_floor,
        "the valid transition must clear the floor before mutation: amount={} floor={floor}",
        valid.unshielding_amount
    );
    mutate_serialized_bundle(
        &mut st,
        BundleField::ValueBalance,
        &BundleMutation::Overwrite(under_floor.to_le_bytes().to_vec()),
    )
    .expect("set under-floor unshielding_amount");

    let result = broadcast_raw(s.ctx.sdk(), &st).await;
    assert_adv_rejected(
        s.ctx.sdk(),
        "SH-023",
        &result,
        &st,
        COMMIT_TIMEOUT,
        &[40904],
    )
    .await;
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_023",
        floor,
        under_floor,
        "under-floor unshield amount correctly rejected by the fee floor"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
