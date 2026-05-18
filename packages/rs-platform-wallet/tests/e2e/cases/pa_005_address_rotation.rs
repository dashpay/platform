//! PA-005 — Address rotation: gap-limit + observed-used cursor.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-005.
//! Priority: P1.
//!
//! Pins two invariants of `next_unused_receive_address`:
//!   1. **Reserve on hand-out.** Each call reserves its index on
//!      hand-out and returns a distinct address; the cursor advances
//!      on hand-out, not on observed-used. Three back-to-back calls
//!      (no sync between) return three pairwise-distinct addresses
//!      (Found-026 `bc87e4dec9`).
//!   2. **Cursor advances after funding + sync.** Once `addr_n` is
//!      observed-used, the next call returns a fresh distinct address.
//!
//! The spec asks for 16 funding rounds to validate sustained rotation
//! through the full DIP-17 gap window (`DIP17_GAP_LIMIT = 20`). We
//! trim to four sequential rounds in this test (chain RTT × 16 ≈ 8
//! min runtime is too long for the P1 tier) — the *cursor advance*
//! invariant is observable after just the second round; rounds 3-4
//! are the regression bound that catches an off-by-one in the cursor
//! step. The "21+ unused addresses" gap-window boundary is split into
//! its own case PA-005b.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;

/// Per-fund credit amount. Bank uses `[ReduceOutput(0)]`, so the
/// recipient receives `FUND_AMOUNT − bank_fee`. Sized above the
/// empirical 1in/1out chain-time fee (~15M) so a non-zero residual
/// triggers the cursor's observed-used advance.
const FUND_AMOUNT: u64 = 30_000_000;

/// Lower bound on what the recipient must hold before the cursor
/// will advance.
const FUND_FLOOR: u64 = 1_000_000;

/// Number of funding rounds. Each round funds the previously
/// returned address and asserts the next derivation is distinct.
const ROUNDS: usize = 4;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_005_address_rotation() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Invariant 1: each hand-out reserves its index (distinct). ----
    let a1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive a1");
    let a2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive a2 (reserved on hand-out)");
    let a3 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive a3 (reserved on hand-out)");
    assert_ne!(
        a1, a2,
        "Invariant 1: each hand-out reserves its index; back-to-back \
         calls must return DISTINCT addresses (Found-026)"
    );
    assert_ne!(
        a1, a3,
        "Invariant 1: each hand-out reserves its index; back-to-back \
         calls must return DISTINCT addresses (Found-026)"
    );
    assert_ne!(
        a2, a3,
        "Invariant 1: each hand-out reserves its index; back-to-back \
         calls must return DISTINCT addresses (Found-026)"
    );

    // ---- Invariant 2: cursor advances after funding + sync. ----
    // Track every address we observe so we can assert distinctness
    // across the full sequence at the end (catches a hypothetical
    // bug where the cursor skips forward then back).
    let mut observed = Vec::with_capacity(ROUNDS + 1);
    observed.push(a1);

    let mut current = a1;
    for round in 0..ROUNDS {
        s.ctx
            .bank()
            .fund_address(&current, FUND_AMOUNT)
            .await
            .unwrap_or_else(|err| panic!("round {round} fund: {err:?}"));
        wait_for_balance(&s.test_wallet, &current, FUND_FLOOR, STEP_TIMEOUT)
            .await
            .unwrap_or_else(|err| panic!("round {round} balance: {err:?}"));

        let next = s
            .test_wallet
            .next_unused_address()
            .await
            .unwrap_or_else(|err| panic!("round {round} derive next: {err:?}"));
        assert_ne!(
            next, current,
            "round {round}: cursor must advance after observed-used; \
             got the same address {current:?} after funding"
        );
        observed.push(next);
        current = next;
    }

    // Pairwise distinctness across the full set — catches a cursor
    // that wraps or revisits prior indices.
    for i in 0..observed.len() {
        for j in (i + 1)..observed.len() {
            assert_ne!(
                observed[i], observed[j],
                "PA-005: observed addresses #{i} and #{j} collided ({:?})",
                observed[i]
            );
        }
    }

    // Final balance audit — every funded address should hold its
    // post-fee credits. Catches a regression where the cursor advances
    // but the funding is silently routed to the wrong address.
    s.test_wallet.sync_balances().await.expect("final sync");
    let balances: BTreeMap<_, _> = s.test_wallet.balances().await;
    for (i, addr) in observed.iter().take(ROUNDS).enumerate() {
        let bal = balances.get(addr).copied().unwrap_or(0);
        assert!(
            bal >= FUND_FLOOR,
            "PA-005: funded address #{i} ({addr:?}) holds {bal} credits, \
             expected ≥ FUND_FLOOR ({FUND_FLOOR}) — funding was misrouted"
        );
    }

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_005",
        rounds = ROUNDS,
        distinct_addresses = observed.len(),
        "address rotation validated"
    );

    s.teardown().await.expect("teardown");
}
