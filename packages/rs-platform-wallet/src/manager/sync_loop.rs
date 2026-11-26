//! The lifecycle the periodic sync coordinators share.
//!
//! Every coordinator runs the same loop on a registry-owned OS thread,
//! admits a pass the same way (claim the `is_syncing` slot, then honor the
//! [`QuiesceGate`]) and drains it the same way. [`SyncLoop`] holds that
//! state and those steps; a coordinator embeds one and keeps what is its
//! own — the pass body, its result type, its completion event and, for the
//! shielded coordinator, its pass-cancellation token.
//!
//! The gate, the slot guard and the drain themselves ([`QuiesceGate`],
//! [`SyncSlotGuard`], [`drain_pass`]) live in the parent module.

use std::future::Future;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dash_async::{ThreadRegistry, WorkerConfig};

use crate::manager::{
    coordinator_worker_config, drain_pass, QuiesceGate, QuiesceGuard, SyncSlotGuard, WalletWorker,
    LATCHED_RETRY_INTERVAL,
};

/// Loop, admission and drain state of one sync coordinator.
pub(crate) struct SyncLoop {
    /// Registry key of this coordinator's loop thread.
    worker: WalletWorker,
    /// Shared registry that owns the loop's lifecycle: it spawns the OS
    /// thread, owns its cancellation token, and joins it at shutdown. A
    /// generation-guarded slot handles a `stop()` + quick `start()`
    /// without a stale loop clobbering the new one.
    registry: Arc<ThreadRegistry<WalletWorker>>,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Gates new passes while a drain waits out an in-flight one, while a
    /// [`QuiesceGuard`] holder mutates state, and terminally once shutdown
    /// seals it. A pass bails (after taking the `is_syncing` slot) when it
    /// is closed, so once a drain observes `is_syncing == false` no
    /// further pass can start.
    gate: QuiesceGate,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

/// Outcome of asking to run a pass.
#[must_use = "check admission and hold the permit until the pass and its callbacks finish"]
pub(crate) enum Admission<'a> {
    /// The pass may run; it holds the slot until the permit drops.
    Admitted(PassPermit<'a>),
    /// Another pass holds the slot.
    Busy,
    /// The slot was free but the gate is closed (a drain in flight, a
    /// held guard, a latched timeout or the seal). The slot has been
    /// released again.
    Closed,
}

/// Holds a coordinator's `is_syncing` slot for one pass and releases it on
/// drop, **including panic unwind out of the pass body**. Declare it where
/// the pass starts: it must outlive the completion-event dispatch, so a
/// drain waiting on the slot also waits out the host callback.
pub(crate) struct PassPermit<'a> {
    _slot: SyncSlotGuard<'a>,
}

impl SyncLoop {
    pub(crate) fn new(
        worker: WalletWorker,
        registry: Arc<ThreadRegistry<WalletWorker>>,
        default_interval_secs: u64,
    ) -> Self {
        Self {
            worker,
            registry,
            interval_secs: AtomicU64::new(default_interval_secs),
            is_syncing: AtomicBool::new(false),
            gate: QuiesceGate::default(),
            last_sync_unix: AtomicU64::new(0),
        }
    }

    /// Set the polling interval. Clamped to a minimum of 1s. The running
    /// loop picks this up on its next sleep.
    pub(crate) fn set_interval(&self, interval: Duration) {
        let secs = interval.as_secs().max(1);
        self.interval_secs.store(secs, Ordering::Release);
    }

    /// Current polling interval.
    pub(crate) fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.load(Ordering::Acquire))
    }

    /// Whether the background loop is currently running.
    pub(crate) fn is_running(&self) -> bool {
        self.registry.is_running(self.worker)
    }

    /// Whether a sync pass is in flight right now.
    pub(crate) fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Acquire)
    }

    /// Unix seconds of the last completed pass, or `None` if no pass has
    /// ever completed.
    pub(crate) fn last_sync_unix_seconds(&self) -> Option<u64> {
        match self.last_sync_unix.load(Ordering::Acquire) {
            0 => None,
            n => Some(n),
        }
    }

    /// Request cancellation of the loop thread. Cancel-only: a pass
    /// already running keeps going; [`drain`](Self::drain) is the barrier.
    pub(crate) fn cancel_loop(&self) {
        self.registry.cancel(self.worker);
    }

    /// Ask to run a pass: claim the `is_syncing` slot, then honor the gate.
    pub(crate) fn admit(&self) -> Admission<'_> {
        // A drained timeout latch is cleared here as well as in
        // `start_sync_loop`: with no loop running, a one-shot pass after a
        // timed-out drain would otherwise bail on the stale latch until
        // something restarted the loop. Safe before the CAS — a false
        // `is_syncing` means no pass is in flight — and the gate check
        // after the CAS still honors a live hold or the seal.
        self.gate.reopen_if_drained(&self.is_syncing);
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Admission::Busy;
        }
        // Clears `is_syncing` on every exit path — including panic unwind —
        // so a failed pass can never wedge a drain.
        let slot = SyncSlotGuard(&self.is_syncing);

        // A drain may have closed the gate between our CAS and here; if
        // so, release the slot and bail without running a pass so the
        // drain can complete and its caller gets a true barrier.
        if self.gate.is_closed() {
            return Admission::Closed;
        }
        Admission::Admitted(PassPermit { _slot: slot })
    }

    /// Whether new sync passes are currently barred — a drain in flight, a
    /// held guard, a latched timeout, or the terminal seal `shutdown`
    /// applies. A pass that outlived a drain reads this before it stores.
    pub(crate) fn admission_closed(&self) -> bool {
        self.gate.is_closed()
    }

    /// Latch admission closed until the pass now holding the slot has
    /// released it — what a timed-out [`drain`](Self::drain) does itself.
    /// For a coordinator whose drain has a second wait of its own that can
    /// time out after the slot fell.
    pub(crate) fn latch_admission_closed(&self) {
        self.gate.latch_closed();
    }

    /// Close admission for good. Call it while still holding the drain's
    /// guard, so the guard's drop cannot reopen the gate.
    pub(crate) fn seal_admission(&self) {
        self.gate.seal();
    }

    /// Record `unix` as the completion time of the pass that just ran.
    pub(crate) fn record_completed(&self, unix: u64) {
        self.last_sync_unix.store(unix, Ordering::Release);
    }

    /// Wall-clock unix seconds, `0` if the clock is before the epoch.
    pub(crate) fn unix_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Close admission, call `stop`, then wait up to `budget` for the
    /// in-flight pass to release the slot — see [`drain_pass`].
    ///
    /// The returned guard keeps admission shut until it drops. `None`
    /// means the pass did not drain in time: the gate is left latched
    /// closed and the caller must fail closed.
    pub(crate) async fn drain(
        &self,
        stop: impl FnOnce(),
        budget: Duration,
    ) -> Option<QuiesceGuard<'_>> {
        drain_pass(&self.gate, &self.is_syncing, stop, budget).await
    }

    /// [`drain`](Self::drain) that **seals** the gate: admission never
    /// reopens on this coordinator instance. Returns whether the pass
    /// drained.
    pub(crate) async fn drain_sealed(&self, stop: impl FnOnce(), budget: Duration) -> bool {
        let guard = self.drain(stop, budget).await;
        let drained = guard.is_some();
        // Seal before the guard drops so its Drop cannot reopen.
        self.seal_admission();
        drop(guard);
        drained
    }

    /// Test-only: claim the `is_syncing` slot and never release it,
    /// standing in for a pass wedged in a network / persister await.
    /// Returns `false` if the slot was already taken.
    #[cfg(test)]
    pub(crate) fn wedge_slot_for_test(&self) -> bool {
        self.is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Test-only: release a slot taken by
    /// [`wedge_slot_for_test`](Self::wedge_slot_for_test), as the wedged
    /// pass finally draining would.
    #[cfg(test)]
    pub(crate) fn release_slot_for_test(&self) {
        self.is_syncing.store(false, Ordering::Release);
    }
}

/// A coordinator whose background loop [`start_sync_loop`] can run.
pub(crate) trait SyncCoordinator: Send + Sync + 'static {
    /// The loop state this coordinator embeds.
    fn sync_loop(&self) -> &SyncLoop;

    /// One pass at the background cadence; its result is discarded. The
    /// future is polled on the loop thread, so it need not be `Send`.
    fn background_pass(&self) -> impl Future<Output = ()>;
}

/// Start `this` coordinator's background loop. Idempotent — a no-op if the
/// loop is already running, or if teardown has begun.
///
/// The loop runs on a dedicated registry-owned OS thread, not on a tokio
/// worker: the SDK futures a pass drives are `!Send`, so they cannot ride
/// on `tokio::spawn`. `Handle::block_on` keeps them on the main runtime's
/// reactor; only the polling thread is dedicated. `stack_size` overrides
/// the platform default stack for that thread, and `label` names the
/// coordinator in the log line below.
///
/// The first pass runs immediately; later ones every
/// [`SyncLoop::interval`].
pub(crate) fn start_sync_loop<C: SyncCoordinator>(
    this: Arc<C>,
    stack_size: Option<NonZeroUsize>,
    label: &'static str,
) {
    let handle = tokio::runtime::Handle::current();
    let sync = this.sync_loop();
    let registry = Arc::clone(&sync.registry);
    let worker = sync.worker;
    let cfg = WorkerConfig {
        stack_size,
        ..coordinator_worker_config()
    };
    // A timed-out drain latched the gate; reopen it once the wedged pass
    // has drained (now, or after each pass below).
    if !sync.gate.reopen_if_drained(&sync.is_syncing) {
        tracing::debug!(
            "{label} sync loop started behind a wedged pass; passes resume once it drains"
        );
    }
    // The registry owns the whole lifecycle under one slot lock: it takes
    // the `closing` / `clearing` latches, installs the cancellation token,
    // spawns the thread, and parks any still-draining prior generation, so
    // there is no check-then-spawn gap to race.
    registry.start_thread(worker, cfg, move |cancel| {
        handle.block_on(async move {
            let sync = this.sync_loop();
            loop {
                if cancel.is_cancelled() {
                    break;
                }

                this.background_pass().await;

                // The pass clears a drained latch itself before its slot CAS.
                // Sample the latch AFTER the pass, so a latch raised or cleared
                // while it ran is what picks the cadence: behind a still-wedged
                // pass the call above bailed, so poll the slot briskly instead of
                // sleeping a full interval and passes resume promptly once that
                // pass drains.
                let gate_open = sync.gate.reopen_if_drained(&sync.is_syncing);
                let interval = if gate_open {
                    sync.interval()
                } else {
                    sync.interval().min(LATCHED_RETRY_INTERVAL)
                };
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = cancel.cancelled() => break,
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::AtomicUsize;

    fn make_loop() -> SyncLoop {
        SyncLoop::new(WalletWorker::DpnsSync, ThreadRegistry::new(), 60)
    }

    #[test]
    fn should_admit_a_pass_and_release_the_slot_when_the_permit_drops() {
        let sync = make_loop();

        let admission = sync.admit();
        assert!(matches!(admission, Admission::Admitted(_)));
        assert!(sync.is_syncing(), "the permit holds the slot");

        drop(admission);
        assert!(!sync.is_syncing());
    }

    #[test]
    fn should_report_busy_and_leave_the_slot_with_its_owner() {
        let sync = make_loop();
        let first = sync.admit();

        assert!(matches!(sync.admit(), Admission::Busy));
        assert!(
            sync.is_syncing(),
            "the losing call must not release the in-flight pass's slot"
        );

        drop(first);
        assert!(matches!(sync.admit(), Admission::Admitted(_)));
    }

    #[tokio::test]
    async fn should_report_closed_and_release_the_slot_while_the_gate_is_held() {
        let sync = make_loop();
        let guard = sync
            .drain(|| {}, Duration::from_secs(1))
            .await
            .expect("an idle loop drains immediately");

        assert!(matches!(sync.admit(), Admission::Closed));
        assert!(!sync.is_syncing(), "the slot must be released on the bail");

        drop(guard);
        assert!(matches!(sync.admit(), Admission::Admitted(_)));
    }

    #[tokio::test]
    async fn should_release_the_slot_when_a_pass_panics() {
        let sync = make_loop();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _permit = sync.admit();
            panic!("pass body panicked");
        }));

        assert!(result.is_err());
        assert!(
            !sync.is_syncing(),
            "the permit clears the slot during unwind"
        );
    }

    #[tokio::test]
    async fn should_call_stop_once_and_latch_the_gate_when_the_drain_times_out() {
        let sync = make_loop();
        assert!(sync.wedge_slot_for_test());
        let stops = AtomicUsize::new(0);

        let guard = sync
            .drain(
                || {
                    stops.fetch_add(1, Ordering::SeqCst);
                },
                Duration::from_millis(50),
            )
            .await;

        assert!(guard.is_none(), "a wedged pass must time the drain out");
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        assert!(sync.admission_closed(), "timed-out drain latches the gate");

        // Still wedged: admission loses the slot and keeps the latch.
        assert!(matches!(sync.admit(), Admission::Busy));
        assert!(sync.admission_closed());

        // Drained: the next admission clears the latch itself.
        sync.release_slot_for_test();
        assert!(matches!(sync.admit(), Admission::Admitted(_)));
        assert!(!sync.admission_closed());
    }

    #[tokio::test]
    async fn should_seal_admission_for_good() {
        let sync = make_loop();

        assert!(sync.drain_sealed(|| {}, Duration::from_secs(1)).await);

        assert!(matches!(sync.admit(), Admission::Closed));
        let later = sync.drain(|| {}, Duration::from_secs(1)).await;
        assert!(later.is_some(), "an idle sealed loop still drains");
        drop(later);
        assert!(matches!(sync.admit(), Admission::Closed));
    }

    #[test]
    fn should_stamp_and_report_the_last_completed_pass() {
        let sync = make_loop();
        assert_eq!(sync.last_sync_unix_seconds(), None);

        sync.record_completed(1_700_000_000);

        assert_eq!(sync.last_sync_unix_seconds(), Some(1_700_000_000));
        assert_ne!(SyncLoop::unix_now(), 0);
    }

    /// A coordinator whose pass only counts itself.
    struct CountingCoordinator {
        sync: SyncLoop,
        passes: AtomicUsize,
    }

    impl SyncCoordinator for CountingCoordinator {
        fn sync_loop(&self) -> &SyncLoop {
            &self.sync
        }

        async fn background_pass(&self) {
            if let Admission::Admitted(_permit) = self.sync.admit() {
                self.passes.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    /// Behind a latched gate the loop polls at `LATCHED_RETRY_INTERVAL`
    /// instead of its interval, so passes resume about a second after the
    /// wedged pass drains even with a long interval configured.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_poll_briskly_behind_a_latched_gate_and_resume_when_it_drains() {
        let coordinator = Arc::new(CountingCoordinator {
            sync: make_loop(),
            passes: AtomicUsize::new(0),
        });
        let sync = coordinator.sync_loop();
        sync.set_interval(Duration::from_secs(3600));

        assert!(sync.wedge_slot_for_test());
        assert!(sync.drain(|| {}, Duration::from_millis(50)).await.is_none());

        start_sync_loop(Arc::clone(&coordinator), None, "counting");
        assert!(sync.is_running());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(coordinator.passes.load(Ordering::SeqCst), 0);

        sync.release_slot_for_test();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while coordinator.passes.load(Ordering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the loop slept its full interval behind the latch"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!sync.admission_closed());

        assert!(sync
            .drain(|| sync.cancel_loop(), Duration::from_secs(2))
            .await
            .is_some());
        assert!(!sync.is_running());
    }
}
