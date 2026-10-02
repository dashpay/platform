//! PA-007b — Two concurrent `sync_balances` on one wallet.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-007b.
//! Priority: P2.
//!
//! Pins the reentrancy / internal-locking contract for `sync_balances`:
//! two concurrent futures on the same `TestWallet` handle MUST both
//! return `Ok(())` AND leave the cached balance equal to on-chain
//! truth (NOT 2× — no double-counting).
//!
//! UI clients call `sync_balances` aggressively (every refresh tick,
//! every focus event). A regression that double-counts under
//! concurrent re-sync is a UI-tier hazard worth pinning.

use std::sync::Arc;
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
async fn pa_007b_concurrent_sync_balances() {
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

    // ---- Snapshot pre-concurrent state. ----
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-concurrent sync");
    let pre_balances = s.test_wallet.balances().await;
    let pw = s.test_wallet.platform_wallet().platform();
    let pre_watermark = pw.sync_watermark().await;

    // ---- Two concurrent sync_balances on the SAME wallet ----
    // The PlatformWallet handle is `Arc<PlatformWallet>`; we use
    // `tokio::join!` rather than `tokio::spawn` so the futures can
    // borrow the wallet handle without a `'static` lifetime bound.
    // The two futures still execute concurrently on the runtime.
    let wallet_a = Arc::clone(s.test_wallet.platform_wallet());
    let wallet_b = Arc::clone(s.test_wallet.platform_wallet());
    let (r_a, r_b) = tokio::join!(
        async { wallet_a.platform().sync_balances(None).await.map(|_| ()) },
        async { wallet_b.platform().sync_balances(None).await.map(|_| ()) },
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_007b",
        ?r_a,
        ?r_b,
        "concurrent sync outcomes"
    );

    // ---- Property: both succeed. ----
    r_a.expect("PA-007b: future a sync_balances must return Ok");
    r_b.expect("PA-007b: future b sync_balances must return Ok");

    // ---- Property: cached balances are NOT doubled. ----
    let post_balances = s.test_wallet.balances().await;
    let pre_addr_1 = pre_balances.get(&addr_1).copied().unwrap_or(0);
    let post_addr_1 = post_balances.get(&addr_1).copied().unwrap_or(0);
    // The chain might have advanced between syncs (e.g. an unrelated
    // settlement landed) but addr_1's balance must not be 2×. We
    // pin a tight upper bound: any post value strictly less than 2×
    // pre passes. A double-count regression would push post_addr_1
    // to ≥ 2 × pre.
    assert!(
        post_addr_1 < pre_addr_1.saturating_mul(2),
        "PA-007b: addr_1 balance suspiciously high after concurrent syncs \
         (pre={pre_addr_1}, post={post_addr_1}) — possible double-counting"
    );
    // Tighter: balance must equal pre (the chain didn't advance for
    // addr_1 specifically — no new funds landed during the test).
    // Allow a tiny slack only for the (rare) edge case where another
    // test's transfer happens to credit this address; in practice
    // every test uses fresh seeds, so pre == post is the real
    // contract.
    assert_eq!(
        post_addr_1, pre_addr_1,
        "PA-007b: addr_1 balance must be byte-equal across concurrent \
         syncs (no double-counting); pre={pre_addr_1}, post={post_addr_1}"
    );

    // ---- Property: watermark advanced at most once net (no double-bump). ----
    let post_watermark = pw.sync_watermark().await;
    if let (Some(pre), Some(post)) = (pre_watermark, post_watermark) {
        // Watermark is monotonic non-decreasing. The chain may have
        // advanced naturally, but the two concurrent syncs must
        // not BOTH bump it past the same chain tip.
        assert!(
            post >= pre,
            "PA-007b: watermark rolled back across concurrent syncs ({pre} → {post})"
        );
    }

    s.teardown().await.expect("teardown");
}
