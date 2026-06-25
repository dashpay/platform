//! Shared lifecycle state + pass protocol for the periodic sync
//! coordinators.
//!
//! The three coordinators ([`IdentitySyncManager`], [`PlatformAddressSyncManager`],
//! [`ShieldedSyncManager`]) each drive a background loop on the shared
//! [`ThreadRegistry`] and gate passes through an `is_syncing` / `quiescing`
//! handshake. That handshake, plus the interval and last-sync bookkeeping,
//! is identical across all three; it lives here so the (subtle, teardown-
//! critical) protocol has a single home and each coordinator keeps only its
//! domain-specific pass body.
//!
//! [`IdentitySyncManager`]: super::identity_sync::IdentitySyncManager
//! [`PlatformAddressSyncManager`]: super::platform_address_sync::PlatformAddressSyncManager
//! [`ShieldedSyncManager`]: super::shielded_sync::ShieldedSyncManager

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dash_async::{AtomicFlagGuard, DrainHook, ThreadRegistry, WorkerConfig};

use super::{
    CoordinatorThreadStatus, WalletWorker, COORDINATOR_WEIGHT, SHUTDOWN_JOIN_TIMEOUT_SECS,
};

/// Shared lifecycle state and pass-gating protocol for one periodic sync
/// coordinator. Each coordinator embeds one of these and delegates its
/// `start` / `stop` / `quiesce` / `is_running` / interval / pass-gate
/// surface to it.
pub(crate) struct CoordinatorLifecycle {
    registry: Arc<ThreadRegistry<WalletWorker>>,
    worker: WalletWorker,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// `Arc` so the registry drain hook (a `'static` closure) can capture a
    /// clone and raise the gate from inside `quiesce`.
    quiescing: Arc<AtomicBool>,
    last_sync_unix: AtomicU64,
}

impl CoordinatorLifecycle {
    pub(crate) fn new(
        registry: Arc<ThreadRegistry<WalletWorker>>,
        worker: WalletWorker,
        default_interval_secs: u64,
    ) -> Self {
        Self {
            registry,
            worker,
            interval_secs: AtomicU64::new(default_interval_secs),
            is_syncing: AtomicBool::new(false),
            quiescing: Arc::new(AtomicBool::new(false)),
            last_sync_unix: AtomicU64::new(0),
        }
    }

    /// The shared worker-lifecycle engine this coordinator's loop runs on.
    pub(crate) fn registry(&self) -> &Arc<ThreadRegistry<WalletWorker>> {
        &self.registry
    }

    /// This coordinator's registry key.
    pub(crate) fn worker(&self) -> WalletWorker {
        self.worker
    }

    /// Set the polling interval. Clamped to a minimum of 1s.
    pub(crate) fn set_interval(&self, interval: Duration) {
        let secs = interval.as_secs().max(1);
        self.interval_secs.store(secs, Ordering::Release);
    }

    /// Current polling interval.
    pub(crate) fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.load(Ordering::Acquire))
    }

    /// Current polling interval in whole seconds (for `Debug`).
    pub(crate) fn interval_secs(&self) -> u64 {
        self.interval_secs.load(Ordering::Acquire)
    }

    /// Whether the background loop is currently running.
    pub(crate) fn is_running(&self) -> bool {
        self.registry.is_running(self.worker)
    }

    /// Whether a sync pass is in flight right now.
    pub(crate) fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Acquire)
    }

    /// Unix seconds of the last completed pass, or `None` if none has ever
    /// completed.
    pub(crate) fn last_sync_unix_seconds(&self) -> Option<u64> {
        match self.last_sync_unix.load(Ordering::Acquire) {
            0 => None,
            n => Some(n),
        }
    }

    /// Record the unix-seconds stamp of a just-completed pass.
    pub(crate) fn store_last_sync_unix(&self, unix_secs: u64) {
        self.last_sync_unix.store(unix_secs, Ordering::Release);
    }

    /// The registry config a coordinator starts its loop with: coordinator
    /// teardown weight, the shared join budget, and the `quiescing`-raising
    /// drain hook.
    pub(crate) fn worker_config(&self) -> WorkerConfig {
        WorkerConfig {
            weight: COORDINATOR_WEIGHT,
            join_budget: Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS),
            drain: Some(self.drain_hook()),
        }
    }

    /// Drain hook handed to the registry: raise the `quiescing` gate so any
    /// pass past its `is_syncing` CAS bails. The registry then cancels the
    /// loop and joins the thread, so the barrier itself is instant.
    fn drain_hook(&self) -> DrainHook {
        let quiescing = Arc::clone(&self.quiescing);
        Arc::new(move || {
            let quiescing = Arc::clone(&quiescing);
            Box::pin(async move {
                quiescing.store(true, Ordering::Release);
            })
        })
    }

    /// Reopen the `quiescing` gate so a (re)start's passes can run; a prior
    /// quiesce raised it via the drain hook.
    pub(crate) fn reopen_quiescing_gate(&self) {
        self.quiescing.store(false, Ordering::Release);
    }

    /// Cancel-only stop: signal the loop and return immediately.
    pub(crate) fn stop(&self) {
        self.registry.cancel(self.worker);
    }

    /// Cancel the loop, drain any in-flight pass, and join the worker,
    /// returning its terminal status. Reopens the `quiescing` gate on every
    /// exit path (the gate is reset by the guard; reopening is safe because
    /// the loop has been cancelled, so no new pass starts).
    ///
    /// The gate is raised **here**, not left to the registry's drain hook:
    /// `registry.quiesce` early-returns `NotRunning` without running the
    /// hook when no background-loop slot is registered, so a coordinator
    /// with only direct `sync_now`/`sync_wallet` traffic (no running loop)
    /// would never see the gate go up — and a direct pass landing
    /// concurrently would slip past the barrier `clear_shielded`/`stop`
    /// promise. Raising it ourselves makes the "no new pass" gate hold
    /// regardless of whether a loop is registered, and preserves
    /// gate-before-cancel: it is up before `registry.quiesce` issues any
    /// cancel.
    pub(crate) async fn quiesce(&self) -> CoordinatorThreadStatus {
        // Gate up first (instant) and held until the guard drops on return.
        self.quiescing.store(true, Ordering::Release);
        let _quiescing_gate = AtomicFlagGuard::new(&self.quiescing);

        // Cancel + bounded join of the background loop (if any). A wedged
        // loop pass surfaces here as a non-clean `Timeout` rather than
        // hanging — its orphaned thread is tracked by the registry for
        // teardown, so we must not wait on it below.
        let status: CoordinatorThreadStatus = self.registry.quiesce(self.worker).await.into();

        // Drain a *direct* in-flight pass the registry could not: with no
        // loop slot, `registry.quiesce` returned `NotRunning` without
        // joining anything; with an idle loop it joined a thread that was
        // not the one holding `is_syncing`. Either way a `sync_now`/
        // `sync_wallet` that entered before the gate rose may still be in
        // flight. The gate keeps a new pass from starting, so this
        // converges, and a panicked pass clears the flag via its own RAII
        // guard. Only drain on a clean status: a non-clean one means a
        // wedged loop pass is the `is_syncing` holder (its thread was
        // orphaned, not joined), and waiting on it would reintroduce the
        // shutdown stall the registry's bounded join exists to prevent.
        if status.is_clean() {
            self.drain_in_flight_pass().await;
        }

        status
    }

    /// Poll until no sync pass holds `is_syncing`. Only sound to call with
    /// the `quiescing` gate already raised (so no new pass can start) and
    /// after the background loop has been cancel-joined (so the only
    /// possible holder is a direct, non-cancellable pass running to
    /// completion). Mirrors the registry's 5ms poll cadence.
    async fn drain_in_flight_pass(&self) {
        while self.is_syncing.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// Raise the `quiescing` gate and hold it raised until the returned
    /// guard drops. Where [`quiesce`](Self::quiesce) reopens the gate the
    /// instant it returns, this lets a multi-step teardown (e.g. Clear)
    /// keep new direct passes off across a check-then-wipe so the "no new
    /// pass" guarantee does not lapse between the two steps. In production
    /// only the shielded Clear flow needs this today; the coordinator pass-
    /// gate tests also exercise it.
    #[cfg(any(test, feature = "shielded"))]
    pub(crate) fn hold_quiescing_gate(&self) -> AtomicFlagGuard<'_> {
        self.quiescing.store(true, Ordering::Release);
        AtomicFlagGuard::new(&self.quiescing)
    }

    /// Enter a sync pass. Atomically claims the `is_syncing` slot, then
    /// checks the `quiescing` gate. Returns the RAII guard that clears
    /// `is_syncing` on drop, or `None` when the caller must bail without
    /// doing work — because a pass is already in flight, or a teardown has
    /// raised the gate. In the gated case the briefly-claimed slot is
    /// released before returning (the guard drops), so a later post-quiesce
    /// pass can still run.
    pub(crate) fn begin_pass(&self) -> Option<AtomicFlagGuard<'_>> {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }

        // RAII guard: clears `is_syncing` on every exit path, including
        // panics. Without it a panic inside the pass would leave
        // `is_syncing = true` forever and wedge `quiesce`'s drain loop.
        let guard = AtomicFlagGuard::new(&self.is_syncing);

        // A `quiesce` may have raised the gate between our CAS and here; if
        // so, bail (dropping `guard`, which clears the slot) so the drain
        // can complete and teardown gets a true "no further pass" barrier.
        if self.quiescing.load(Ordering::Acquire) {
            return None;
        }
        Some(guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn make_lifecycle() -> Arc<CoordinatorLifecycle> {
        let registry = ThreadRegistry::<WalletWorker>::new();
        Arc::new(CoordinatorLifecycle::new(
            registry,
            WalletWorker::IdentitySync,
            60,
        ))
    }

    /// With NO background loop registered, `quiesce` must still raise the
    /// `quiescing` gate — so a concurrent direct `sync_now`/`sync_wallet`
    /// that lands after it bails — and drain an already-in-flight direct
    /// pass before returning. The registry's drain hook cannot cover this:
    /// `registry.quiesce` early-returns `NotRunning` WITHOUT running the
    /// hook when no loop slot exists, so the gate would otherwise never go
    /// up and the in-flight pass would not be drained. Regression for the
    /// `clear_shielded`/`stop` contract ("a concurrent direct
    /// sync_now/sync_wallet is held off"). Must fail against the pre-fix
    /// `quiesce` that only delegated to the registry.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_raises_gate_and_drains_direct_pass_without_background_loop() {
        let lifecycle = make_lifecycle();
        assert!(
            !lifecycle.is_running(),
            "precondition: no background loop registered"
        );

        // A direct sync_now/sync_wallet pass already past `begin_pass`, held
        // in flight on a task until we release it.
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let lc_pass = Arc::clone(&lifecycle);
        let pass_task = tokio::spawn(async move {
            let _pass = lc_pass.begin_pass().expect("first pass enters the slot");
            ready_tx.send(()).expect("signal in-flight");
            release_rx.await.expect("await release");
            // `_pass` drops here → is_syncing = false
        });

        ready_rx.await.expect("pass reached in-flight");
        assert!(lifecycle.is_syncing(), "direct pass holds is_syncing");

        // Drive `quiesce` concurrently: it must raise the gate, then block
        // draining the in-flight pass.
        let lc_q = Arc::clone(&lifecycle);
        let quiesce_task = tokio::spawn(async move { lc_q.quiesce().await });

        // Give `quiesce` time to raise the gate and enter the drain.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            lifecycle.quiescing.load(Ordering::Acquire),
            "quiesce must raise the gate even with no background loop registered"
        );
        assert!(
            lifecycle.is_syncing(),
            "in-flight direct pass still held; quiesce has not skipped the drain"
        );
        assert!(
            !quiesce_task.is_finished(),
            "quiesce must block until the in-flight pass drains"
        );

        // Release the pass; `quiesce` drains `is_syncing`, then returns.
        release_tx.send(()).expect("release the pass");
        let status = tokio::time::timeout(Duration::from_secs(2), quiesce_task)
            .await
            .expect("quiesce completes once the pass drains")
            .expect("quiesce task joined");
        assert_eq!(status, CoordinatorThreadStatus::NotRunning);
        assert!(
            !lifecycle.is_syncing(),
            "is_syncing was drained before quiesce returned"
        );

        pass_task.await.expect("pass task joined");
    }
}
