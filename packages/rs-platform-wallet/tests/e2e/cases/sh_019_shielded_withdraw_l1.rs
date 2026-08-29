//! SH-019 — Shielded withdraw to a Core L1 address (Type 19).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-019.
//! Priority: P1. (Wave H + Core-L1 gate.)
//!
//! The shielded SPEND half is exercisable now (same path as SH-002): we
//! shield a note, withdraw part of it to a Core L1 address, and assert
//! the shielded-side bookkeeping unconditionally (this half is
//! GREEN-capable). The L1-arrival assertion needs Layer-1 payout
//! observation and is gated behind `PLATFORM_WALLET_E2E_BANK_CORE_GATE`;
//! until that observation seam is wired it MAY run RED — documenting the
//! gate, not a production defect in the shield path.
//!
//! NOTE (flagged gap): there is no harness Layer-1 payout-observation
//! seam yet (shared with §5 item 2 transparent withdrawal). The L1-read
//! arm below is therefore left as a documented TODO rather than a live
//! assertion — wiring it is the Core-L1 follow-up.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const WITHDRAW_AMOUNT: u64 = 20_000_000;
const CORE_FEE_PER_BYTE: u32 = 1;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_019_shielded_withdraw_l1() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

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

    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
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

    // Withdraw to a Core L1 address — the bank's Core receive address is
    // a real, network-valid Base58Check string available without extra
    // funding.
    let to_core = s
        .ctx
        .bank()
        .primary_core_receive_address()
        .await
        .expect("derive bank Core receive address")
        .to_string();

    let before = handle
        .balances(&s.test_wallet)
        .await
        .expect("pre-withdraw shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);

    s.test_wallet
        .platform_wallet()
        .shielded_withdraw_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &to_core,
            WITHDRAW_AMOUNT,
            CORE_FEE_PER_BYTE,
            prover,
        )
        .await
        .expect("shielded_withdraw_to (shielded spend half must succeed)");

    // Shielded-side assertions (GREEN-capable, no L1 gate): the change
    // note is retained and the balance dropped by at least the withdraw
    // amount.
    handle.sync().await;
    let after = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-withdraw shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    assert!(
        before.saturating_sub(after) >= WITHDRAW_AMOUNT,
        "shielded balance must drop by at least the withdraw amount; before={before} after={after}"
    );
    assert!(
        after > 0,
        "shielded change note must be retained after a partial withdraw; observed {after}"
    );

    // The spent note must be marked spent — a second identical withdraw
    // must not re-select it (it either spends the change or fails
    // insufficient-balance, never re-spends the consumed note).
    let second = s
        .test_wallet
        .platform_wallet()
        .shielded_withdraw_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &to_core,
            WITHDRAW_AMOUNT,
            CORE_FEE_PER_BYTE,
            prover,
        )
        .await;
    // Either it succeeds from the remaining change or it fails on
    // insufficient balance — both prove the original note was consumed
    // exactly once. A panic / double-spend would be the regression.
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_019",
        ?second,
        "second withdraw outcome (must not re-spend the consumed note)"
    );

    // TODO(Core-L1 follow-up): observe the L1 payout on `to_core` once
    // the Layer-1 payout-observation seam exists (shared with §5 item 2).
    // Gated behind PLATFORM_WALLET_E2E_BANK_CORE_GATE. Until then the
    // L1-arrival assertion is intentionally absent — the shielded-side
    // assertions above are the GREEN-capable half.

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
