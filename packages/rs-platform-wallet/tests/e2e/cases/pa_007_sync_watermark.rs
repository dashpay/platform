//! PA-007 — Sync watermark idempotency.
//!
//! **RED-by-design (Found-032)**: `sync_balances()` does not advance the watermark
//! or refresh the local balance map when the incremental DAPI query returns 0 new
//! entries (`query_height >= metadata_height`). Addresses chain-confirmed via
//! `wait_for_balance` are populated through a separate DAPI polling path that does
//! not touch the watermark, so all three `sync_watermark()` reads return `None`.
//! See TEST_SPEC.md Found-032.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-007.
//! Priority: P1.
//!
//! Pins three properties of `sync_balances`:
//!   1. Repeated calls all succeed (no spurious "already syncing" /
//!      "session expired" failures).
//!   2. The internal sync watermark
//!      (`PlatformAddressWallet::sync_watermark`) is monotonic
//!      non-decreasing across calls. UI clients pull this for
//!      "last seen block" displays — a regression that rolls it
//!      back would bake stale info into apps.
//!   3. Cached balances are byte-equal across calls. A second sync
//!      that mutates a cache line in place (double-counting) would
//!      surface here as a per-address mismatch.

use std::time::Duration;

use crate::framework::prelude::*;

/// Gross credits the bank submits when funding `addr_1`. Bank uses
/// `[ReduceOutput(0)]`; addr_1 receives `FUNDING_CREDITS − bank_fee`.
const FUNDING_CREDITS: u64 = 50_000_000;

/// Lower bound on what addr_1 must receive before the test
/// proceeds.
const FUNDING_FLOOR: u64 = 30_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_007_sync_watermark_idempotency() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

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
    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    // ---- Three back-to-back sync_balances calls. ----
    // Each call must Ok; watermark must be monotonic; cached
    // balances must not change.
    let pw = s.test_wallet.platform_wallet().platform();

    s.test_wallet.sync_balances().await.expect("sync #1");
    let wm_1 = pw.sync_watermark().await;
    let bal_1 = s.test_wallet.balances().await;

    s.test_wallet.sync_balances().await.expect("sync #2");
    let wm_2 = pw.sync_watermark().await;
    let bal_2 = s.test_wallet.balances().await;

    s.test_wallet.sync_balances().await.expect("sync #3");
    let wm_3 = pw.sync_watermark().await;
    let bal_3 = s.test_wallet.balances().await;

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_007",
        ?wm_1,
        ?wm_2,
        ?wm_3,
        bal_1_count = bal_1.len(),
        bal_2_count = bal_2.len(),
        bal_3_count = bal_3.len(),
        "watermark snapshots"
    );

    // ---- Property 1: each watermark exists. ----
    // After a successful sync_balances against a non-empty chain
    // the wallet must have a watermark; `None` here implies the
    // sync silently failed to advance state.
    assert!(
        wm_1.is_some(),
        "Found-032 (RED-by-design): sync_balances() does not advance the watermark \
         when the incremental DAPI delta returns 0 new entries — watermark stays None \
         even after chain-confirmed funding. See TEST_SPEC.md Found-032."
    );
    assert!(
        wm_2.is_some(),
        "Found-032 (RED-by-design): sync_balances() does not advance the watermark \
         when the incremental DAPI delta returns 0 new entries — watermark stays None \
         even after chain-confirmed funding. See TEST_SPEC.md Found-032."
    );
    assert!(
        wm_3.is_some(),
        "Found-032 (RED-by-design): sync_balances() does not advance the watermark \
         when the incremental DAPI delta returns 0 new entries — watermark stays None \
         even after chain-confirmed funding. See TEST_SPEC.md Found-032."
    );

    // ---- Property 2: watermark is monotonic non-decreasing. ----
    // The chain may have advanced between syncs — we don't enforce
    // equality. We DO enforce strict non-rollback.
    let (w1, w2, w3) = (wm_1.unwrap(), wm_2.unwrap(), wm_3.unwrap());
    assert!(
        w2 >= w1,
        "PA-007: watermark rolled back across sync #1 → #2 ({w1} → {w2})"
    );
    assert!(
        w3 >= w2,
        "PA-007: watermark rolled back across sync #2 → #3 ({w2} → {w3})"
    );

    // ---- Property 3: cached balances are byte-equal across syncs. ----
    // A regression that double-counts on re-sync surfaces here as
    // a per-address mismatch. The address set must also be stable
    // (no spurious additions / removals from re-syncing the same
    // chain state).
    assert_eq!(
        bal_1, bal_2,
        "PA-007: cached balances diverged between sync #1 and #2 \
         (double-counting / spurious mutation regression?)"
    );
    assert_eq!(
        bal_2, bal_3,
        "PA-007: cached balances diverged between sync #2 and #3 \
         (double-counting / spurious mutation regression?)"
    );

    // Sanity: addr_1 must still hold its funded credits (a regression
    // that resets balances to zero on re-sync would surface here).
    let addr_1_bal = bal_3.get(&addr_1).copied().unwrap_or(0);
    assert!(
        addr_1_bal >= FUNDING_FLOOR,
        "PA-007: addr_1 balance dropped below FUNDING_FLOOR after \
         re-syncs ({addr_1_bal} < {FUNDING_FLOOR})"
    );

    s.teardown().await.expect("teardown");
}
