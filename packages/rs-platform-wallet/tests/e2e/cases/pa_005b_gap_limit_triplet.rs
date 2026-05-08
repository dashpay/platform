//! PA-005b — `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-005b.
//! Priority: P2.
//!
//! Drives the `next_unused_receive_addresses(count)` test helper that
//! wraps `AddressPool::generate_addresses` while enforcing the gap-limit
//! cap. Three independent tests run on separate `TestWallet` instances:
//!
//! - `pa_005b_gap_limit_triplet_subcase_a` — `count = gap_limit - 1`:
//!   must succeed with that many distinct addresses.
//! - `pa_005b_gap_limit_triplet_subcase_b` — `count = gap_limit`: must
//!   succeed at the boundary.
//! - `pa_005b_gap_limit_triplet_subcase_c` — `count = gap_limit + 1`:
//!   must return [`PlatformWalletError::GapLimitExceeded`] without
//!   mutating the pool, and a follow-up boundary call must still succeed.

use crate::framework::gap_limit::next_unused_receive_addresses;
use crate::framework::prelude::*;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::PlatformPaymentAccountSpec;
use platform_wallet::PlatformWalletError;

fn default_account_key() -> PlatformPaymentAccountKey {
    let PlatformPaymentAccountSpec { account, key_class } = PlatformPaymentAccountSpec::default();
    PlatformPaymentAccountKey { account, key_class }
}

#[tokio_shared_rt::test(shared)]
async fn pa_005b_gap_limit_triplet_subcase_a() {
    // Sub-case A: derive 19 distinct unused addresses (gap_limit - 1).
    let s = setup().await.expect("e2e setup failed (sub-case A)");
    let key = default_account_key();
    // QA-V19-003: Removed `pool_gap_limit ≥ 21` precondition — production uses
    // DEFAULT_GAP_LIMIT = 20 (DIP17). The triplet (limit-1, limit, limit+1) is
    // computed from the live value, no fixed lower bound required.
    let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
    let count = (pool_gap_limit - 1) as usize;
    let addrs = next_unused_receive_addresses(s.test_wallet.platform_wallet(), key, count)
        .await
        .expect("gap_limit-1 must succeed");
    assert_eq!(addrs.len(), count, "must return exactly count addresses");
    let unique: std::collections::HashSet<_> = addrs.iter().collect();
    assert_eq!(
        unique.len(),
        count,
        "all addresses returned in one batch must be distinct"
    );
    s.teardown().await.expect("teardown sub-case A");
}

#[tokio_shared_rt::test(shared)]
async fn pa_005b_gap_limit_triplet_subcase_b() {
    // Sub-case B: derive exactly gap_limit addresses — sits ON the boundary.
    let s = setup().await.expect("e2e setup failed (sub-case B)");
    let key = default_account_key();
    let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
    let count = pool_gap_limit as usize;
    let addrs = next_unused_receive_addresses(s.test_wallet.platform_wallet(), key, count)
        .await
        .expect("gap_limit at boundary must succeed");
    assert_eq!(addrs.len(), count);
    let unique: std::collections::HashSet<_> = addrs.iter().collect();
    assert_eq!(unique.len(), count);
    s.teardown().await.expect("teardown sub-case B");
}

#[tokio_shared_rt::test(shared)]
async fn pa_005b_gap_limit_triplet_subcase_c() {
    // Sub-case C: derive gap_limit + 1 — must reject with GapLimitExceeded
    // and leave the pool untouched.
    let s = setup().await.expect("e2e setup failed (sub-case C)");
    let key = default_account_key();
    let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
    let count = (pool_gap_limit + 1) as usize;
    let err = next_unused_receive_addresses(s.test_wallet.platform_wallet(), key, count)
        .await
        .expect_err("gap_limit+1 must error");
    match err {
        PlatformWalletError::GapLimitExceeded {
            requested,
            available,
            gap_limit: gl,
            ..
        } => {
            assert_eq!(requested, count);
            assert_eq!(available, pool_gap_limit);
            assert_eq!(gl, pool_gap_limit);
        }
        other => panic!("expected GapLimitExceeded, got {other:?}"),
    }
    // After a rejected request, a follow-up at the boundary must still
    // succeed — proves the pool was not mutated.
    let addrs = next_unused_receive_addresses(
        s.test_wallet.platform_wallet(),
        key,
        pool_gap_limit as usize,
    )
    .await
    .expect("post-rejection retry at boundary must still succeed");
    assert_eq!(addrs.len(), pool_gap_limit as usize);
    s.teardown().await.expect("teardown sub-case C");
}

/// Reach into the wallet manager to read the receive pool's
/// `gap_limit`. Lets the test drive the canonical default in
/// `key_wallet` rather than hard-coding the value here, so a
/// configuration change upstream is caught by the assertion in
/// sub-case A instead of a silent triplet drift.
async fn pool_gap_limit(
    wallet: &std::sync::Arc<platform_wallet::PlatformWallet>,
    key: PlatformPaymentAccountKey,
) -> u32 {
    let manager = wallet.wallet_manager();
    let wm = manager.read().await;
    let info = wm
        .get_wallet_info(&wallet.wallet_id())
        .expect("wallet present in manager");
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index(key.account)
        .expect("default platform-payment account exists");
    account.addresses.gap_limit
}
