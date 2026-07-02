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

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dash_async::{
    AtomicFlagGuard, RefcountedFlagGuard, ThreadRegistry, WorkerConfig, WorkerStatus,
};

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
    /// Refcounted "no new pass" gate: raised iff count > 0. Refcount
    /// semantics let the two teardown paths — the public `quiesce()`'s
    /// local guard and the Clear flow's `hold_quiescing_gate` — compose
    /// without one path's Drop lowering the other path's barrier.
    quiescing: Arc<AtomicUsize>,
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
            quiescing: Arc::new(AtomicUsize::new(0)),
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
    /// teardown weight and the shared join budget. No registry-side drain
    /// hook is installed: the lifecycle's own [`quiesce`](Self::quiesce)
    /// holds the gate via a [`RefcountedFlagGuard`] across the whole drain
    /// (cancellation-safe — its `Drop` runs on every exit path including a
    /// future dropped at an outer `await`), and
    /// [`quiesce_under_held_gate`](Self::quiesce_under_held_gate) requires
    /// the caller to already hold one. The gate lives in the lifecycle, not
    /// a registry-side hook, because a hook's `fetch_add` runs inside
    /// `registry.quiesce` while its `fetch_sub` outlives the same `await`: a
    /// `tokio::time::timeout` wrap (e.g. `clear_shielded_inner`) firing
    /// mid-drain would then leak the refcount and wedge the coordinator.
    pub(crate) fn worker_config(&self) -> WorkerConfig {
        WorkerConfig {
            weight: COORDINATOR_WEIGHT,
            join_budget: Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS),
            drain: None,
        }
    }

    /// Spawn the standard coordinator loop on the registry: take the
    /// worker config and drive `biased; cancel-first` select! of `pass()`
    /// followed by an `interval()` sleep — both arms break on cancellation.
    /// The loop runs inside `Handle::block_on` on a fresh OS thread (so
    /// `pass` can yield `!Send` SDK futures); cancellation drops an in-
    /// flight `pass()` at its next `.await`.
    ///
    /// With refcount semantics on `quiescing`, there is nothing to "reopen"
    /// here: properly-dropped guards (`quiesce`'s local, the Clear flow's
    /// `hold_quiescing_gate`) leave the count at 0, and the gate is only
    /// observed "raised" while a teardown holder is in scope.
    ///
    /// Each coordinator's `start` calls this with a thunk that captures its
    /// `Arc<Self>` and invokes its own `sync_now`.
    pub(crate) fn spawn_periodic_loop<F, Fut, I>(&self, pass: F, interval: I)
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()>,
        I: Fn() -> Duration + Send + 'static,
    {
        // Defense-in-depth: bail when our key is in the registry's clearing
        // set. `start_thread` below would refuse the start regardless, and
        // refcount semantics make the `quiescing` gate race-free in either
        // case — the bail just avoids the wasted setup work.
        if self.registry.is_clearing(self.worker) {
            return;
        }
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

    /// Cancel-only stop: signal the loop and return immediately.
    pub(crate) fn stop(&self) {
        self.registry.cancel(self.worker);
    }

    /// Cancel the loop, drain any in-flight pass, and join the worker,
    /// returning its terminal status. Raises the `quiescing` gate via a
    /// [`RefcountedFlagGuard`] that decrements on every exit path, leaving
    /// the gate's overall state determined by whatever other holders
    /// (e.g. the Clear flow's `hold_quiescing_gate`) are still in scope.
    ///
    /// The gate is raised **here**, by the lifecycle itself — there is no
    /// registry-side drain hook. The guard's `Drop` is cancellation-safe
    /// (it runs on every exit path including a future dropped at an outer
    /// `await`), so the gate is reliably released exactly once per call
    /// and the "no new pass" barrier holds regardless of whether a
    /// background-loop slot is registered. Gate-before-cancel is
    /// preserved: the raise is synchronous and lives above the
    /// `registry.quiesce` await that issues any cancel.
    ///
    /// Refcount semantics make the `is_clearing` short-circuit and the
    /// gate-guard install race-free: if a Clear lands in the ns-window
    /// between the check and the guard, its `hold_quiescing_gate` raise
    /// composes with ours — both refs keep the count > 0 until each holder's
    /// own guard drops, so the local guard's decrement on our return can
    /// never lower the gate past the Clear's contribution. The short-circuit
    /// is defense-in-depth (skip a redundant drain while a Clear is already
    /// doing one), not a correctness anchor.
    ///
    /// "Direct pass" here means the gated entry points that take the
    /// `is_syncing` slot via [`begin_pass`](Self::begin_pass): every
    /// coordinator's `sync_now`, plus the shielded coordinator's
    /// `sync_wallet`. The platform-address coordinator's `sync_wallet` is
    /// intentionally **ungated** (it never touches `is_syncing`; callers
    /// that need exclusion gate themselves), so the gate/drain barrier does
    /// not apply to it.
    pub(crate) async fn quiesce(&self) -> WorkerStatus {
        // Defense-in-depth: if a concurrent `clear_shielded` is mid-flight
        // for our key, it is already draining + joining the worker under
        // its own gate, so a second drain here would be redundant. The
        // refcount semantics below make the gate handling race-free
        // regardless of whether this check fires, but skipping the work
        // saves a wasted drain cycle.
        if self.registry.is_clearing(self.worker) {
            return WorkerStatus::NotRunning;
        }
        // Gate up first (instant) and held until the guard drops on return.
        // SeqCst on the refcount: store-half of the `quiescing`<->
        // `is_syncing` handshake (see `begin_pass`).
        let _quiescing_gate = RefcountedFlagGuard::raise(&self.quiescing);
        self.cancel_join_and_drain().await
    }

    /// Like [`quiesce`](Self::quiesce) but for a caller that has **already**
    /// raised the `quiescing` gate (via [`hold_quiescing_gate`](Self::hold_quiescing_gate))
    /// and is holding the registry's per-key clearing latch around the same
    /// teardown: this skips both the [`quiesce`](Self::quiesce) call's
    /// `is_clearing` short-circuit (which would otherwise bail without
    /// running the drain) and its own gate raise (the caller's ref already
    /// keeps the gate up). The shielded Clear flow uses this to keep the
    /// "no new pass" barrier raised *continuously* across the drain, the
    /// orphan-liveness check, and the store wipe — with no lapse for a
    /// direct `sync_now`/`sync_wallet` to slip through and re-persist into
    /// the store being cleared. Gate-before-cancel still holds: the caller
    /// raised the gate before this runs.
    #[cfg(any(test, feature = "shielded"))]
    pub(crate) async fn quiesce_under_held_gate(&self) -> WorkerStatus {
        debug_assert!(
            self.quiescing.load(Ordering::Acquire) > 0,
            "quiesce_under_held_gate requires the caller to already hold the quiescing gate"
        );
        self.cancel_join_and_drain().await
    }

    /// Cancel + bounded-join the background loop (if any), then drain a
    /// direct in-flight pass on a clean status. Assumes the `quiescing`
    /// gate is **already raised** by the caller: [`quiesce`](Self::quiesce)
    /// installs its own local [`RefcountedFlagGuard`] in scope across the
    /// whole `await`, and [`quiesce_under_held_gate`](Self::quiesce_under_held_gate)
    /// requires the caller to hold one. Either guard's `Drop` runs on
    /// every exit path — including a future dropped at an outer `await`
    /// (e.g. a `tokio::time::timeout` firing mid-drain) — so the gate is
    /// released exactly once per teardown, no matter how the call unwinds.
    ///
    /// A wedged loop pass surfaces from `registry.quiesce` as a non-clean
    /// `Timeout` rather than hanging — its orphaned thread is tracked by
    /// the registry for teardown, so the drain below must not wait on it.
    /// On a clean status the only possible `is_syncing` holder is a direct
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
    /// the `quiescing` gate already raised (refcount > 0, so no new pass
    /// can start) and after the background loop has been cancel-joined (so
    /// the only possible holder is a direct, non-cancellable pass running
    /// to completion). Mirrors the registry's 5ms poll cadence. Unbounded
    /// by design — the caller bounds the whole teardown (the FFI `stop` /
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
    /// guard drops. Under refcount semantics, multiple holders compose
    /// safely: the gate stays raised while ANY guard is held, and each
    /// guard's Drop only releases its own contribution. This lets a
    /// multi-step teardown (e.g. the shielded Clear flow) keep new direct
    /// passes off across a check-then-wipe even while a concurrent public
    /// `quiesce()` lands inside the window — neither party's Drop can lower
    /// the other's barrier. Used by the shielded Clear flow and by
    /// `PlatformWalletManager::shutdown`, which holds every coordinator's
    /// gate across the whole teardown (including the registry's event-adapter
    /// join); the coordinator pass-gate tests also exercise it.
    pub(crate) fn hold_quiescing_gate(&self) -> RefcountedFlagGuard<'_> {
        // SeqCst on the refcount: store-half of the `quiescing`<->
        // `is_syncing` handshake (see `begin_pass`). The Clear flow raises
        // the gate through here, so this raise must be self-fencing just
        // like `quiesce`'s.
        RefcountedFlagGuard::raise(&self.quiescing)
    }

    /// Test-only read of the `quiescing` gate as a boolean ("is the gate
    /// raised?"). Returns `true` iff the refcount is > 0. Used by
    /// regression tests that assert the gate stays raised across a refused
    /// (re)start or a racing public `quiesce()`.
    #[cfg(test)]
    pub(crate) fn quiescing_load_for_test(&self, ordering: Ordering) -> bool {
        self.quiescing.load(ordering) > 0
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
        // SeqCst — load-half of the handshake described above. The gate is
        // observed "raised" iff the refcount is > 0 (any teardown holder in
        // scope).
        if self.quiescing.load(Ordering::SeqCst) > 0 {
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
            lifecycle.quiescing.load(Ordering::Acquire) > 0,
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
            lifecycle.quiescing.load(Ordering::Acquire) > 0,
            "caller's hold raised the gate"
        );

        // Drain under the held gate (no loop registered → NotRunning); the
        // gate must remain raised across the call.
        let status = lifecycle.quiesce_under_held_gate().await;
        assert_eq!(status, WorkerStatus::NotRunning);
        assert!(
            lifecycle.quiescing.load(Ordering::Acquire) > 0,
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
        assert_eq!(
            lifecycle.quiescing.load(Ordering::Acquire),
            0,
            "gate reopens once the caller's hold guard drops"
        );
    }

    /// Racing public `quiesce()` during Clear: the Clear flow holds
    /// `hold_quiescing_gate` continuously across its drain + wipe; a
    /// concurrent public `quiesce()` that arrives in the window AFTER the
    /// Clear has raised the gate but BEFORE the Clear has acquired the
    /// registry's per-key `hold_clearing` latch must NOT lower the gate
    /// on its return — the Clear's continuously-held barrier is what keeps
    /// a direct `sync_now`/`sync_wallet` from re-persisting into the store
    /// being wiped. We stage exactly this ordering by holding the
    /// `hold_quiescing_gate` BUT NOT the clearing latch — so `quiesce`'s
    /// `is_clearing` short-circuit does NOT fire and the local guard IS
    /// installed.
    ///
    /// Proves the refcount gate keeps the barrier raised through the racing
    /// `quiesce()`: its guard's decrement composes with Clear's still-held
    /// ref rather than forcing the flag to `false`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_during_held_gate_does_not_lower_clears_barrier() {
        let lifecycle = make_lifecycle();

        // Clear has raised the gate (but has NOT yet taken the clearing
        // latch — the racy ordering this test exercises).
        let clearing_gate = lifecycle.hold_quiescing_gate();
        assert!(
            lifecycle.quiescing_load_for_test(Ordering::Acquire),
            "precondition: Clear's hold_quiescing_gate raised the gate"
        );

        // Racing public quiesce arrives. With no clearing latch held,
        // `is_clearing` returns false and the local refcount guard IS
        // installed (count goes 1 → 2 → back to 1 on its drop).
        let status = lifecycle.quiesce().await;
        assert_eq!(status, WorkerStatus::NotRunning);

        // The gate is STILL up: Clear's continuously-held ref keeps it so,
        // so a direct `sync_now`/`sync_wallet` cannot slip past `begin_pass`
        // into the store being wiped.
        assert!(
            lifecycle.quiescing_load_for_test(Ordering::Acquire),
            "gate must stay raised across the racing public quiesce — \
             Clear's continuously-held hold_quiescing_gate must not be \
             lowered by another path's Drop"
        );
        // And `begin_pass` still bails, so no direct pass can claim the slot
        // while the gate is held.
        assert!(
            lifecycle.begin_pass().is_none(),
            "the still-raised gate keeps a direct pass from claiming the slot"
        );

        drop(clearing_gate);
        assert!(
            !lifecycle.quiescing_load_for_test(Ordering::Acquire),
            "gate reopens once Clear's own guard drops"
        );
    }

    /// Refcount composition: two `hold_quiescing_gate` guards raise the
    /// gate to count 2; dropping one leaves count 1 (still "raised");
    /// dropping the second returns to 0. The wallet-side invariant that
    /// makes the public `quiesce()` ↔ `hold_quiescing_gate` race-free —
    /// neither party's Drop can lower the other's barrier — rides on
    /// exactly this property.
    #[test]
    fn refcount_guards_compose() {
        let registry = ThreadRegistry::<WalletWorker>::new();
        let lifecycle = CoordinatorLifecycle::new(registry, WalletWorker::IdentitySync, 60);

        let g1 = lifecycle.hold_quiescing_gate();
        assert_eq!(lifecycle.quiescing.load(Ordering::Acquire), 1);
        let g2 = lifecycle.hold_quiescing_gate();
        assert_eq!(lifecycle.quiescing.load(Ordering::Acquire), 2);

        drop(g1);
        assert_eq!(
            lifecycle.quiescing.load(Ordering::Acquire),
            1,
            "dropping one holder must not lower the gate past the surviving holder's contribution"
        );
        assert!(
            lifecycle.quiescing_load_for_test(Ordering::Acquire),
            "gate is still observed raised while any holder remains in scope"
        );
        assert!(
            lifecycle.begin_pass().is_none(),
            "the still-raised gate keeps `begin_pass` from claiming the slot"
        );

        drop(g2);
        assert_eq!(lifecycle.quiescing.load(Ordering::Acquire), 0);
        assert!(
            !lifecycle.quiescing_load_for_test(Ordering::Acquire),
            "gate reopens once the last holder drops"
        );
    }

    /// Regression: a timeout / forced cancellation that drops the
    /// `lifecycle.quiesce()` future mid-drain must NOT leak the `quiescing`
    /// refcount. A registry-side `drain_hook` would be unsound here — its
    /// `fetch_add` runs inside `registry.quiesce` while the matching
    /// `fetch_sub` outlives the inner `.await` in `cancel_join_and_drain`,
    /// so the outer drop strands the hook's contribution: count stuck `+1`,
    /// gate stuck raised, every future `begin_pass` bailing, the coordinator
    /// silently dead. The gate therefore lives only in the lifecycle's own
    /// `RefcountedFlagGuard`, whose `Drop` runs on the cancellation unwind
    /// and returns the count to `0`. `clear_shielded_inner` wraps
    /// `quiesce_under_held_gate()` in a `tokio::time::timeout`, so this is
    /// the live cancellation path the test drives.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_cancellation_during_drain_does_not_leak_quiescing_refcount() {
        let lifecycle = make_lifecycle();

        // Register a background loop whose `pass` blocks the OS thread with
        // `std::thread::sleep` once in flight, so the registry's join poll
        // sits in the mid-drain window we drop the future on top of.
        // `pass_entered` lets the test wait until the loop body is genuinely
        // running: drive teardown any earlier and the biased `select!` could
        // observe cancel first, so `pass` never runs and the test is vacuous.
        let pass_entered = Arc::new(AtomicBool::new(false));
        let pass_release = Arc::new(AtomicBool::new(false));
        let pass_entered_c = Arc::clone(&pass_entered);
        let pass_release_c = Arc::clone(&pass_release);
        lifecycle.spawn_periodic_loop(
            move || {
                let entered = Arc::clone(&pass_entered_c);
                let release = Arc::clone(&pass_release_c);
                async move {
                    entered.store(true, Ordering::Release);
                    while !release.load(Ordering::Acquire) {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            },
            || Duration::from_secs(60),
        );

        // Wait for the OS thread to be IN `pass` (past `select!`'s biased
        // cancel arm), so a cancel signal from the upcoming `quiesce`
        // cannot win the select before the loop body runs.
        let start = std::time::Instant::now();
        while !pass_entered.load(Ordering::Acquire) {
            if start.elapsed() > Duration::from_secs(2) {
                pass_release.store(true, Ordering::Release);
                panic!("background loop never entered `pass`");
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Drive `quiesce` on a separate task so we can abort its future
        // mid-drain — simulating the `clear_shielded_inner`
        // `tokio::time::timeout` firing inside `registry.quiesce`.
        let lc_q = Arc::clone(&lifecycle);
        let quiesce_task = tokio::spawn(async move { lc_q.quiesce().await });

        // Wait for the lifecycle's local guard to raise the gate, then let
        // the registry settle into its join wait so the abort below lands
        // mid-drain. 50ms is ample: the join poll runs on a 5ms cadence.
        let start = std::time::Instant::now();
        while !lifecycle.quiescing_load_for_test(Ordering::Acquire) {
            if start.elapsed() > Duration::from_secs(2) {
                pass_release.store(true, Ordering::Release);
                panic!("quiesce never raised the gate");
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel mid-drain — the OS thread is still parked in
        // `std::thread::sleep`, the registry is still polling the join wait.
        // Dropping the `quiesce` future here must still run the local
        // guard's `Drop`, returning the refcount to 0.
        quiesce_task.abort();
        let _ = quiesce_task.await;

        // Assert the refcount returns to 0 within a generous window. With
        // the leak, it stays at `1` forever and this loop times out.
        let start = std::time::Instant::now();
        while lifecycle.quiescing_load_for_test(Ordering::Acquire) {
            if start.elapsed() > Duration::from_millis(500) {
                let leaked = lifecycle.quiescing.load(Ordering::Acquire);
                pass_release.store(true, Ordering::Release);
                panic!(
                    "quiescing refcount leaked past cancellation: count = {leaked} \
                     (expected 0 once the dropped future's guards unwound)"
                );
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Cleanup: release the pass so the OS thread observes cancel and
        // exits before the test runtime is dropped.
        pass_release.store(true, Ordering::Release);
    }
}
