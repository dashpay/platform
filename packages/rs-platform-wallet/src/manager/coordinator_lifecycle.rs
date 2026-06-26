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

use dash_async::{AtomicFlagGuard, DrainHook, ThreadRegistry, WorkerConfig, WorkerStatus};

use super::{WalletWorker, COORDINATOR_WEIGHT, SHUTDOWN_JOIN_TIMEOUT_SECS};

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
                // SeqCst: store-half of the `quiescing`<->`is_syncing`
                // handshake (see `begin_pass`).
                quiescing.store(true, Ordering::SeqCst);
            })
        })
    }

    /// Spawn the standard coordinator loop on the registry: reopen the
    /// quiescing gate, take the worker config, and drive
    /// `biased; cancel-first` select! of `pass()` followed by an
    /// `interval()` sleep — both arms break on cancellation. The loop
    /// runs inside `Handle::block_on` on a fresh OS thread (so `pass`
    /// can yield `!Send` SDK futures); cancellation drops an in-flight
    /// `pass()` at its next `.await`.
    ///
    /// Each coordinator's `start` calls this with a thunk that captures its
    /// `Arc<Self>` and invokes its own `sync_now`.
    pub(crate) fn spawn_periodic_loop<F, Fut, I>(&self, pass: F, interval: I)
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()>,
        I: Fn() -> Duration + Send + 'static,
    {
        // Bail before lowering the gate when our key is in the registry's
        // clearing set: a concurrent `clear_shielded` is holding both the
        // per-key latch AND the quiescing gate continuously. `start_thread`
        // below would refuse the start (latch), but `reopen_quiescing_gate`
        // is the wallet-side flag — the registry latch does not protect it.
        // Without this check we'd lower the gate `clear_shielded` is
        // counting on, opening a window for a direct `sync_now`/`sync_wallet`
        // to slip past `begin_pass` and re-persist into the wiping store.
        if self.registry.is_clearing(self.worker) {
            return;
        }
        self.reopen_quiescing_gate();
        let cfg = self.worker_config();
        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let worker = self.worker;
        registry.start_thread(worker, cfg, move |cancel| {
            handle.block_on(async move {
                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        _ = pass() => {}
                    }
                    let sleep_for = interval();
                    tokio::select! {
                        _ = tokio::time::sleep(sleep_for) => {}
                        _ = cancel.cancelled() => break,
                    }
                }
            });
        });
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
    /// with only direct pass traffic (no running loop) would never see the
    /// gate go up — and a direct pass landing concurrently would slip past
    /// the barrier `clear_shielded`/`stop` promise. Raising it ourselves
    /// makes the "no new pass" gate hold regardless of whether a loop is
    /// registered, and preserves gate-before-cancel: it is up before
    /// `registry.quiesce` issues any cancel.
    ///
    /// "Direct pass" here means the gated entry points that take the
    /// `is_syncing` slot via [`begin_pass`](Self::begin_pass): every
    /// coordinator's `sync_now`, plus the shielded coordinator's
    /// `sync_wallet`. The platform-address coordinator's `sync_wallet` is
    /// intentionally **ungated** (it never touches `is_syncing`; callers
    /// that need exclusion gate themselves), so the gate/drain barrier does
    /// not apply to it.
    pub(crate) async fn quiesce(&self) -> WorkerStatus {
        // If a concurrent `clear_shielded` is mid-flight for our key,
        // the clear is holding the same `quiescing` atomic raised
        // continuously via `hold_quiescing_gate`. The local
        // `AtomicFlagGuard` below would unconditionally lower that gate
        // on Drop — lapsing the clear's "no new pass" barrier and
        // letting a direct `sync_now`/`sync_wallet` slip in to re-
        // persist into the wiping store. Bail before installing the
        // guard; the in-flight clear is already draining + joining the
        // worker under its own gate.
        if self.registry.is_clearing(self.worker) {
            return WorkerStatus::NotRunning;
        }
        // Gate up first (instant) and held until the guard drops on return.
        // SeqCst: store-half of the `quiescing`<->`is_syncing` handshake
        // (see `begin_pass`).
        self.quiescing.store(true, Ordering::SeqCst);
        let _quiescing_gate = AtomicFlagGuard::new(&self.quiescing);
        self.cancel_join_and_drain().await
    }

    /// Like [`quiesce`](Self::quiesce) but for a caller that has **already**
    /// raised the `quiescing` gate (via [`hold_quiescing_gate`](Self::hold_quiescing_gate))
    /// and will keep holding it: this neither raises nor lowers the gate, so
    /// a multi-step teardown (the shielded Clear flow) keeps the "no new
    /// pass" barrier raised *continuously* across the drain, the orphan-
    /// liveness check, and the store wipe — with no lapse for a direct
    /// `sync_now`/`sync_wallet` to slip through and re-persist into the
    /// store being cleared. (`quiesce`'s own RAII guard would lower the gate
    /// the instant it returned, which is why Clear cannot just call it and
    /// re-raise afterwards: a single shared `AtomicFlagGuard` always clears
    /// the flag on drop, so the re-raise would leave a window.) Gate-before-
    /// cancel still holds: the caller raised the gate before this runs.
    #[cfg(any(test, feature = "shielded"))]
    pub(crate) async fn quiesce_under_held_gate(&self) -> WorkerStatus {
        debug_assert!(
            self.quiescing.load(Ordering::Acquire),
            "quiesce_under_held_gate requires the caller to already hold the quiescing gate"
        );
        self.cancel_join_and_drain().await
    }

    /// Cancel + bounded-join the background loop (if any), then drain a
    /// direct in-flight pass on a clean status. Assumes the `quiescing` gate
    /// is **already raised** (by [`quiesce`](Self::quiesce)'s own guard or a
    /// caller's hold guard) and does not touch it.
    ///
    /// A wedged loop pass surfaces from `registry.quiesce` as a non-clean
    /// `Timeout` rather than hanging — its orphaned thread is tracked by the
    /// registry for teardown, so the drain below must not wait on it. On a
    /// clean status the only possible `is_syncing` holder is a direct
    /// `sync_now`/`sync_wallet` that entered before the gate rose (with no
    /// loop slot `registry.quiesce` joined nothing; with an idle loop it
    /// joined a thread that was not the one holding the flag). The raised
    /// gate keeps a new pass from starting, so the drain converges, and a
    /// panicked pass clears the flag via its own RAII guard.
    async fn cancel_join_and_drain(&self) -> WorkerStatus {
        let status = self.registry.quiesce(self.worker).await;
        if status.is_clean() {
            self.drain_in_flight_pass().await;
        }
        status
    }

    /// Poll until no sync pass holds `is_syncing`. Only sound to call with
    /// the `quiescing` gate already raised (so no new pass can start) and
    /// after the background loop has been cancel-joined (so the only
    /// possible holder is a direct, non-cancellable pass running to
    /// completion). Mirrors the registry's 5ms poll cadence. Unbounded by
    /// design — the caller bounds the whole teardown (the FFI `stop` /
    /// `clear` bridges wrap it in a `SHUTDOWN_JOIN_TIMEOUT_SECS` timeout).
    async fn drain_in_flight_pass(&self) {
        // SeqCst: load-half of the `quiescing`<->`is_syncing` handshake (see
        // `begin_pass`). Pairs with `begin_pass`'s SeqCst CAS so a pass that
        // claimed the slot just as the gate rose is observed here and waited
        // out, rather than slipping past an unsynchronized read.
        while self.is_syncing.load(Ordering::SeqCst) {
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
        // SeqCst: store-half of the `quiescing`<->`is_syncing` handshake (see
        // `begin_pass`). The Clear flow raises the gate through here, so this
        // raise must be self-fencing just like `quiesce`'s.
        self.quiescing.store(true, Ordering::SeqCst);
        AtomicFlagGuard::new(&self.quiescing)
    }

    /// Test-only read of the `quiescing` flag. Used by regression tests
    /// that assert the gate stays raised across a refused (re)start.
    #[cfg(test)]
    pub(crate) fn quiescing_load_for_test(&self, ordering: Ordering) -> bool {
        self.quiescing.load(ordering)
    }

    /// Enter a sync pass. Atomically claims the `is_syncing` slot, then
    /// checks the `quiescing` gate. Returns the RAII guard that clears
    /// `is_syncing` on drop, or `None` when the caller must bail without
    /// doing work — because a pass is already in flight, or a teardown has
    /// raised the gate. In the gated case the briefly-claimed slot is
    /// released before returning (the guard drops), so a later post-quiesce
    /// pass can still run.
    pub(crate) fn begin_pass(&self) -> Option<AtomicFlagGuard<'_>> {
        // LOAD-BEARING MEMORY ORDERING: the `is_syncing` claim (this CAS) and
        // the `quiescing` gate read below form a Dekker-style mutual-exclusion
        // handshake with `quiesce`'s `store(quiescing) … load(is_syncing)`.
        // The guarantee we need is that a teardown and a pass-entry can never
        // BOTH miss each other — either this pass observes the raised gate and
        // bails, or the drain observes our `is_syncing` claim and waits it
        // out. That is a StoreLoad relationship across two distinct atomics,
        // which Release/Acquire do NOT order; only SeqCst (a single total
        // order over all four ops) does. So the CAS *store* here, the gate
        // load here, and the matching `store(quiescing)` / `load(is_syncing)`
        // on the teardown side are all `SeqCst`. (Today the lock `registry`
        // takes would also fence this, but that is incidental — relying on it
        // would make the handshake silently fragile to a lock-free refactor.)
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
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
        // SeqCst — load-half of the handshake described above.
        if self.quiescing.load(Ordering::SeqCst) {
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
    /// up and the in-flight pass would not be drained. Guards the
    /// `clear_shielded`/`stop` contract ("a concurrent direct
    /// sync_now/sync_wallet is held off"). Non-vacuous: a `quiesce` that
    /// merely delegated to the registry would raise no gate and let the
    /// concurrent pass slip through.
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
        assert_eq!(status, WorkerStatus::NotRunning);
        assert!(
            !lifecycle.is_syncing(),
            "is_syncing was drained before quiesce returned"
        );

        pass_task.await.expect("pass task joined");
    }

    /// `quiesce_under_held_gate` must NOT lower the `quiescing` gate the
    /// caller is holding — the mechanism that lets the shielded Clear flow
    /// keep the "no new pass" barrier raised *continuously* across the
    /// drain, the liveness check, and the store wipe. The plain
    /// [`quiesce`](CoordinatorLifecycle::quiesce)'s own RAII guard would
    /// lower it on return, leaving a window a direct pass could slip into
    /// before Clear re-raised it. Must fail against a variant that delegates
    /// to `quiesce` (whose guard clears the shared flag on drop).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_under_held_gate_keeps_caller_gate_raised() {
        let lifecycle = make_lifecycle();

        // Caller (the Clear flow) raises and holds the gate before draining.
        let hold = lifecycle.hold_quiescing_gate();
        assert!(
            lifecycle.quiescing.load(Ordering::Acquire),
            "caller's hold raised the gate"
        );

        // Drain under the held gate (no loop registered → NotRunning); the
        // gate must remain raised across the call.
        let status = lifecycle.quiesce_under_held_gate().await;
        assert_eq!(status, WorkerStatus::NotRunning);
        assert!(
            lifecycle.quiescing.load(Ordering::Acquire),
            "gate stays raised across the drain — no lapse for a direct pass"
        );

        // A direct pass attempting to begin during Clear (gate held) is
        // refused: it bails after the CAS on the raised gate.
        assert!(
            lifecycle.begin_pass().is_none(),
            "the continuously-held gate holds off a new direct pass"
        );

        // Once Clear's own guard drops, the gate reopens for later work.
        drop(hold);
        assert!(
            !lifecycle.quiescing.load(Ordering::Acquire),
            "gate reopens once the caller's hold guard drops"
        );
    }
}
