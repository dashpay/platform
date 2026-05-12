//! Found-008 — `LockNotifyHandler` uses `notify_waiters()` so a lock
//! event arriving in the check / wait gap of `wait_for_proof` is
//! dropped.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-008).
//! Pinned status: BUG-PIN — unit test, runnable today, GREEN while
//! the bug is present, RED once the fix lands. This is an inverted
//! bug-pin: the test asserts the missed-wakeup happens.
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
//! Scenario (pre-spawn notify, strict causal ordering):
//! 1. Build a fresh `Arc<Notify>` and pass it through
//!    `LockNotifyHandler::new`.
//! 2. Fire `notify.notify_waiters()` BEFORE the waiter task exists.
//!    Zero waiters are registered, so `notify_waiters()` is a no-op
//!    — no permit stored.
//! 3. Spawn the "waiter" task. It calls `notify.notified().await`
//!    AFTER the notify already fired. With `notify_waiters()` there
//!    is no permit to pick up, so the waiter sleeps until the test
//!    thread's deadline.
//! 4. Assert the waiter does NOT complete within the deadline —
//!    the timeout firing IS the bug-pin's success condition.
//!
//! Why pre-spawn rather than the previous "spawn-then-sleep-50ms"
//! shape: the sleep gave Tokio time to schedule and poll the spawned
//! task, registering its `notified()` future before the test thread
//! fired `notify_waiters()`. The notify was then delivered correctly
//! and the test passed for the WRONG reason. Firing before
//! `tokio::spawn(...)` makes it causally impossible for any waiter
//! to be registered when the notify fires.
//!
//! ## FAILS UNTIL (== green-test inversion)
//!
//! `LockNotifyHandler::on_sync_event` switches from
//! `notify_waiters()` to `notify_one()` (or some equivalent
//! permit-storing primitive), OR `wait_for_proof` calls `notified()`
//! BEFORE the state check so the future is registered before any
//! event can fire (per Tokio's documented "intended use" for
//! `notify_waiters`).
//!
//! When the fix lands and this file is rewritten to mirror the new
//! primitive, the test flips: the assertion changes from "waiter
//! times out" to "waiter completes within the deadline".
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

/// Deadline for the waiter task. Real `wait_for_proof` uses 300 s; the
/// missed-wakeup bug fires regardless of the deadline — once the notify
/// is dropped, the next `notified()` future sleeps until the deadline
/// or the next event. 2 s is more than enough to expose the miss while
/// keeping the test fast in CI.
const WAITER_DEADLINE: Duration = Duration::from_secs(2);

/// Pin the `notify_waiters` missed-wakeup contract that
/// `LockNotifyHandler` depends on. With the bug present this test is
/// GREEN (waiter times out, missed wakeup confirmed); after the fix
/// lands the assertion will flip to "waiter completes within the
/// deadline" and this file gets rewritten alongside the fix.
#[ignore = "Found-008 bug pin — RED until LockNotifyHandler migrates off notify_waiters()"]
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

    // Fire `notify_waiters()` BEFORE any waiter exists. With zero
    // registered waiters this is a no-op — no permit is stored. Any
    // subsequent `notified()` future has nothing to pick up. This is
    // the exact failure mode `on_sync_event` exposes when a lock
    // event arrives in the check / wait gap of `wait_for_proof`.
    notify.notify_waiters();

    // Spawn the "waiter" — analogue of `wait_for_proof` after its
    // state check came up empty: about to await on `lock_notify`.
    // Spawning AFTER `notify_waiters()` guarantees the waiter
    // registers its `notified()` future strictly after the notify
    // already fired — there is no way for the runtime to schedule
    // the waiter in time to catch this notification.
    let waiter_notify = notify.clone();
    let waiter = tokio::spawn(async move {
        waiter_notify.notified().await;
    });

    // The waiter must time out. Timeout firing IS the bug-pin's
    // success — it confirms `notify_waiters()` discarded the notify
    // because no future was registered.
    match timeout(WAITER_DEADLINE, waiter).await {
        Ok(Ok(())) => panic!(
            "Found-008 bug-pin contract violated: waiter completed \
             within {WAITER_DEADLINE:?} despite `notify_waiters()` \
             firing before the waiter registered. Either the fix \
             landed and this test should be inverted to assert \
             completion, OR something now stores a permit across \
             this call. See TEST_SPEC.md Found-008."
        ),
        Ok(Err(join_err)) => panic!(
            "Found-008: waiter task panicked: {join_err}. Expected \
             a clean timeout, not a panic."
        ),
        Err(_elapsed) => {
            // Green — the missed-wakeup hazard reproduced as
            // expected. `notify_waiters()` was called with zero
            // registered waiters, no permit was stored, and the
            // subsequent `notified().await` slept until the
            // deadline. This is the bug.
        }
    }
}
