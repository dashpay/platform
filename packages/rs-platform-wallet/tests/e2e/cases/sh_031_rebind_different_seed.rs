//! SH-031 — ADVERSARIAL: double-bind / rebind with a DIFFERENT seed — no
//! key-material mix, no leak.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-031. Priority: P1. HIGH-if-fails.
//!
//! Attack: `bind_shielded(seed_A, &[0])`, shield + sync some notes, then
//! `bind_shielded(seed_B, &[0])` with a DIFFERENT seed on the same
//! wallet/coordinator. The rebind path unregisters+reregisters and the
//! doc claims "replace-not-merge".
//!
//! Correct behavior: after rebind to seed_B, seed_A's notes are NOT
//! visible/spendable under seed_B's keys (different IVK ⇒ no decryption).
//! RED if seed-A notes leak into seed-B's balance (privacy/accounting
//! break) or stale pending reservations make seed-B skip spendable notes.
//!
//! Achievable through the public API (`bind_shielded` twice) — no
//! production-seam change needed.

#![cfg(feature = "shielded")]

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee` (see `sh_010`),
// with 1e8 headroom.
const FUNDING_CREDITS: u64 = 1_312_851_200;
const SHIELD_AMOUNT: u64 = 50_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_031_rebind_different_seed() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_031",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let pw = s.test_wallet.platform_wallet();

    // Bind with seed_A (the wallet's real seed) and shield a note.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind seed_A");
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
    pw.shielded_shield_from_account(
        &handle.coordinator,
        0,
        0,
        SHIELD_AMOUNT,
        s.test_wallet.address_signer(),
        prover,
    )
    .await
    .expect("shield under seed_A");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("seed_A note never synced");

    // Rebind the SAME wallet/coordinator with a DIFFERENT seed.
    let (seed_b, _hex) = crate::framework::wallet_factory::fresh_seed();
    pw.bind_shielded(&seed_b, &[0], &handle.coordinator)
        .await
        .expect("rebind seed_B");

    // Under seed_B's IVK, seed_A's note must NOT be visible. Re-scan and
    // assert account 0 reports 0 (no cross-seed decryption / leak).
    handle.sync().await;
    let under_b = handle
        .balances(&s.test_wallet)
        .await
        .expect("balances under seed_B")
        .get(&0)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        under_b, 0,
        "SH-031 FINDING: seed_A's note ({SHIELD_AMOUNT}) leaked into seed_B's balance \
         after rebind — key-material mix / privacy break. observed {under_b}"
    );

    // Rebind back to seed_A and confirm its note re-discovers cleanly
    // (the rebind purge did not corrupt or strand it).
    pw.bind_shielded(&s.test_wallet.seed_bytes(), &[0], &handle.coordinator)
        .await
        .expect("rebind back to seed_A");
    let restored =
        wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
            .await
            .expect("seed_A note not re-discovered after rebind-back (stale-state corruption)");
    assert_eq!(
        restored, SHIELD_AMOUNT,
        "rebind back to seed_A must re-discover its note exactly; observed {restored}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
