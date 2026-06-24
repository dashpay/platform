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
    /// exit path (the registry's drain hook raised it; reopening is safe
    /// because the loop has been cancelled, so no new pass starts).
    pub(crate) async fn quiesce(&self) -> CoordinatorThreadStatus {
        let _quiescing_gate = AtomicFlagGuard::new(&self.quiescing);
        self.registry.quiesce(self.worker).await.into()
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
