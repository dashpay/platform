//! Multi-wallet manager with SPV coordination.

pub mod accessors;
mod coordinator_lifecycle;
pub mod identity_sync;
mod load;
pub mod platform_address_sync;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
mod wallet_lifecycle;

use std::sync::Arc;

#[cfg(any(test, feature = "shielded"))]
use dash_async::WorkerStatus;
use dash_async::{ShutdownReport, ShutdownWeight, ThreadRegistry, WorkerConfig};
use tokio::sync::{Notify, RwLock};

use key_wallet_manager::WalletManager;

use crate::changeset::{wallet_event_adapter_loop, PlatformWalletPersistence};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::asset_lock::LockNotifyHandler;
use crate::wallet::core::BalanceUpdateHandler;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Identity of a background worker on the manager's shared
/// [`ThreadRegistry`]. The three periodic sync coordinators run as
/// OS-thread workers (their SDK futures are `!Send`); the wallet-event
/// adapter runs as a tokio task. Drained in ascending weight order on
/// [`shutdown`](PlatformWalletManager::shutdown): the coordinators
/// (weight 0) first, then the event adapter (weight 10) they store into.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum WalletWorker {
    /// Platform-address (BLAST) balance sync loop.
    PlatformAddressSync,
    /// Per-identity token-state sync loop.
    IdentitySync,
    /// Shielded (Orchard) note sync loop.
    ShieldedSync,
    /// Wallet-event adapter task (sinks coordinator stores).
    EventAdapter,
}

/// Teardown weight of the periodic sync coordinators — drained first.
pub(crate) const COORDINATOR_WEIGHT: ShutdownWeight = ShutdownWeight(0);
/// Teardown weight of the wallet-event adapter — drained after the
/// coordinators that feed it.
pub(crate) const EVENT_ADAPTER_WEIGHT: ShutdownWeight = ShutdownWeight(10);

/// Multi-wallet coordinator with SPV sync and event handling.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// [`PlatformEventHandler`]s by reference (no cloning).
pub struct PlatformWalletManager<P: PlatformWalletPersistence + 'static> {
    pub(super) sdk: Arc<dash_sdk::Sdk>,
    pub(super) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Map of registered wallets. Held in an `Arc` so the
    /// `BalanceUpdateHandler` can hold a clone and look up wallets to
    /// update their lock-free balance atomics from event-handler
    /// context, without touching the SPV-contended `wallet_manager`
    /// lock.
    pub(super) wallets: Arc<RwLock<std::collections::BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    /// Notified on InstantLock / ChainLock events for `AssetLockManager` waiters.
    pub(super) lock_notify: Arc<Notify>,
    pub(super) spv_manager: Arc<SpvRuntime>,
    /// Periodic platform-address (BLAST) balance sync coordinator.
    /// Not auto-started — call `start` after wallets are registered.
    pub(super) platform_address_sync_manager: Arc<PlatformAddressSyncManager>,
    /// Periodic per-identity token state sync coordinator. Refreshes
    /// the per-(identity, token) balance cache on every registered
    /// wallet. Not auto-started — call `start` after wallets are
    /// registered. See [`IdentitySyncManager`].
    pub(super) identity_sync_manager: Arc<IdentitySyncManager<P>>,
    /// Periodic shielded (Orchard) note sync coordinator (spends are
    /// detected during the note scan, no separate nullifier pass).
    /// Iterates every wallet that has been bound via
    /// [`PlatformWallet::bind_shielded`](crate::wallet::PlatformWallet::bind_shielded);
    /// unbound wallets are skipped silently. Not auto-started — call
    /// `start` after wallets are registered.
    #[cfg(feature = "shielded")]
    pub(super) shielded_sync_manager: Arc<ShieldedSyncManager>,
    /// Network-scoped shielded coordinator. `None` until
    /// `configure_shielded` opens the per-network SQLite tree;
    /// once `Some`, every wallet's `bind_shielded` reuses the
    /// same `Arc<RwLock<FileBackedShieldedStore>>` (held inside
    /// the coordinator) so there's exactly one SQLite handle per
    /// network manager. Phase 2 will move the sync loop here
    /// from the per-wallet `ShieldedSyncManager` iteration; Phase
    /// 4 deletes `ShieldedWallet` outright and the coordinator
    /// owns the spend surface too.
    #[cfg(feature = "shielded")]
    pub(super) shielded_coordinator:
        Arc<RwLock<Option<Arc<crate::wallet::shielded::NetworkShieldedCoordinator>>>>,
    /// Shared `PlatformEventManager` — held on the manager so
    /// `configure_shielded` can install a per-chunk progress handler
    /// onto the freshly-created `NetworkShieldedCoordinator` that
    /// forwards into `on_shielded_sync_progress`. Sub-managers
    /// (`SpvRuntime`, `PlatformAddressSyncManager`, etc.) hold their
    /// own clones already, so `configure_shielded` is the only reader of
    /// this retained handle — hence it is `shielded`-gated.
    #[cfg(feature = "shielded")]
    pub(super) event_manager: Arc<PlatformEventManager>,
    pub(super) persister: Arc<P>,
    /// Shared worker-lifecycle engine. Owns every background worker's
    /// cancellation token + join handle, the restart reap-or-park, and the
    /// orphan list. The coordinators hold a clone and register their loops
    /// on it; the event adapter runs here as a tokio task. [`shutdown`]
    /// drains it in weight order and joins every worker before returning.
    pub(super) registry: Arc<ThreadRegistry<WalletWorker>>,
}

/// Maximum time (seconds) the teardown paths — `shutdown()` and
/// `clear_shielded` — wait for one coordinator's quiesce+join to
/// complete. (The FFI `shielded_sync_stop` bridge is cancel-only and
/// does not consume this budget.)
///
/// This is a backstop, not the primary stop mechanism. `quiesce()`
/// cancels the loop, which aborts any in-flight pass at its `.await`
/// point (see each coordinator's `start()` select), so the `is_syncing`
/// drain clears promptly and the join normally lands far inside this
/// window. The deadline fires only if a pass's *drop* itself wedges
/// (e.g. a blocking destructor); on timeout the coordinator slot reports
/// [`WorkerStatus::Timeout`] rather than hanging forever.
pub const SHUTDOWN_JOIN_TIMEOUT_SECS: u64 = 30;

/// Grace period (seconds) [`PlatformWalletManager::shutdown`] spends
/// polling any orphans parked in the shared [`ThreadRegistry`] before
/// declaring a survivor [`Detached`](WorkerStatus::Detached).
///
/// Unlike a live coordinator — whose `quiesce()` may legitimately spend
/// seconds draining an in-flight pass, hence the 30 s
/// [`SHUTDOWN_JOIN_TIMEOUT_SECS`] — an orphan is a thread an earlier reap
/// already had to detach *because it was wedged past its 1 s backstop*.
/// A healthy detached thread finishes within milliseconds of the
/// cancellation it long ago received (so `is_finished()` is usually true
/// on the first poll and the join is instant); one still alive after this
/// grace is wedged in a non-yielding `Drop` and will not finish however
/// long we wait. A short grace therefore separates "finishing" from
/// "wedged" without stretching teardown, and reporting `Detached` is the
/// conservative, UAF-safe outcome (the host delays freeing its context).
pub(crate) const SHUTDOWN_ORPHAN_GRACE_SECS: u64 = 1;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Create a new PlatformWalletManager.
    ///
    /// `app_handler` receives all SPV and platform events by reference.
    /// Internally, a `LockNotifyHandler` is also registered to wake
    /// `AssetLockManager` async waiters on lock events.
    pub fn new(
        sdk: Arc<dash_sdk::Sdk>,
        persister: Arc<P>,
        app_handler: Arc<dyn PlatformEventHandler>,
    ) -> Self {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(sdk.network)));
        let wallets = Arc::new(RwLock::new(std::collections::BTreeMap::new()));
        let lock_notify = Arc::new(Notify::new());

        // Shared worker-lifecycle engine. The 1 s reap backstop (separate
        // from the 30 s managed-join budget) is the grace a wedged prior
        // thread gets before it is reported `Detached`.
        let registry = ThreadRegistry::with_reap_backstop(std::time::Duration::from_secs(
            SHUTDOWN_ORPHAN_GRACE_SECS,
        ));

        // Register the wallet-event adapter as a tokio task on the
        // registry. It sinks the coordinators' stores, so it drains AFTER
        // them (weight 10 vs the coordinators' 0).
        let adapter_wallet_manager = Arc::clone(&wallet_manager);
        let adapter_persister = Arc::clone(&persister);
        registry.start_task(
            WalletWorker::EventAdapter,
            WorkerConfig {
                weight: EVENT_ADAPTER_WEIGHT,
                join_budget: std::time::Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS),
                drain: None,
            },
            move |cancel| {
                wallet_event_adapter_loop(adapter_wallet_manager, adapter_persister, cancel)
            },
        );

        // Build handler list: app handler + internal handlers.
        // BalanceUpdateHandler holds a clone of the wallets map (a
        // separate lock from wallet_manager) so it can look up
        // PlatformWallets and write to their lock-free balance
        // atomics from broadcast-handler context without contending
        // with SPV's write lock.
        let lock_handler = Arc::new(LockNotifyHandler::new(Arc::clone(&lock_notify)));
        let balance_handler = Arc::new(BalanceUpdateHandler::new(Arc::clone(&wallets)));
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            app_handler,
            lock_handler,
            balance_handler,
        ]));

        let spv = Arc::new(SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::clone(&event_manager),
        ));
        let platform_address_sync = Arc::new(PlatformAddressSyncManager::new(
            Arc::clone(&wallets),
            Arc::clone(&event_manager),
            Arc::clone(&registry),
        ));
        let identity_sync = Arc::new(IdentitySyncManager::new(
            Arc::clone(&sdk),
            Arc::clone(&persister),
            Arc::clone(&registry),
        ));
        #[cfg(feature = "shielded")]
        let shielded_coordinator: Arc<
            RwLock<Option<Arc<crate::wallet::shielded::NetworkShieldedCoordinator>>>,
        > = Arc::new(RwLock::new(None));
        #[cfg(feature = "shielded")]
        let shielded_sync = Arc::new(ShieldedSyncManager::new(
            Arc::clone(&event_manager),
            Arc::clone(&shielded_coordinator),
            Arc::clone(&registry),
        ));
        Self {
            sdk,
            wallet_manager,
            wallets,
            lock_notify,
            spv_manager: spv,
            platform_address_sync_manager: platform_address_sync,
            identity_sync_manager: identity_sync,
            #[cfg(feature = "shielded")]
            shielded_sync_manager: shielded_sync,
            #[cfg(feature = "shielded")]
            shielded_coordinator,
            #[cfg(feature = "shielded")]
            event_manager,
            persister,
            registry,
        }
    }

    /// Configure network-scoped shielded support. Opens the
    /// per-network commitment-tree SQLite file at `db_path` and
    /// installs a [`NetworkShieldedCoordinator`] that every
    /// subsequent `PlatformWallet::bind_shielded` will share.
    ///
    /// Must be called before any wallet's `bind_shielded` —
    /// per-wallet bind looks up the coordinator from the manager
    /// and errors out if it hasn't been configured.
    ///
    /// Subsequent calls with the **same** `db_path` are a no-op
    /// (configuration is idempotent at the path level). A second
    /// call with a **different** path returns
    /// `ShieldedNotConfigured` — the SQLite handle is opened
    /// once per manager and can't be repointed at a different
    /// file mid-flight. (Design-doc choice (c): the path is a
    /// manager-level concern, not per-wallet.)
    #[cfg(feature = "shielded")]
    pub async fn configure_shielded(
        &self,
        db_path: impl AsRef<std::path::Path>,
    ) -> Result<(), crate::error::PlatformWalletError> {
        use crate::wallet::shielded::{FileBackedShieldedStore, NetworkShieldedCoordinator};
        let db_path: std::path::PathBuf = db_path.as_ref().to_path_buf();

        let mut slot = self.shielded_coordinator.write().await;
        if let Some(existing) = slot.as_ref() {
            if existing.db_path() == db_path.as_path() {
                return Ok(());
            }
            return Err(crate::error::PlatformWalletError::ShieldedStoreError(
                format!(
                    "shielded already configured at {} — refusing to repoint at {}",
                    existing.db_path().display(),
                    db_path.display(),
                ),
            ));
        }

        // The store opens (and creates if missing) the SQLite
        // file synchronously. 100 = shardtree's max retained
        // checkpoints; matches the prior per-wallet default at
        // `PlatformWallet::bind_shielded`.
        let store = FileBackedShieldedStore::open_path(&db_path, 100)
            .map_err(|e| crate::error::PlatformWalletError::ShieldedStoreError(e.to_string()))?;

        let coordinator = Arc::new(NetworkShieldedCoordinator::new(
            Arc::clone(&self.sdk),
            self.sdk.network,
            db_path,
            store,
        ));
        // Bridge sync-internal chunk progress (~every 2048 notes)
        // into the public `PlatformEventHandler::on_shielded_sync_progress`
        // event so UI clients can render a live counter / progress
        // bar during long cold syncs. Cheap closure — just forwards
        // two u64s to the event manager.
        let event_manager_for_progress = Arc::clone(&self.event_manager);
        coordinator.install_progress_handler(Some(Arc::new(
            move |cumulative_scanned: u64, block_height: u64| {
                event_manager_for_progress
                    .on_shielded_sync_progress(cumulative_scanned, block_height);
            },
        )));
        // Bridge sync-internal tree-commit progress (once per
        // committed batch) into the public
        // `PlatformEventHandler::on_shielded_tree_progress` event — the
        // second "checked / committed-to-tree" signal, distinct from
        // the "downloaded" counter above. `leaves_committed` is the
        // cumulative tree leaf count; `total_target` is the on-chain
        // MMR total (0 ⇒ indeterminate). Lets UI clients render a dual
        // ProgressView ("downloaded" vs "checked") during cold syncs.
        let event_manager_for_tree_progress = Arc::clone(&self.event_manager);
        coordinator.install_tree_progress_handler(Some(Arc::new(
            move |leaves_committed: u64, total_target: u64| {
                event_manager_for_tree_progress
                    .on_shielded_tree_progress(leaves_committed, total_target);
            },
        )));
        *slot = Some(coordinator);
        Ok(())
    }

    /// Snapshot of the currently-installed shielded coordinator,
    /// or `None` if `configure_shielded` hasn't run yet on this
    /// manager. Cloned `Arc` so callers can hold the coordinator
    /// past the read-lock guard's drop.
    #[cfg(feature = "shielded")]
    pub async fn shielded_coordinator(
        &self,
    ) -> Option<Arc<crate::wallet::shielded::NetworkShieldedCoordinator>> {
        self.shielded_coordinator
            .read()
            .await
            .as_ref()
            .map(Arc::clone)
    }

    /// Tear down shielded sync state for a Clear / wipe flow.
    ///
    /// Single library entry point so the FFI stays a one-call bridge:
    /// first **quiesce** the sync manager (cancel the loop *and* drain
    /// any in-flight pass, including its persister-callback fan-out, so
    /// nothing can re-persist notes after this returns), then **clear**
    /// the network coordinator's per-subwallet registries. Idempotent —
    /// the coordinator step is a no-op when shielded support was never
    /// configured. The per-network commitment-tree SQLite file stays on
    /// disk but its contents are reset to empty so the next bind cold-
    /// resyncs from index 0.
    ///
    /// Returns an error — and leaves the store untouched — in two cases, so
    /// the host knows **not** to commit its own persistence wipe:
    /// - the in-flight sync pass did not drain cleanly (timed out on the join
    ///   backstop, or its loop ended non-cleanly) →
    ///   [`crate::error::PlatformWalletError::ShieldedShutdownIncomplete`]; or
    /// - the coordinator's store reset itself fails.
    ///
    /// **Concurrency**: a concurrent direct `sync_now`/`sync_wallet` and a
    /// full `shielded_sync_start` are both held off for the whole clear. The
    /// quiescing gate is raised *continuously* (from before the drain, across
    /// the liveness check, through the wipe), so a direct pass observes it and
    /// bails with no lapse; the registry's per-key clearing latch no-ops a
    /// racing `shielded_sync_start` over the same span, so a fresh loop cannot
    /// land after the liveness check and re-persist into the store being wiped.
    #[cfg(feature = "shielded")]
    pub async fn clear_shielded(&self) -> Result<(), crate::error::PlatformWalletError> {
        self.clear_shielded_inner(std::time::Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS))
            .await
    }

    /// [`clear_shielded`](Self::clear_shielded) with an explicit drain
    /// deadline. Split out so tests can exercise the timeout path without
    /// waiting the full production budget.
    #[cfg(feature = "shielded")]
    async fn clear_shielded_inner(
        &self,
        drain_timeout: std::time::Duration,
    ) -> Result<(), crate::error::PlatformWalletError> {
        // Latch the shielded key on the registry for the WHOLE clear: a
        // concurrent `shielded_sync_start()` is no-op'd by the registry
        // until this clear completes, so a fresh worker cannot land
        // after our liveness check and re-persist into the store we are
        // about to wipe. Held in addition to the quiescing gate below
        // (direct passes go through `begin_pass`, which the gate covers;
        // a full start lands on the registry, which the latch covers).
        let _clearing_latch = self.registry.hold_clearing(WalletWorker::ShieldedSync);

        // Raise and HOLD the shielded quiescing gate for the WHOLE clear,
        // BEFORE quiescing — so the "no new pass" barrier never lapses
        // between the drain, the liveness check, and the store wipe: a direct
        // `sync_now`/`sync_wallet` landing anywhere in here observes the gate
        // and bails instead of re-persisting into the store we are about to
        // clear. `quiesce_under_held_gate` deliberately does NOT touch the
        // gate (a single `AtomicFlagGuard` always clears the flag on drop, so
        // letting `quiesce` manage it and re-raising afterwards would leave a
        // window). The guard lowers the gate on return (every path).
        let _clearing_gate = self.shielded_sync_manager.hold_quiescing_gate();

        // Cancel the loop and drain any in-flight pass (incl. its persister
        // fan-out). Bound the drain so a heavy direct pass cannot hang the
        // host's Clear: on timeout the clear reports `Timeout` and aborts
        // BEFORE the wipe, leaving the store intact.
        let status = match tokio::time::timeout(
            drain_timeout,
            self.shielded_sync_manager.quiesce_under_held_gate(),
        )
        .await
        {
            Ok(status) => status,
            Err(_elapsed) => WorkerStatus::Timeout,
        };

        // Only commit the store wipe once the in-flight pass has fully
        // drained. A partial/timed-out drain could let a surviving pass
        // write into a store we just cleared, desyncing the host's own
        // wipe from a repopulated tree.
        if !status.is_clean() {
            return Err(crate::error::PlatformWalletError::ShieldedShutdownIncomplete { status });
        }

        // Also refuse if a prior-generation shielded thread is still parked
        // alive: it holds an `Arc` to the persister/store and could re-persist
        // notes into the store we are about to wipe. The check is shielded-
        // scoped (shares the `shielded_worker_alive` gate), so the other
        // coordinators / the always-on event adapter running normally do not
        // block Clear.
        if self.shielded_worker_alive() {
            return Err(
                crate::error::PlatformWalletError::ShieldedShutdownIncomplete {
                    status: WorkerStatus::Detached,
                },
            );
        }
        if let Some(coord) = self.shielded_coordinator().await {
            coord.clear().await?;
        }
        Ok(())
    }

    /// Stop all background workers and wait for them to exit.
    ///
    /// Delegates to the shared [`ThreadRegistry::shutdown`], which drains
    /// in ascending weight order: the periodic coordinators (weight 0)
    /// first — concurrently, since they share no lock — then the
    /// wallet-event adapter (weight 10) that sinks their stores, then any
    /// parked orphans. Each worker's drain raises its `quiescing` gate,
    /// cancels the loop, and **joins** its OS thread / task, so when this
    /// returns every `!Send` loop has fully exited. Idempotent.
    ///
    /// Ordering matters: cancel-only `stop()` would let a pass already
    /// inside `sync_now` keep running and call `persister.store(...)` /
    /// fire a host completion callback after the FFI's `destroy` returned
    /// and the host freed the persister / event-handler context — a
    /// use-after-free. Quiescing the coordinators (weight 0) before the
    /// event adapter (weight 10) closes that window: no further store can
    /// start before its sink is torn down.
    ///
    /// A host that drops the tokio runtime right after `shutdown()`
    /// (one-shot / headless / stdio) is therefore safe — no coordinator
    /// can still be polling `tokio::time` on a shutting-down runtime. The
    /// returned [`ShutdownReport`] reports per-worker how each ended.
    ///
    /// **Precondition: must be called from a multi-thread Tokio runtime.**
    /// Each coordinator's OS thread drives its loop via
    /// [`Handle::block_on`](tokio::runtime::Handle::block_on) and needs the
    /// runtime's timer/IO driver; a `current_thread` runtime can only
    /// service one `block_on` at a time, so the join would deadlock.
    /// [`ThreadRegistry::shutdown`] asserts this in both debug and release.
    ///
    /// Each worker's join is bounded by its own
    /// [`SHUTDOWN_JOIN_TIMEOUT_SECS`] budget; on timeout its handle is
    /// re-parked and the slot reports
    /// [`WorkerStatus::Timeout`] rather than hanging forever
    /// (F1: a dropped/timed-out join can never detach a live
    /// thread). The clear-on-panic half rides on unwinding, so it holds
    /// under `panic = "unwind"`; under the iOS `panic = "abort"` profiles a
    /// pass panic aborts the process outright.
    pub async fn shutdown(&self) -> ShutdownReport<WalletWorker> {
        self.registry.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AO};
    use std::time::Duration;

    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletChangeSet};
    use crate::manager::platform_address_sync::PlatformAddressSyncSummary;

    /// No-op persister — the lifecycle tests below never exercise the
    /// real persistence pipeline, they just need a handle that satisfies
    /// the manager's `P` bound.
    struct NoopPersister;

    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// No-op event handler standing in for the host's FFI handler.
    struct NoopHandler;
    impl dash_spv::EventHandler for NoopHandler {}
    impl PlatformEventHandler for NoopHandler {}

    /// Build a manager over a mock SDK + no-op persister/handler. Cheap:
    /// `new` wires the sub-managers and spawns the event adapter but
    /// starts no coordinator threads.
    fn make_manager() -> PlatformWalletManager<NoopPersister> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopHandler);
        PlatformWalletManager::new(sdk, persister, handler)
    }

    /// Build a manager that fires a slow (300 ms std::thread::sleep) callback
    /// on `on_platform_address_sync_completed`. Used by the in-flight-pass
    /// drain test.
    fn make_manager_with_slow_handler(
        started: Arc<AtomicBool>,
        completed: Arc<AtomicBool>,
    ) -> PlatformWalletManager<NoopPersister> {
        struct SlowHandler {
            started: Arc<AtomicBool>,
            completed: Arc<AtomicBool>,
        }
        impl dash_spv::EventHandler for SlowHandler {}
        impl PlatformEventHandler for SlowHandler {
            fn on_platform_address_sync_completed(&self, _: &PlatformAddressSyncSummary) {
                self.started.store(true, AO::Release);
                std::thread::sleep(Duration::from_millis(300));
                self.completed.store(true, AO::Release);
            }
        }

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(SlowHandler { started, completed });
        PlatformWalletManager::new(sdk, persister, handler)
    }

    /// Start every periodic coordinator's background OS-thread loop.
    fn start_coordinators<P: PlatformWalletPersistence + 'static>(m: &PlatformWalletManager<P>) {
        Arc::clone(&m.platform_address_sync_manager).start();
        Arc::clone(&m.identity_sync_manager).start();
        #[cfg(feature = "shielded")]
        Arc::clone(&m.shielded_sync_manager).start();
    }

    /// Happy path: `shutdown()` joins every started worker and reports
    /// `Ok`; it completes within a bounded time (no `spawn_blocking`
    /// starvation/deadlock); a second `shutdown()` finds nothing left to
    /// join (`NotRunning`) — idempotent.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_joins_all_workers_reports_ok_and_is_idempotent() {
        let manager = make_manager();
        start_coordinators(&manager);
        // Let the loops enter `block_on` so we exercise the live-loop
        // join path (a thread cancelled before its first poll joins too).
        tokio::time::sleep(Duration::from_millis(50)).await;

        let status = tokio::time::timeout(Duration::from_secs(10), manager.shutdown())
            .await
            .expect("shutdown join must complete within bound");
        assert_eq!(
            status.per_worker.get(&WalletWorker::PlatformAddressSync),
            Some(&WorkerStatus::Ok)
        );
        assert_eq!(
            status.per_worker.get(&WalletWorker::IdentitySync),
            Some(&WorkerStatus::Ok)
        );
        #[cfg(feature = "shielded")]
        assert_eq!(
            status.per_worker.get(&WalletWorker::ShieldedSync),
            Some(&WorkerStatus::Ok)
        );
        #[cfg(not(feature = "shielded"))]
        assert!(status.per_worker.get(&WalletWorker::ShieldedSync).is_none());
        assert_eq!(
            status.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::Ok)
        );
        assert!(status.all_clean());

        // Handles consumed by the first join → nothing left to join.
        let again = manager.shutdown().await;
        assert_eq!(
            again.per_worker.get(&WalletWorker::PlatformAddressSync),
            Some(&WorkerStatus::NotRunning)
        );
        assert_eq!(
            again.per_worker.get(&WalletWorker::IdentitySync),
            Some(&WorkerStatus::NotRunning)
        );
        assert_eq!(
            again.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::NotRunning)
        );
        assert!(again.all_clean());
    }

    /// Never-started coordinators are absent from the report — the registry
    /// only keys workers it actually registered, so a coordinator whose
    /// `start()` never ran has no `per_worker` entry. The event adapter is
    /// spawned in `new`, so it still joins `Ok`. Absent workers do not affect
    /// `all_clean()`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_without_starting_reports_not_running() {
        let manager = make_manager();

        let status = manager.shutdown().await;
        assert!(!status
            .per_worker
            .contains_key(&WalletWorker::PlatformAddressSync));
        assert!(!status.per_worker.contains_key(&WalletWorker::IdentitySync));
        assert!(!status.per_worker.contains_key(&WalletWorker::ShieldedSync));
        assert_eq!(
            status.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::Ok)
        );
        assert!(status.all_clean());
    }

    /// `Stopped` and `Timeout` are NOT clean; `Ok` and `NotRunning` ARE.
    /// Unit-tests the `is_clean` predicate directly so we don't need to
    /// trigger a real timeout (30s) in a deterministic test.
    #[test]
    fn worker_status_clean_predicate() {
        assert!(WorkerStatus::Ok.is_clean());
        assert!(WorkerStatus::NotRunning.is_clean());

        assert!(!WorkerStatus::Stopped(None).is_clean());
        assert!(!WorkerStatus::Stopped(Some("cancelled".into())).is_clean());
        assert!(!WorkerStatus::Panicked("boom".into()).is_clean());
        assert!(!WorkerStatus::Timeout.is_clean());
        assert!(!WorkerStatus::Error("infra".into()).is_clean());
        // A detached-but-still-live coordinator thread is non-clean: the
        // host must not free its callback context yet.
        assert!(!WorkerStatus::Detached.is_clean());
    }

    /// `all_clean()` on the `ShutdownReport` that `shutdown()` returns is
    /// false whenever any per-worker slot is non-clean, a reaped orphan was
    /// non-clean, or an orphan survived the reap (`detached > 0`).
    #[test]
    fn shutdown_report_all_clean() {
        use std::collections::BTreeMap;

        let clean = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([
                (WalletWorker::PlatformAddressSync, WorkerStatus::Ok),
                (WalletWorker::IdentitySync, WorkerStatus::NotRunning),
                (WalletWorker::EventAdapter, WorkerStatus::Ok),
            ]),
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };
        assert!(clean.all_clean());

        let with_timeout = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([
                (WalletWorker::PlatformAddressSync, WorkerStatus::Timeout),
                (WalletWorker::IdentitySync, WorkerStatus::Ok),
                (WalletWorker::EventAdapter, WorkerStatus::Ok),
            ]),
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };
        assert!(!with_timeout.all_clean());

        let with_stopped = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([
                (WalletWorker::PlatformAddressSync, WorkerStatus::Ok),
                (WalletWorker::IdentitySync, WorkerStatus::Ok),
                (
                    WalletWorker::ShieldedSync,
                    WorkerStatus::Stopped(Some("aborted".into())),
                ),
                (WalletWorker::EventAdapter, WorkerStatus::Ok),
            ]),
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };
        assert!(!with_stopped.all_clean());

        // A still-live detached orphan alone makes the aggregate
        // non-clean — the slot the rest of the teardown can't see.
        let with_detached = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([
                (WalletWorker::PlatformAddressSync, WorkerStatus::Ok),
                (WalletWorker::IdentitySync, WorkerStatus::Ok),
                (WalletWorker::EventAdapter, WorkerStatus::Ok),
            ]),
            detached: 1,
            orphan_status: WorkerStatus::Detached,
        };
        assert!(!with_detached.all_clean());
    }

    /// A panicked reaped orphan (orphan finished within the reap grace, so
    /// `detached == 0` but `orphan_status` is `Panicked`) must still make the
    /// report non-clean. Without `orphan_status` folding into `all_clean()`,
    /// a survivor count of zero would silently pass the panic.
    #[test]
    fn panicked_orphan_status_makes_report_non_clean() {
        use std::collections::BTreeMap;

        // No survivors, but a non-clean reaped-orphan status. An empty
        // `per_worker` means no worker slot is itself non-clean, so the
        // verdict rides entirely on `orphan_status`.
        let report = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::new(),
            detached: 0,
            orphan_status: WorkerStatus::Panicked("orphan panic during reap".into()),
        };

        assert!(
            !report.all_clean(),
            "all_clean() must reflect a panicked reaped-orphan status",
        );
    }

    /// Complement: a clean reaped-orphan status (`Ok`) leaves the report
    /// clean. Guards against over-triggering the orphan-status fold.
    #[test]
    fn clean_orphan_status_keeps_report_clean() {
        use std::collections::BTreeMap;

        let mut per_worker = BTreeMap::new();
        per_worker.insert(WalletWorker::PlatformAddressSync, WorkerStatus::Ok);
        per_worker.insert(WalletWorker::IdentitySync, WorkerStatus::Ok);
        #[cfg(feature = "shielded")]
        per_worker.insert(WalletWorker::ShieldedSync, WorkerStatus::Ok);
        per_worker.insert(WalletWorker::EventAdapter, WorkerStatus::Ok);

        let report = ShutdownReport::<WalletWorker> {
            per_worker,
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };

        assert!(report.all_clean());
    }

    /// `shutdown()` must wait for an in-flight sync pass to drain before
    /// joining the coordinator thread.
    ///
    /// A slow `on_platform_address_sync_completed` callback (300 ms)
    /// keeps `is_syncing=true` while it runs. We call `shutdown()` while
    /// the callback is in-flight and assert that `shutdown()` blocks
    /// until the callback completes, then returns `Ok`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_waits_for_in_flight_pass_to_drain() {
        let handler_started = Arc::new(AtomicBool::new(false));
        let handler_completed = Arc::new(AtomicBool::new(false));
        let manager = make_manager_with_slow_handler(
            Arc::clone(&handler_started),
            Arc::clone(&handler_completed),
        );

        // Start the address-sync coordinator; first pass fires immediately.
        Arc::clone(&manager.platform_address_sync_manager).start();

        // Wait until the slow completion callback is running
        // (`is_syncing` stays true for its 300 ms duration).
        tokio::time::timeout(Duration::from_secs(5), async {
            while !handler_started.load(AO::Acquire) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("handler did not start within 5s");

        // Shutdown must drain the in-flight pass before joining.
        let status = tokio::time::timeout(Duration::from_secs(5), manager.shutdown())
            .await
            .expect("shutdown must complete within 5 s");

        assert_eq!(
            status.per_worker.get(&WalletWorker::PlatformAddressSync),
            Some(&WorkerStatus::Ok),
            "coordinator must join cleanly after drain"
        );
        assert!(
            handler_completed.load(AO::Acquire),
            "shutdown must not return before the in-flight pass completes"
        );
    }

    /// Race regression — start coordinators with a long sleep interval so
    /// they spend nearly all their time in a live `tokio::time::sleep`,
    /// then `shutdown()` and drop the runtime.
    ///
    /// With the thread join in `shutdown()` every coordinator has fully
    /// exited its `block_on` before `drop(runtime)` — no race possible.
    /// Loop 10 times to give any latent race a reliable window: WITHOUT
    /// the join, the coordinator's `select!` wakeup (via tokio) would
    /// race the runtime teardown and reliably trigger the
    /// "Tokio … being shutdown" panic across the 10 iterations.
    ///
    /// Uses `std::panic::catch_unwind` around `drop(runtime)` rather than
    /// a process-global panic hook; the hook would be live for seconds and
    /// could swallow diagnostics from other concurrently-running tests.
    #[test]
    fn shutdown_then_drop_runtime_does_not_panic() {
        static SHUTDOWN_PANICS: AtomicUsize = AtomicUsize::new(0);

        for _ in 0..10 {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("build runtime");

            let status = runtime.block_on(async {
                let manager = make_manager();
                // Long interval: coordinator spends ~10 s in a live
                // tokio::time::sleep, maximising the race window for a
                // join-less runtime drop.
                manager
                    .platform_address_sync_manager
                    .set_interval(Duration::from_secs(10));
                manager
                    .identity_sync_manager
                    .set_interval(Duration::from_secs(10));
                #[cfg(feature = "shielded")]
                manager
                    .shielded_sync_manager
                    .set_interval(Duration::from_secs(10));
                start_coordinators(&manager);
                // Wait for coordinators to finish their first (instant)
                // pass and enter the long sleep.
                tokio::time::sleep(Duration::from_millis(100)).await;
                // shutdown() joins each thread before returning; without
                // the join this drop would race the select!/block_on exit.
                manager.shutdown().await
            });

            // Wrap the runtime drop in catch_unwind to intercept the specific
            // "A Tokio 1.x context ... being shutdown" panic without installing
            // a process-wide hook that would suppress diagnostics from other
            // concurrently running tests.
            let drop_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drop(runtime);
            }));
            if let Err(payload) = drop_result {
                let msg = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("");
                if msg.contains("being shutdown") {
                    SHUTDOWN_PANICS.fetch_add(1, AO::SeqCst);
                } else {
                    // Unexpected panic — propagate so the test fails loudly.
                    std::panic::resume_unwind(payload);
                }
            }

            // Brief settle — any stray thread activity surfaces here.
            std::thread::sleep(Duration::from_millis(50));

            assert_eq!(
                status.per_worker.get(&WalletWorker::PlatformAddressSync),
                Some(&WorkerStatus::Ok)
            );
            assert_eq!(
                status.per_worker.get(&WalletWorker::IdentitySync),
                Some(&WorkerStatus::Ok)
            );
            assert!(status.all_clean(), "workers did not wind down: {status:?}");
        }

        assert_eq!(
            SHUTDOWN_PANICS.load(AO::SeqCst),
            0,
            "dropping the runtime after shutdown raced a coordinator thread \
             ({} panics across 10 iterations)",
            SHUTDOWN_PANICS.load(AO::SeqCst)
        );
    }

    /// Spawn a thread that parks until `release` is signalled (or the
    /// sender drops), standing in for a coordinator thread wedged in a
    /// non-yielding `Drop` that ignores the cancellation it received.
    fn spawn_wedged_thread() -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            // Block here regardless of any cancellation, exactly like a
            // Drop that never yields, until the test releases us.
            let _ = release_rx.recv();
        });
        (release_tx, handle)
    }

    /// Headline regression: a coordinator thread detached past the reap
    /// backstop and parked in the orphans list makes a subsequent
    /// `shutdown()` report the result as **non-clean** — so the FFI
    /// `destroy` returns `ErrorShutdownIncomplete` and the host delays
    /// freeing the callback context the still-live thread may touch.
    ///
    /// Non-vacuous: if the registry dropped the orphan at reap instead of
    /// parking it, `detached` would be `0` and `all_clean()` would be
    /// `true`, failing both assertions.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_reports_detached_orphan_as_non_clean() {
        let manager = make_manager();

        // Stand in for the genuine-wedge outcome: an earlier tight
        // stop()->start() reap had to detach a still-live coordinator thread
        // past its backstop, so the registry parked it as an orphan.
        let (release_tx, wedged) = spawn_wedged_thread();
        manager
            .registry
            .park_orphan_for_test(WalletWorker::ShieldedSync, wedged);

        let status = tokio::time::timeout(Duration::from_secs(10), manager.shutdown())
            .await
            .expect("shutdown must complete within bound");

        assert_eq!(
            status.detached, 1,
            "a still-live detached orphan must surface in the survivor count"
        );
        assert!(
            !status.all_clean(),
            "all_clean() must be false while a detached coordinator thread is \
             still alive: {status:?}"
        );

        // Cleanup: shutdown() re-parked the survivor; release it and reap so
        // no live thread leaks past the test.
        release_tx.send(()).unwrap();
        let _ = manager.registry.reap_orphans(Duration::from_secs(5)).await;
    }

    /// TC-002 (F2): `clear_shielded` must refuse while a prior-generation
    /// shielded thread is parked alive — even though the current shielded
    /// quiesce is clean and the other coordinators / the always-on event
    /// adapter are legitimately running. Releasing + reaping the orphan
    /// lets a retry succeed.
    ///
    /// Non-vacuous: a gate checking only `!status.is_clean()` would let the
    /// clean `NotRunning` quiesce pass the guard and wipe the store under the
    /// live orphan — `clear_shielded` would return `Ok`.
    #[cfg(feature = "shielded")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn clear_shielded_refuses_while_shielded_orphan_alive() {
        let manager = make_manager();

        // Park a wedged thread under the ShieldedSync key: a prior-
        // generation shielded thread an earlier reap could not join.
        let (release_tx, wedged) = spawn_wedged_thread();
        manager
            .registry
            .park_orphan_for_test(WalletWorker::ShieldedSync, wedged);

        assert!(manager.registry.any_alive_for(WalletWorker::ShieldedSync));
        assert!(!manager.shielded_sync_manager.is_running());

        // Refuses: the live shielded orphan could re-persist into the store
        // the wipe is about to clear.
        let err = manager
            .clear_shielded()
            .await
            .expect_err("clear_shielded must refuse while a shielded orphan is alive");
        assert!(
            matches!(
                err,
                crate::error::PlatformWalletError::ShieldedShutdownIncomplete { .. }
            ),
            "expected ShieldedShutdownIncomplete, got {err:?}"
        );

        // Release + reap the orphan; the shielded-scoped gate now clears and
        // a retry succeeds (no shielded store configured → clear is a no-op).
        release_tx.send(()).unwrap();
        let _ = manager.registry.reap_orphans(Duration::from_secs(5)).await;
        assert!(!manager.registry.any_alive_for(WalletWorker::ShieldedSync));
        manager
            .clear_shielded()
            .await
            .expect("clear_shielded must succeed once the orphan is reaped");
    }

    /// While `clear_shielded` is mid-flight, a concurrent
    /// `shielded_sync().start()` must be no-op'd by the registry's per-key
    /// clearing latch — otherwise a fresh worker can land between the
    /// clear's liveness check and the store wipe and re-persist into the
    /// store about to be cleared. The test holds the latch directly (mirrors
    /// what `clear_shielded_inner` does) and verifies a start under the latch
    /// leaves `is_running()==false`.
    ///
    /// Non-vacuous: without the latch the start would register a worker
    /// on the registry (`is_running()==true`) regardless of any in-
    /// flight clear.
    #[cfg(feature = "shielded")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shielded_sync_start_no_ops_while_clearing_latch_held() {
        let manager = make_manager();

        // Hold the same latch `clear_shielded_inner` raises.
        let _clearing_latch = manager.registry.hold_clearing(WalletWorker::ShieldedSync);

        // A concurrent caller drives `start()` on the shielded coordinator.
        manager.shielded_sync_manager.clone().start();
        assert!(
            !manager.shielded_sync_manager.is_running(),
            "registry must refuse the shielded start while the clearing latch is held"
        );

        // Sibling coordinator (different key) is unaffected.
        manager.platform_address_sync_manager.clone().start();
        assert!(
            manager.platform_address_sync_manager.is_running(),
            "the latch is per-key; sibling coordinators must still start"
        );

        // Drop the latch; a fresh start now succeeds.
        drop(_clearing_latch);
        manager.shielded_sync_manager.clone().start();
        assert!(
            manager.shielded_sync_manager.is_running(),
            "latch release must let the shielded coordinator start again"
        );

        // Cleanup.
        let _ = manager.shutdown().await;
    }

    /// SEC-002 continuity under concurrent (re)start during clear: when
    /// `clear_shielded` holds both the per-key clearing latch AND the
    /// quiescing gate continuously, a racing `shielded_sync().start()`
    /// must NOT lower the gate even though `start_thread` is refused.
    /// Without the `is_clearing` short-circuit in
    /// `CoordinatorLifecycle::spawn_periodic_loop`, the start path's
    /// `reopen_quiescing_gate` call lowers the gate before the latch
    /// check refuses the spawn — opening a window where a direct
    /// `sync_now`/`sync_wallet` via `begin_pass` would observe the
    /// lowered gate and slip past the clear's "no new pass" barrier.
    ///
    /// Non-vacuous: against the pre-fix `spawn_periodic_loop` (reopen
    /// → start_thread, latch checked only in start_thread), the gate
    /// would be `false` after the refused start.
    #[cfg(feature = "shielded")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shielded_start_during_clear_preserves_quiescing_gate() {
        use std::sync::atomic::Ordering;

        let manager = make_manager();

        // Acquire BOTH guards `clear_shielded_inner` holds: the per-key
        // registry latch + the wallet-side quiescing gate.
        let _clearing_latch = manager.registry.hold_clearing(WalletWorker::ShieldedSync);
        let _clearing_gate = manager.shielded_sync_manager.hold_quiescing_gate();
        assert!(
            manager
                .shielded_sync_manager
                .quiescing_load_for_test(Ordering::SeqCst),
            "precondition: gate raised by the held clearing guard"
        );

        // A racing (re)start arrives.
        manager.shielded_sync_manager.clone().start();

        // start_thread is refused by the latch, as before...
        assert!(
            !manager.shielded_sync_manager.is_running(),
            "registry latch must refuse the start"
        );
        // ...and the gate stays UP. Pre-fix this would be `false` because
        // `spawn_periodic_loop` lowered the gate before the latch check.
        assert!(
            manager
                .shielded_sync_manager
                .quiescing_load_for_test(Ordering::SeqCst),
            "gate must remain raised: refused start must not lower clear_shielded's continuously-held gate",
        );

        // Cleanup — drop the guards in reverse order before shutdown.
        drop(_clearing_gate);
        drop(_clearing_latch);
        let _ = manager.shutdown().await;
    }

    /// SEC-001: `clear_shielded` must BOUND its in-flight-pass drain so a
    /// heavy direct `sync_now`/`sync_wallet` that won't drain in time cannot
    /// hang the host's Clear. On the drain deadline the clear reports
    /// `ShieldedShutdownIncomplete` and aborts BEFORE the store wipe, leaving
    /// the store intact.
    ///
    /// Non-vacuous: against an unbounded drain the held pass keeps
    /// `is_syncing` set forever and `clear_shielded_inner` never returns — the
    /// test's outer timeout fires and the `expect` below panics.
    #[cfg(feature = "shielded")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn clear_shielded_aborts_without_wiping_when_drain_times_out() {
        let manager = Arc::new(make_manager());

        // A direct sync pass already in flight (holds `is_syncing`); it never
        // drains within the clear's drain budget.
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let ssm = Arc::clone(&manager.shielded_sync_manager);
        let pass_task = tokio::spawn(async move {
            let _pass = ssm
                .begin_pass_for_test()
                .expect("direct pass enters the slot");
            ready_tx.send(()).expect("signal in-flight");
            release_rx.await.expect("await release");
            // `_pass` drops here → is_syncing = false
        });

        ready_rx.await.expect("pass reached in-flight");
        assert!(manager.shielded_sync_manager.is_syncing());

        // Clear with a short drain budget: the held pass can't drain in time,
        // so the clear must return ShieldedShutdownIncomplete — bounded, never
        // hanging — and never reach the wipe.
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            manager.clear_shielded_inner(Duration::from_millis(100)),
        )
        .await
        .expect("clear must return within its bounded drain, never hang");
        assert!(
            matches!(
                result,
                Err(crate::error::PlatformWalletError::ShieldedShutdownIncomplete { .. })
            ),
            "bounded drain timeout must surface as ShieldedShutdownIncomplete, got {result:?}"
        );

        // Release the held pass and join.
        release_tx.send(()).expect("release the pass");
        pass_task.await.expect("pass task joined");
    }

    /// The [`ShutdownReport`] that `shutdown()` returns reads per-worker by
    /// key, surfaces a surviving orphan through `detached`, and folds all
    /// three signals (per-worker status, `detached`, `orphan_status`) into
    /// `all_clean()`.
    #[test]
    fn shutdown_report_per_worker_shape_and_all_clean() {
        use std::collections::BTreeMap;

        // All Ok, no orphans.
        let report = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([
                (WalletWorker::PlatformAddressSync, WorkerStatus::Ok),
                (WalletWorker::IdentitySync, WorkerStatus::Ok),
                (WalletWorker::ShieldedSync, WorkerStatus::Ok),
                (WalletWorker::EventAdapter, WorkerStatus::Ok),
            ]),
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };
        assert_eq!(
            report.per_worker.get(&WalletWorker::PlatformAddressSync),
            Some(&WorkerStatus::Ok)
        );
        assert_eq!(
            report.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::Ok)
        );
        assert!(report.all_clean());

        // A surviving orphan -> detached > 0 -> non-clean; an absent worker
        // has no `per_worker` entry.
        let report = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::new(),
            detached: 1,
            orphan_status: WorkerStatus::Detached,
        };
        assert_eq!(report.detached, 1);
        assert!(!report
            .per_worker
            .contains_key(&WalletWorker::PlatformAddressSync));
        assert!(!report.all_clean());

        // A per-worker Timeout is read back by key and is non-clean.
        let report = ShutdownReport::<WalletWorker> {
            per_worker: BTreeMap::from([(WalletWorker::IdentitySync, WorkerStatus::Timeout)]),
            detached: 0,
            orphan_status: WorkerStatus::Ok,
        };
        assert_eq!(
            report.per_worker.get(&WalletWorker::IdentitySync),
            Some(&WorkerStatus::Timeout)
        );
        assert!(!report.all_clean());
    }
}
