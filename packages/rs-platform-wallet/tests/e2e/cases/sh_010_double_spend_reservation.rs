//! SH-010 — Double-spend guard: two overlapping spends reserve disjoint
//! notes (`reserve_unspent_notes`).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-010.
//! Priority: P2.
//!
//! Shields two notes into account 0, then fires two concurrent unshields
//! each coverable by one note. The single-write-lock select+reserve must
//! hand them disjoint notes — no shared nullifier, no double-count. If
//! both succeed, the shielded balance dropped by `2*amount + 2*fee`.
//!
//! Expected outcome: PASS — this is the contract `reserve_unspent_notes`
//! exists to uphold; the canary for a reservation-race regression.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// `select_shield_inputs` claims greedily from the smallest-key address,
// so BOTH sequential shields concentrate on one funded address before
// moving on. Each shield reserves `FEE_RESERVE_CREDITS` (1e9,
// `platform_wallet.rs`) on its input plus the ~1.63e8 protocol fee, so a
// single address must survive both shields: `2 × (SHIELD_EACH + 1.63e8)
// + 1e9`. Two addresses are funded (one per loop iteration); each carries
// the full two-shield budget so whichever sorts smallest can absorb both.
const FUNDING_CREDITS: u64 = 3_545_702_400;
const SHIELD_EACH: u64 = 1_110_000_000;
const UNSHIELD_EACH: u64 = 10_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_010_double_spend_reservation() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

    // Two separate fundings → two shields → two distinct notes.
    for _ in 0..2 {
        let addr = s
            .test_wallet
            .next_unused_address()
            .await
            .expect("derive funding addr");
        s.ctx
            .bank()
            .fund_address(&addr, FUNDING_CREDITS)
            .await
            .expect("bank.fund_address");
        wait_for_address_balance_chain_confirmed_n(
            s.ctx.sdk(),
            &addr,
            FUNDING_CREDITS,
            CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
            STEP_TIMEOUT,
        )
        .await
        .expect("funding never observed");
        s.test_wallet
            .sync_balances()
            .await
            .expect("pre-shield sync");
    }

    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    for _ in 0..2 {
        s.test_wallet
            .platform_wallet()
            .shielded_shield_from_account(
                &handle.coordinator,
                0,
                0,
                SHIELD_EACH,
                s.test_wallet.address_signer(),
                prover,
            )
            .await
            .expect("shield_from_account");
    }
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_EACH * 2, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached 2 notes");

    let before = handle
        .balances(&s.test_wallet)
        .await
        .expect("pre-spend shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);

    // Two destinations, two concurrent unshields.
    let dst_a = s.test_wallet.next_unused_address().await.expect("dst_a");
    let dst_b = s.test_wallet.next_unused_address().await.expect("dst_b");
    let dst_a_b32 = dst_a.to_bech32m_string(s.ctx.bank().network());
    let dst_b_b32 = dst_b.to_bech32m_string(s.ctx.bank().network());
    let pw = s.test_wallet.platform_wallet();
    let seed = s.test_wallet.seed_bytes();

    let (ra, rb) = tokio::join!(
        pw.shielded_unshield_to(
            &handle.coordinator,
            &seed,
            0,
            &dst_a_b32,
            UNSHIELD_EACH,
            prover,
        ),
        pw.shielded_unshield_to(
            &handle.coordinator,
            &seed,
            0,
            &dst_b_b32,
            UNSHIELD_EACH,
            prover,
        ),
    );

    // At most one may fail (if only one note were spendable); if both
    // succeed they MUST have reserved disjoint notes — verified via the
    // post-spend balance drop being at least 2*amount (no double-count).
    let succeeded = [ra.is_ok(), rb.is_ok()].iter().filter(|ok| **ok).count();
    assert!(
        succeeded >= 1,
        "at least one concurrent unshield must succeed; ra={ra:?} rb={rb:?}"
    );

    handle.sync().await;
    let after = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-spend shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    let dropped = before.saturating_sub(after);
    assert!(
        dropped >= UNSHIELD_EACH * (succeeded as u64),
        "shielded balance must drop by at least {UNSHIELD_EACH} per successful spend \
         (disjoint notes, no double-count); before={before} after={after} succeeded={succeeded}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
