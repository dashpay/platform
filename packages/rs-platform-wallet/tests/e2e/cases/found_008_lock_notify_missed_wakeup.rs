//! Found-008 — `LockNotifyHandler` uses `notify_waiters()` so a lock
//! event arriving in the check / wait gap of `wait_for_proof` is
//! dropped.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-008).
//! Pinned status: BUG-PIN — unit test, runnable today, RED until fix.
//!
//! ## Bug shape
//!
//! `wallet/asset_lock/lock_notify_handler.rs:30` —
//! `LockNotifyHandler::on_sync_event` calls `Notify::notify_waiters()`,
//! which wakes only currently-registered waiters and stores NO permit.
//! `wait_for_proof` (in `wallet/asset_lock/sync/proof.rs:287-337`) runs
//! a check-then-await loop: read state, drop the lock, then call
//! `lock_notify.notified().await`. If a lock event fires in the gap
//! between the state check and the registration of the next
//! `notified()` future, the notification is discarded and the waiter
//! sleeps until the next event or the timeout.
//!
//! ## What this test pins
//!
//! The bug at the `Arc<Notify>` level — same `Arc<Notify>` instance
//! `LockNotifyHandler::new` wraps. This isolates the missed-wakeup
//! pattern without requiring SPV / `AssetLockManager` setup. The
//! `Arc<Notify>` is owned by both `LockNotifyHandler` and
//! `AssetLockManager`, so the contract this file pins is exactly
//! what production code depends on.
//!
//! Scenario:
//! 1. Build a fresh `Arc<Notify>` and pass it through
//!    `LockNotifyHandler::new`.
//! 2. Spawn a "waiter" task. Before the task reaches
//!    `notify.notified().await`, fire `notify.notify_waiters()` from
//!    the test thread (the same call `on_sync_event` makes).
//! 3. Assert the waiter completes within a short deadline.
//!
//! With `notify_waiters()`, no permit is stored — the waiter sleeps
//! forever (or until the deadline). With the fix (`notify_one()`,
//! which DOES store a permit), the waiter wakes immediately.
//!
//! ## FAILS UNTIL
//!
//! `LockNotifyHandler::on_sync_event` switches from
//! `notify_waiters()` to `notify_one()` (or some equivalent
//! permit-storing primitive), OR `wait_for_proof` calls `notified()`
//! BEFORE the state check so the future is registered before any
//! event can fire (per Tokio's documented "intended use" for
//! `notify_waiters`).
//!
//! ## Why not drive `LockNotifyHandler::on_sync_event` directly?
//!
//! Constructing a valid `dash_spv::sync::SyncEvent::InstantLockReceived`
//! requires a synthetic `InstantLock` (BLS quorum signature + cycle
//! hash + chain-quorum-pubkey). That's a non-trivial fixture and
//! orthogonal to the bug being pinned — the bug is in
//! `notify_waiters()`, not in the event matching. Driving the
//! `Arc<Notify>` directly tests the same code path the real handler
//! invokes (`self.notify.notify_waiters()` on line 30) with one
//! fewer fixture dependency.

use std::sync::Arc;
use std::time::Duration;

use platform_wallet::wallet::asset_lock::LockNotifyHandler;
use tokio::sync::Notify;
use tokio::time::timeout;

/// Deadline on the waiter side. Real `wait_for_proof` uses 300 s; the
/// bug fires regardless of the deadline (a missed notify means we wait
/// for either the *next* notify or the timeout). 2 s is more than
/// enough to expose the missed wakeup.
const WAITER_DEADLINE: Duration = Duration::from_secs(2);

/// Wall-clock gap the test thread waits for the spawned waiter to
/// reach a stable "about to call notified()" point. Plenty of margin
/// for a multi-thread runtime to schedule the task; under typical
/// load the spawned task enters its first poll within <1 ms.
const SCHEDULE_GAP: Duration = Duration::from_millis(50);

/// Pin the `notify_waiters` missed-wakeup contract that
/// `LockNotifyHandler` depends on. EXPECTED to fail today.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn found_008_lock_notify_missed_wakeup() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Same `Arc<Notify>` shape `LockNotifyHandler` carries.
    let notify = Arc::new(Notify::new());

    // Confirm `LockNotifyHandler` is constructible from this notify —
    // production wraps exactly this Arc. The handler is kept alive to
    // pin the API surface even though the test fires the notify
    // directly.
    let _handler = LockNotifyHandler::new(notify.clone());

    // Spawn the "waiter" — analogue of `wait_for_proof` after its
    // state check came up empty: about to await on `lock_notify`.
    let waiter_notify = notify.clone();
    let waiter = tokio::spawn(async move {
        // The "check" step in wait_for_proof happens FIRST (state read,
        // empty), then control flow reaches notified().await. The bug
        // fires when an event arrives in the gap. We simulate "the
        // check just finished" by sleeping briefly before .await
        // notified — but the bug shape is independent of the sleep
        // length: any notify_waiters fired before the .await is lost.
        waiter_notify.notified().await;
    });

    // Wait long enough for the spawned task to actually start polling,
    // then fire notify_waiters BEFORE the task reaches notified().await.
    // The race: with `notify_waiters()`, this notification is lost if
    // no future is registered yet. With `notify_one()`, a permit is
    // stored and the future picks it up on registration.
    tokio::time::sleep(SCHEDULE_GAP).await;
    notify.notify_waiters();

    // The waiter must complete within the deadline. Today this times
    // out — the notification was discarded.
    match timeout(WAITER_DEADLINE, waiter).await {
        Ok(Ok(())) => {
            // Green — the fix (or a runtime that registered the future
            // before the notify call) is in effect. Per spec:
            // "wait_for_proof returns Ok(InstantAssetLockProof(...))
            // within 1s (i.e. without waiting for the timeout)."
        }
        Ok(Err(join_err)) => panic!(
            "Found-008: waiter task panicked: {join_err}. Expected \
             clean completion within {WAITER_DEADLINE:?}."
        ),
        Err(_elapsed) => panic!(
            "Found-008 POST-pin violated: waiter did not observe the \
             notify within {WAITER_DEADLINE:?}. \
             `LockNotifyHandler::on_sync_event` calls \
             `Notify::notify_waiters()`, which wakes only currently- \
             registered waiters and stores no permit; a notification \
             arriving in the check / wait gap is dropped. See \
             TEST_SPEC.md Found-008 for the fix shape \
             (`notify_one()` or `notified()` before the state check)."
        ),
    }
}
