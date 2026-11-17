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
//!   must return [`GapLimitError::Exceeded`] without mutating the
//!   pool, and a follow-up boundary call must still succeed.
//!
//! ## Real-pool starting state (Fix-B rebaseline)
//!
//! Production builds the DIP-17 platform-payment receive pool via the
//! eager `AddressPool::new`, which fills indices `0..=gap_limit-1` at
//! construction (`highest_generated = Some(gap_limit-1)`), and the
//! QA-002 setup hook reserves index 0. The generated window is
//! already full, so no `gap_limit`-wide fresh window exists. A wallet
//! that has actually
//! cycled through its first gap window has its highest eagerly-built
//! index used; [`open_full_gap_window`] models exactly that by
//! marking index `gap_limit-1` used, which shifts the ceiling up by
//! `gap_limit` and opens a genuine `gap_limit`-wide window. The
//! triplet then pins the same DIP-17 boundary from the production
//! starting state instead of an empty-pool premise production never
//! reaches.

use crate::framework::gap_limit::{next_unused_receive_addresses, GapLimitError};
use crate::framework::prelude::*;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::PlatformPaymentAccountSpec;
use std::sync::Arc;

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
    let w = open_full_gap_window(s.test_wallet.platform_wallet(), key, pool_gap_limit).await;
    // Window opened by marking the highest eager index used: a genuine
    // `gap_limit`-wide fresh run now exists past `highest_generated`.
    assert_eq!(w.available, pool_gap_limit);

    let count = (pool_gap_limit - 1) as usize;
    let addrs = next_unused_receive_addresses(s.test_wallet.platform_wallet(), key, count)
        .await
        .expect("gap_limit-1 must succeed within the opened window");
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
    let w = open_full_gap_window(s.test_wallet.platform_wallet(), key, pool_gap_limit).await;
    assert_eq!(w.available, pool_gap_limit);

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
    // Sub-case C: derive gap_limit + 1 — must reject with GapLimitError::Exceeded
    // and leave the pool untouched.
    let s = setup().await.expect("e2e setup failed (sub-case C)");
    let key = default_account_key();
    let pool_gap_limit = pool_gap_limit(s.test_wallet.platform_wallet(), key).await;
    let w = open_full_gap_window(s.test_wallet.platform_wallet(), key, pool_gap_limit).await;
    assert_eq!(w.available, pool_gap_limit);

    let count = (pool_gap_limit + 1) as usize;
    let err = next_unused_receive_addresses(s.test_wallet.platform_wallet(), key, count)
        .await
        .expect_err("gap_limit+1 must error");
    match err {
        GapLimitError::Exceeded {
            requested,
            available,
            highest_used,
            highest_generated,
            gap_limit: gl,
        } => {
            // Every field is pinned against the LIVE watermarks read
            // back from the pool after the window was opened — eager
            // fill leaves `highest_generated = gap_limit-1`, and the
            // mark-used precondition leaves `highest_used = gap_limit-1`.
            assert_eq!(requested, count);
            assert_eq!(available, pool_gap_limit);
            assert_eq!(gl, pool_gap_limit);
            assert_eq!(highest_used, w.highest_used);
            assert_eq!(highest_generated, w.highest_generated);
            assert_eq!(highest_used, Some(pool_gap_limit - 1));
            assert_eq!(highest_generated, Some(pool_gap_limit - 1));
        }
        other => panic!("expected GapLimitError::Exceeded, got {other:?}"),
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

/// Live watermark snapshot taken right after [`open_full_gap_window`],
/// so sub-case C asserts the `Exceeded` fields against the real pool
/// state rather than recomputed constants.
struct GapWindow {
    highest_used: Option<u32>,
    highest_generated: Option<u32>,
    available: u32,
}

/// Mark the highest eagerly-generated index (`gap_limit - 1`) used so
/// the gap-limit ceiling shifts up by `gap_limit`, opening a genuine
/// `gap_limit`-wide fresh-unused window past `highest_generated`.
///
/// Models a real wallet that has cycled through its first DIP-17 gap
/// window: production's `AddressPool::new` eagerly fills `0..=gap-1`
/// and the QA-002 hook reserves index 0, so the generated window is
/// already full. Marking index `gap_limit-1` used reflects a
/// wallet whose highest built address has been spent to — a higher
/// fidelity premise than the empty pool the triplet originally assumed.
/// Returns the live watermarks plus the helper's derived headroom.
async fn open_full_gap_window(
    wallet: &Arc<platform_wallet::PlatformWallet>,
    key: PlatformPaymentAccountKey,
    gap_limit: u32,
) -> GapWindow {
    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let info = wm
        .get_wallet_info_mut(&wallet_id)
        .expect("wallet present in manager");
    // TODO: reaches into the deprecated platform_payment_managed_account
    // pool for state the modern PlatformPaymentAddressProvider doesn't yet
    // expose (mark_index_used mutation on a boundary index). Migrate or
    // retire once the pool moves to the provider (per @QuantumExplorer's
    // review on #3648).
    #[allow(deprecated)]
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index_mut(key.account)
        .expect("default platform-payment account exists");

    let top = gap_limit - 1;
    assert!(
        account.addresses.mark_index_used(top),
        "index {top} must be present (eager fill) and not yet used"
    );

    let highest_used = account.addresses.highest_used;
    let highest_generated = account.addresses.highest_generated;
    // Mirror the helper's headroom math (framework/gap_limit.rs) so the
    // window assertion fails loudly if the eager-fill contract drifts.
    let ceiling = highest_used
        .map(|h| h.saturating_add(gap_limit))
        .unwrap_or(gap_limit.saturating_sub(1));
    let next_index = highest_generated.map(|h| h.saturating_add(1)).unwrap_or(0);
    let available = ceiling.saturating_sub(next_index).saturating_add(1);

    GapWindow {
        highest_used,
        highest_generated,
        available,
    }
}

/// Reach into the wallet manager to read the receive pool's
/// `gap_limit`. Lets the test drive the canonical default in
/// `key_wallet` rather than hard-coding the value here, so a
/// configuration change upstream is caught by the assertion in
/// sub-case A instead of a silent triplet drift.
async fn pool_gap_limit(
    wallet: &Arc<platform_wallet::PlatformWallet>,
    key: PlatformPaymentAccountKey,
) -> u32 {
    wallet
        .platform()
        .platform_payment_account_gap_limit(key.account)
        .await
        .expect("default platform-payment account exists")
}
