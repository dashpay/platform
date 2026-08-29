//! SH-023 — ADVERSARIAL: fee underpayment below `compute_minimum_shielded_fee`
//! — backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-023. Priority: P1. HIGH-if-fails.
//!
//! Attack: build a spend declaring a fee BELOW the minimum. This case
//! exercises the CLIENT floor (the `build_*_st` path delegates to the dpp
//! `build_unshield_transition`, which rejects `Some(f) if f < min_fee`
//! internally at `unshield.rs:60-65`), proving the wallet refuses to emit
//! an under-floor transition.
//!
//! # RESIDUAL PRODUCTION GAP (flagged, not fixed)
//!
//! The independent BACKEND-floor arm (confirm Drive ALSO rejects an
//! under-floor fee submitted by a client WITHOUT the guard) is not
//! reachable: the fee is folded into the spend's value math during build,
//! there is no post-build `fee` field on the `SerializedBundle` to mutate,
//! and the only assembly path (the dpp builder) enforces the floor. A
//! deeper raw-bundle seam (assemble from arbitrary value_balance + actions
//! bypassing the builder's fee math) would be required to drive the
//! backend-floor arm. Documented; the client-floor arm is asserted live.

#![cfg(feature = "shielded")]

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, build_unshield_st_against_notes, shielded_prover,
    teardown_sweep_shielded, unspent_notes, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee` (see `sh_010`),
// with 1e8 headroom.
const FUNDING_CREDITS: u64 = 1_312_851_200;
const SHIELD_AMOUNT: u64 = 50_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

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

    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    let one_note = vec![notes[0].clone()];
    let dst = s.test_wallet.next_unused_address().await.expect("dst");

    // Declare a zero fee (well under the floor). The dpp builder must
    // refuse to emit the transition.
    let built = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst,
        UNSHIELD_AMOUNT,
        0,
        &one_note,
    )
    .await;
    assert!(
        built.is_err(),
        "SH-023: building an under-floor-fee unshield must be rejected (client fee floor); \
         observed Ok — the wallet emitted an under-floor transition"
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_023",
        "under-floor fee correctly rejected at build (client floor); backend-floor arm is a \
         documented residual gap (no post-build fee seam)"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
