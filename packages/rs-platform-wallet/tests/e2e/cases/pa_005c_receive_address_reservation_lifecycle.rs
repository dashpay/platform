//! PA-005c — Concurrent receive-address reservations and release/reissue.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-005c.
//! Priority: P2.
//!
//! Exercises the in-memory reservation lifecycle only. No funding or balance
//! polling is needed: allocation and release mutate the account pool under the
//! wallet-manager write lock.

use std::collections::BTreeSet;
use std::sync::Arc;

use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::PlatformPaymentAccountSpec;
use key_wallet::PoolStats;
use platform_wallet::PlatformWallet;
use tokio::sync::Barrier;
use tokio::task::JoinSet;

use crate::framework::prelude::*;

const CONCURRENT_REQUESTS: usize = 8;

fn default_account_key() -> PlatformPaymentAccountKey {
    let PlatformPaymentAccountSpec { account, key_class } = PlatformPaymentAccountSpec::default();
    PlatformPaymentAccountKey { account, key_class }
}

async fn pool_stats(
    wallet: &Arc<PlatformWallet>,
    account_key: PlatformPaymentAccountKey,
) -> PoolStats {
    let wallet_id = wallet.wallet_id();
    let wm = wallet.wallet_manager().read().await;
    let info = wm
        .get_wallet_info(&wallet_id)
        .expect("test wallet must remain registered");
    #[allow(deprecated)]
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index(account_key.account)
        .expect("default platform-payment account must exist");
    account.addresses.stats()
}

#[tokio_shared_rt::test(shared)]
async fn pa_005c_receive_address_reservation_lifecycle() {
    let s = setup().await.expect("e2e setup failed");
    let account_key = default_account_key();
    let wallet = Arc::clone(s.test_wallet.platform_wallet());

    let baseline = pool_stats(&wallet, account_key).await;
    assert_eq!(
        baseline.reserved_count, 1,
        "setup's slot-0 guard must contribute exactly one reservation"
    );

    let barrier = Arc::new(Barrier::new(CONCURRENT_REQUESTS + 1));
    let mut requests = JoinSet::new();
    for _ in 0..CONCURRENT_REQUESTS {
        let barrier = Arc::clone(&barrier);
        let wallet = Arc::clone(&wallet);
        requests.spawn(async move {
            barrier.wait().await;
            wallet
                .platform()
                .next_unused_receive_address(account_key)
                .await
        });
    }
    barrier.wait().await;

    let mut addresses = Vec::with_capacity(CONCURRENT_REQUESTS);
    while let Some(result) = requests.join_next().await {
        addresses.push(
            result
                .expect("receive-address task must not panic")
                .expect("concurrent receive-address reservation must succeed"),
        );
    }
    assert_eq!(addresses.len(), CONCURRENT_REQUESTS);
    assert_eq!(
        addresses.iter().cloned().collect::<BTreeSet<_>>().len(),
        CONCURRENT_REQUESTS,
        "all concurrent reservations must return pairwise-distinct addresses"
    );

    let after_reserve = pool_stats(&wallet, account_key).await;
    assert_eq!(
        after_reserve.reserved_count,
        baseline.reserved_count + CONCURRENT_REQUESTS as u32
    );
    assert_eq!(after_reserve.used_count, baseline.used_count);

    let released = addresses[0];
    assert!(
        wallet
            .platform()
            .release_receive_reservation(account_key, &released)
            .await
            .expect("first release must not error"),
        "first release must clear the reservation"
    );
    let after_release = pool_stats(&wallet, account_key).await;
    assert_eq!(
        after_release.reserved_count,
        after_reserve.reserved_count - 1
    );
    assert_eq!(after_release.used_count, baseline.used_count);

    assert!(
        !wallet
            .platform()
            .release_receive_reservation(account_key, &released)
            .await
            .expect("idempotent second release must not error"),
        "releasing the same address twice must return false"
    );
    let after_second_release = pool_stats(&wallet, account_key).await;
    assert_eq!(
        after_second_release.reserved_count,
        after_release.reserved_count
    );
    assert_eq!(after_second_release.used_count, after_release.used_count);

    let reissued = wallet
        .platform()
        .next_unused_receive_address(account_key)
        .await
        .expect("released address must be reservable again");
    assert_eq!(
        reissued, released,
        "released address must be re-issued first"
    );

    let final_stats = pool_stats(&wallet, account_key).await;
    assert_eq!(final_stats.reserved_count, after_reserve.reserved_count);
    assert_eq!(final_stats.used_count, baseline.used_count);

    s.teardown().await.expect("teardown");
}
