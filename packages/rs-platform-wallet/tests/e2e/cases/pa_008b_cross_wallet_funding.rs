//! PA-008b — Two `TestWallet`s × three concurrent funders each.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-008b.
//! Priority: P2.
//!
//! PA-008 keeps contention inside one `TestWallet`. PA-008b proves the
//! bank's `FUNDING_MUTEX` serialisation works under cross-wallet
//! contention too — six concurrent fund calls (three for wallet A,
//! three for wallet B) must all land without nonce collisions or
//! lost funding.
//!
//! This is the realistic CI shape: two test bodies sharing one
//! process, both calling `bank.fund_address` simultaneously. A
//! regression that bypasses the mutex on a per-wallet basis would
//! corrupt the bank's outgoing nonce sequence.
//!
//! Setup tradeoff: deriving 6 distinct unused addresses (3 on A, 3 on
//! B) requires marking each pool's cursor as observed-used before
//! deriving the next slot. We do that with a small marker fund per
//! address — `MARKER_AMOUNT` is sized above the empirical 1in/1out
//! chain-time fee (~15M).

use std::time::Duration;

use crate::framework::prelude::*;

/// Marker fund used to advance each wallet's receive-pool cursor.
const MARKER_AMOUNT: u64 = 30_000_000;
const MARKER_FLOOR: u64 = 1_000_000;

/// Concurrent fund amount per address.
const FUND_AMOUNT: u64 = 30_000_000;
const FUND_FLOOR: u64 = 1_000_000;

const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared)]
async fn pa_008b_two_wallets_six_concurrent_funders() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s_a = setup().await.expect("e2e setup A failed");
    let s_b = setup().await.expect("e2e setup B failed");

    // Helper: derive 3 distinct addresses on a wallet by alternating
    // marker funds + cursor advances.
    async fn derive_three_distinct(s: &SetupGuard) -> [dpp::address_funds::PlatformAddress; 3] {
        let bank = s.ctx.bank();
        let a = s.test_wallet.next_unused_address().await.expect("derive a");
        bank.fund_address(&a, MARKER_AMOUNT)
            .await
            .expect("marker a");
        wait_for_balance(&s.test_wallet, &a, MARKER_FLOOR, STEP_TIMEOUT)
            .await
            .expect("marker a never observed");

        let b = s.test_wallet.next_unused_address().await.expect("derive b");
        assert_ne!(a, b);
        bank.fund_address(&b, MARKER_AMOUNT)
            .await
            .expect("marker b");
        wait_for_balance(&s.test_wallet, &b, MARKER_FLOOR, STEP_TIMEOUT)
            .await
            .expect("marker b never observed");

        let c = s.test_wallet.next_unused_address().await.expect("derive c");
        assert_ne!(a, c);
        assert_ne!(b, c);
        [a, b, c]
    }

    let [a1, a2, a3] = derive_three_distinct(&s_a).await;
    let [b1, b2, b3] = derive_three_distinct(&s_b).await;

    // ---- Six concurrent funds. ----
    // Both wallets share the same bank (singleton via `s_*.ctx.bank()`).
    let bank = s_a.ctx.bank();
    let (r1, r2, r3, r4, r5, r6) = tokio::join!(
        bank.fund_address(&a1, FUND_AMOUNT),
        bank.fund_address(&a2, FUND_AMOUNT),
        bank.fund_address(&a3, FUND_AMOUNT),
        bank.fund_address(&b1, FUND_AMOUNT),
        bank.fund_address(&b2, FUND_AMOUNT),
        bank.fund_address(&b3, FUND_AMOUNT),
    );
    r1.expect("concurrent fund a1");
    r2.expect("concurrent fund a2");
    r3.expect("concurrent fund a3");
    r4.expect("concurrent fund b1");
    r5.expect("concurrent fund b2");
    r6.expect("concurrent fund b3");

    // ---- Wait for each wallet to observe its three concurrent funds. ----
    for (s, addrs) in [(&s_a, [a1, a2, a3]), (&s_b, [b1, b2, b3])] {
        for addr in addrs {
            wait_for_balance(&s.test_wallet, &addr, FUND_FLOOR, STEP_TIMEOUT)
                .await
                .unwrap_or_else(|err| {
                    panic!(
                        "PA-008b: address {:?} never observed concurrent fund: {err:?}",
                        addr
                    )
                });
        }
    }

    // ---- Final balance audit on each address: ≥ FUND_FLOOR. ----
    s_a.test_wallet.sync_balances().await.expect("final sync A");
    s_b.test_wallet.sync_balances().await.expect("final sync B");
    let bal_a = s_a.test_wallet.balances().await;
    let bal_b = s_b.test_wallet.balances().await;

    for (label, addr, bal) in [
        ("a1", &a1, bal_a.get(&a1).copied().unwrap_or(0)),
        ("a2", &a2, bal_a.get(&a2).copied().unwrap_or(0)),
        ("a3", &a3, bal_a.get(&a3).copied().unwrap_or(0)),
        ("b1", &b1, bal_b.get(&b1).copied().unwrap_or(0)),
        ("b2", &b2, bal_b.get(&b2).copied().unwrap_or(0)),
        ("b3", &b3, bal_b.get(&b3).copied().unwrap_or(0)),
    ] {
        tracing::info!(
            target: "platform_wallet::e2e::cases::pa_008b",
            label,
            ?addr,
            bal,
            "post-concurrent balance"
        );
        assert!(
            bal >= FUND_FLOOR,
            "PA-008b: address {label} ({addr:?}) held {bal} credits, \
             expected ≥ FUND_FLOOR ({FUND_FLOOR}) — concurrent fund \
             likely lost on a cross-wallet nonce race"
        );
    }

    s_b.teardown().await.expect("teardown B");
    s_a.teardown().await.expect("teardown A");
}
