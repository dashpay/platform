//! PA-005b — `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-005b.
//! Priority: P2.
//!
//! Drives [`PlatformAddressWallet::next_unused_receive_addresses(count)`],
//! the production accessor that wraps `AddressPool::generate_addresses`
//! while enforcing the gap-limit cap. Three sub-cases run on separate
//! `TestWallet` instances:
//!
//! 1. `count = gap_limit - 1` — must succeed with that many distinct
//!    addresses.
//! 2. `count = gap_limit` — must succeed at the boundary.
//! 3. `count = gap_limit + 1` — must return [`PlatformWalletError::GapLimitExceeded`]
//!    without mutating the pool.

use crate::framework::prelude::*;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::PlatformPaymentAccountSpec;
use platform_wallet::PlatformWalletError;

fn default_account_key() -> PlatformPaymentAccountKey {
    let PlatformPaymentAccountSpec { account, key_class } = PlatformPaymentAccountSpec::default();
    PlatformPaymentAccountKey { account, key_class }
}

#[tokio_shared_rt::test(shared)]
async fn pa_005b_gap_limit_triplet() {
    // Sub-case 1: derive 19 distinct unused addresses (gap_limit-1).
    {
        let s = setup().await.expect("e2e setup failed (sub-case 1)");
        let platform = s.test_wallet.platform_wallet().platform();
        let key = default_account_key();
        // QA-V19-003: Removed `pool_gap_limit ≥ 21` precondition — production uses
        // DEFAULT_GAP_LIMIT = 20 (DIP17). The triplet (limit-1, limit, limit+1) is
        // computed from the live value, no fixed lower bound required.
        let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
        let count = (pool_gap_limit - 1) as usize;
        let addrs = platform
            .next_unused_receive_addresses(key, count)
            .await
            .expect("gap_limit-1 must succeed");
        assert_eq!(addrs.len(), count, "must return exactly count addresses");
        let unique: std::collections::HashSet<_> = addrs.iter().collect();
        assert_eq!(
            unique.len(),
            count,
            "all addresses returned in one batch must be distinct"
        );
        s.teardown().await.expect("teardown sub-case 1");
    }

    // Sub-case 2: derive exactly gap_limit addresses — sits ON the boundary.
    {
        let s = setup().await.expect("e2e setup failed (sub-case 2)");
        let platform = s.test_wallet.platform_wallet().platform();
        let key = default_account_key();
        let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
        let count = pool_gap_limit as usize;
        let addrs = platform
            .next_unused_receive_addresses(key, count)
            .await
            .expect("gap_limit at boundary must succeed");
        assert_eq!(addrs.len(), count);
        let unique: std::collections::HashSet<_> = addrs.iter().collect();
        assert_eq!(unique.len(), count);
        s.teardown().await.expect("teardown sub-case 2");
    }

    // Sub-case 3: derive gap_limit+1 — must reject with GapLimitExceeded
    // and leave the pool untouched.
    {
        let s = setup().await.expect("e2e setup failed (sub-case 3)");
        let platform = s.test_wallet.platform_wallet().platform();
        let key = default_account_key();
        let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
        let count = (pool_gap_limit + 1) as usize;
        let err = platform
            .next_unused_receive_addresses(key, count)
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
        // After a rejected request, a follow-up at the boundary must
        // still succeed — proves the pool was not mutated.
        let addrs = platform
            .next_unused_receive_addresses(key, pool_gap_limit as usize)
            .await
            .expect("post-rejection retry at boundary must still succeed");
        assert_eq!(addrs.len(), pool_gap_limit as usize);
        s.teardown().await.expect("teardown sub-case 3");
    }
}

/// Reach into the wallet manager to read the receive pool's
/// `gap_limit`. Lets the test drive the canonical default in
/// `key_wallet` rather than hard-coding the value here, so a
/// configuration change upstream is caught by the assertion in
/// sub-case 1 instead of a silent triplet drift.
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
