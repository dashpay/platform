//! SH-022 — ADVERSARIAL: value not conserved (outputs > inputs) —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-022. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails (value forgery / unlimited
//! shielded-pool inflation).
//!
//! Attack: capture a VALID Type-17 unshield (spending the funded note,
//! unshielding `UNSHIELD_AMOUNT`), then overwrite `unshielding_amount` to
//! exceed the spent note value — minting value from nothing — and
//! broadcast raw.
//! Orchard's value-balance check + Drive's credit accounting must refuse
//! a bundle where shielded inputs < outputs + fee. The Halo-2 proof binds
//! `value_balance`, so the mismatch must fail proof verification or the
//! consensus value check (`ShieldedInvalidValueBalanceError`, code 10822).
//!
//! RED if accepted — value forgery.

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

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
/// Far above the spent note's value (`SHIELD_AMOUNT`) — mints value from
/// nothing.
const FORGED_AMOUNT: u64 = 1_000_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_022_value_not_conserved() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_022",
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

    // Capture a valid 20M unshield, then forge the declared amount to 1B.
    let mut st = capture_unshield_st(&s.test_wallet, &handle, 0, &dst, UNSHIELD_AMOUNT)
        .await
        .expect("capture valid unshield ST");
    mutate_serialized_bundle(
        &mut st,
        BundleField::ValueBalance,
        &BundleMutation::Overwrite(FORGED_AMOUNT.to_le_bytes().to_vec()),
    )
    .expect("forge unshielding_amount");
    let result = broadcast_raw(s.ctx.sdk(), &st).await;
    // Verdict is load-bearing: the TRUE consensus result (not just check_tx
    // admission) gates PASS/FAIL, and the value-balance / proof reason pins the
    // rejection so a transport drop can't read as "value conserved". FAILS only
    // if the forged outputs-greater-than-inputs transition committed at consensus.
    assert_adv_rejected(
        s.ctx.sdk(),
        "SH-022",
        &result,
        &st,
        COMMIT_TIMEOUT,
        &[
            "value",
            "balance",
            "proof",
            "bundle",
            "verification",
            "invalid",
            "conserv",
        ],
    )
    .await;
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_022",
        "value-not-conserved transition correctly rejected by backend"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
