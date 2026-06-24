//! Multi-wallet manager with SPV coordination.

pub mod accessors;
pub mod identity_sync;
mod load;
pub mod platform_address_sync;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
mod wallet_lifecycle;

use std::sync::Arc;

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

/// How one background coordinator thread terminated.
///
/// The three periodic coordinators run their loops on dedicated OS
/// threads (the SDK futures are `!Send`, so they ride
/// [`Handle::block_on`](tokio::runtime::Handle::block_on) rather than
/// `tokio::spawn`). [`PlatformWalletManager::shutdown`] joins each
/// thread and reports how it ended so a host can tell a clean wind-down
/// from a panicked loop instead of silently dropping the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorThreadStatus {
    /// The loop exited and its thread/task joined cleanly.
    Ok,
    /// The thread/task exited for a non-panic reason that is not a clean
    /// return — e.g. a tokio task was cancelled or aborted. Carries a
    /// reason string when one is available.
    Stopped(Option<String>),
    /// The thread/task panicked; carries the best-effort panic message.
    Panicked(String),
    /// The join did not complete within [`SHUTDOWN_JOIN_TIMEOUT_SECS`].
    Timeout,
    /// No thread/task was running to join — never started, or already
    /// joined by a previous `shutdown()`.
    NotRunning,
    /// Infrastructural join failure that is neither a timeout nor a
    /// panic — e.g. the `spawn_blocking` task itself failed because
    /// the runtime was torn down before the join could run (unreachable
    /// in normal operation).
    Error(String),
    /// At least one coordinator OS thread that an earlier tight
    /// `stop()`→`start()` reap had to detach past its 1 s wedge-backstop
    /// was still alive at the shutdown deadline.
    ///
    /// Such a thread was parked in the shared [`ThreadRegistry`]'s orphan
    /// list (not silently dropped) precisely so this case is visible.
    /// A still-live detached thread keeps an `Arc` to the host event
    /// handler and may fire one final callback, so the host must NOT
    /// free the callback context yet — this status keeps
    /// [`is_clean`](Self::is_clean) `false` so the FFI `destroy` returns
    /// `ErrorShutdownIncomplete` instead of `ok()`.
    Detached,
}

impl CoordinatorThreadStatus {
    /// `true` only for a fully clean outcome: joined normally (`Ok`) or
    /// never ran (`NotRunning`). `Stopped`, `Panicked`, `Timeout`,
    /// `Error`, and `Detached` are all considered non-clean.
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Ok | Self::NotRunning)
    }
}

/// Relocate a registry [`WorkerStatus`](dash_async::WorkerStatus) into the
/// FFI-stable `CoordinatorThreadStatus`. The variant sets and payloads
/// correspond 1:1, so the body is an exhaustive by-name `From` match that
/// the compiler keeps total. The two enums intentionally keep their own
/// declaration order and carry no `#[repr]`, so this is a match, never a
/// layout-compatible cast — the FFI `destroy` / shielded-stop adapters keep
/// reading the same logical shape.
impl From<dash_async::WorkerStatus> for CoordinatorThreadStatus {
    fn from(status: dash_async::WorkerStatus) -> Self {
        use dash_async::WorkerStatus as W;
        match status {
            W::Ok => Self::Ok,
            W::Stopped(reason) => Self::Stopped(reason),
            W::Panicked(msg) => Self::Panicked(msg),
            W::Timeout => Self::Timeout,
            W::Detached => Self::Detached,
            W::NotRunning => Self::NotRunning,
            W::Error(msg) => Self::Error(msg),
        }
    }
}

/// Per-thread terminal status of every background worker, returned by
/// [`PlatformWalletManager::shutdown`].
///
/// A host that drops its tokio runtime right after `shutdown()`
/// (one-shot / headless / stdio) reads this to confirm each `!Send`
/// coordinator loop fully wound down on its OS thread *before* the
/// runtime goes away — closing the race where a still-polling loop hits
/// `tokio::time` on a shutting-down runtime and panics with
/// `A Tokio 1.x context was found, but it is being shutdown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinatorExitStatus {
    /// Platform-address (BLAST) balance sync loop.
    pub platform_address_sync: CoordinatorThreadStatus,
    /// Per-identity token-state sync loop.
    pub identity_sync: CoordinatorThreadStatus,
    /// Shielded (Orchard) note sync loop. `None` in builds without the
    /// `shielded` feature (the coordinator does not exist).
    pub shielded_sync: Option<CoordinatorThreadStatus>,
    /// Wallet-event adapter (a `tokio` task, not an OS thread).
    pub event_adapter: CoordinatorThreadStatus,
    /// Aggregate status of any coordinator OS threads that an earlier
    /// tight `stop()`→`start()` reap had to detach past its 1 s
    /// wedge-backstop and park in the shared [`ThreadRegistry`]'s orphan
    /// list.
    ///
    /// [`Ok`](CoordinatorThreadStatus::Ok) when none were detached (or
    /// every detached thread has since joined cleanly);
    /// [`Detached`](CoordinatorThreadStatus::Detached) when at least one
    /// is still alive at the shutdown deadline. This is what keeps
    /// [`all_clean`](Self::all_clean) honest for the wedge case the rest
    /// of the teardown can't see — without it a detached-but-still-live
    /// thread would let the host free a callback context the thread may
    /// still touch (a residual use-after-free).
    pub detached_threads: CoordinatorThreadStatus,
}

impl CoordinatorExitStatus {
    /// `true` only when every worker — including any parked
    /// [`detached_threads`](Self::detached_threads) — is
    /// [`Ok`](CoordinatorThreadStatus::Ok) or
    /// [`NotRunning`](CoordinatorThreadStatus::NotRunning); any
    /// `Stopped`, `Panicked`, `Timeout`, `Error`, or `Detached` slot
    /// makes it `false`.
    pub fn all_clean(&self) -> bool {
        self.platform_address_sync.is_clean()
            && self.identity_sync.is_clean()
            && self.shielded_sync.as_ref().is_none_or(|s| s.is_clean())
            && self.event_adapter.is_clean()
            && self.detached_threads.is_clean()
    }

    /// Build the FFI-stable exit status from the registry's weight-ordered
    /// [`ShutdownReport`]. A worker absent from the report never ran, so it
    /// maps to [`NotRunning`](CoordinatorThreadStatus::NotRunning); a
    /// non-zero orphan-survivor count surfaces as
    /// [`Detached`](CoordinatorThreadStatus::Detached), keeping
    /// [`all_clean`](Self::all_clean) honest for a still-live wedged thread.
    pub(crate) fn from_report(report: ShutdownReport<WalletWorker>) -> Self {
        let worker = |key: WalletWorker| -> CoordinatorThreadStatus {
            report
                .per_worker
                .get(&key)
                .cloned()
                .map(CoordinatorThreadStatus::from)
                .unwrap_or(CoordinatorThreadStatus::NotRunning)
        };
        Self {
            platform_address_sync: worker(WalletWorker::PlatformAddressSync),
            identity_sync: worker(WalletWorker::IdentitySync),
            #[cfg(feature = "shielded")]
            shielded_sync: Some(worker(WalletWorker::ShieldedSync)),
            #[cfg(not(feature = "shielded"))]
            shielded_sync: None,
            event_adapter: worker(WalletWorker::EventAdapter),
            detached_threads: if report.detached > 0 {
                CoordinatorThreadStatus::Detached
            } else {
                CoordinatorThreadStatus::Ok
            },
        }
    }
}

/// Maximum time (seconds) the teardown paths — `shutdown()`,
/// `clear_shielded`, and the FFI shielded-stop bridge — wait for one
/// coordinator's quiesce+join to complete.
///
/// This is a backstop, not the primary stop mechanism. `quiesce()`
/// cancels the loop, which aborts any in-flight pass at its `.await`
/// point (see each coordinator's `start()` select), so the `is_syncing`
/// drain clears promptly and the join normally lands far inside this
/// window. The deadline fires only if a pass's *drop* itself wedges
/// (e.g. a blocking destructor); on timeout the coordinator slot reports
/// [`CoordinatorThreadStatus::Timeout`] rather than hanging forever.
pub const SHUTDOWN_JOIN_TIMEOUT_SECS: u64 = 30;

/// Grace period (seconds) [`PlatformWalletManager::shutdown`] spends
/// polling any orphans parked in the shared [`ThreadRegistry`] before
/// declaring a survivor [`Detached`](CoordinatorThreadStatus::Detached).
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
    /// **Host-serialization precondition**: the caller must not invoke
    /// `shielded_sync_start` for this manager concurrently with `clear`. A
    /// concurrent direct `sync_now`/`sync_wallet` is held off (the quiescing
    /// gate stays raised across the liveness check and the wipe), but a full
    /// restart re-opens that gate as it spawns a fresh loop, so a `start`
    /// racing `clear` can still re-persist into the wiped store. The wallet
    /// UI drives these from one place; that ordering is the host's contract
    /// until the registry grows a per-key clearing latch.
    #[cfg(feature = "shielded")]
    pub async fn clear_shielded(&self) -> Result<(), crate::error::PlatformWalletError> {
        // Quiesce the shielded loop: cancel it, drain any in-flight pass
        // (incl. its persister fan-out), and join its OS thread. The
        // registry bounds the join by the coordinator's own
        // `SHUTDOWN_JOIN_TIMEOUT_SECS` budget — returning `Timeout` rather
        // than hanging if a pass's drop wedges — so no outer timeout is
        // needed here.
        let status = self.shielded_sync_manager.quiesce().await;

        // Only commit the store wipe once the in-flight pass has fully
        // drained. A partial/timed-out drain could let a surviving pass
        // write into a store we just cleared, desyncing the host's own
        // wipe from a repopulated tree.
        if !status.is_clean() {
            return Err(crate::error::PlatformWalletError::ShieldedShutdownIncomplete { status });
        }
        // Hold the shielded quiescing gate raised across BOTH the liveness
        // check below and the store wipe, so the gate guarding "no new pass"
        // does not lapse between check and act: a direct `sync_now` /
        // `sync_wallet` that lands here observes the gate and bails instead
        // of writing into the store we are about to clear. The guard lowers
        // the gate on return (every path), so a later start/sync works.
        let _clearing_gate = self.shielded_sync_manager.hold_quiescing_gate();

        // [F2 FIX] Also refuse if a prior-generation shielded thread is
        // still parked alive: it holds an `Arc` to the persister/store and
        // could re-persist notes into the store we are about to wipe. The
        // check is shielded-scoped, so the other coordinators / the
        // always-on event adapter running normally do not block Clear.
        if self.registry.any_alive_for(WalletWorker::ShieldedSync) {
            return Err(
                crate::error::PlatformWalletError::ShieldedShutdownIncomplete {
                    status: CoordinatorThreadStatus::Detached,
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
    /// returned [`CoordinatorExitStatus`] reports per-worker how each ended.
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
    /// [`CoordinatorThreadStatus::Timeout`] rather than hanging forever
    /// (the F1 fix — a dropped/timed-out join can never detach a live
    /// thread). The clear-on-panic half rides on unwinding, so it holds
    /// under `panic = "unwind"`; under the iOS `panic = "abort"` profiles a
    /// pass panic aborts the process outright.
    pub async fn shutdown(&self) -> CoordinatorExitStatus {
        CoordinatorExitStatus::from_report(self.registry.shutdown().await)
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
        assert_eq!(status.platform_address_sync, CoordinatorThreadStatus::Ok);
        assert_eq!(status.identity_sync, CoordinatorThreadStatus::Ok);
        #[cfg(feature = "shielded")]
        assert_eq!(status.shielded_sync, Some(CoordinatorThreadStatus::Ok));
        #[cfg(not(feature = "shielded"))]
        assert_eq!(status.shielded_sync, None);
        assert_eq!(status.event_adapter, CoordinatorThreadStatus::Ok);
        assert!(status.all_clean());

        // Handles consumed by the first join → nothing left to join.
        let again = manager.shutdown().await;
        assert_eq!(
            again.platform_address_sync,
            CoordinatorThreadStatus::NotRunning
        );
        assert_eq!(again.identity_sync, CoordinatorThreadStatus::NotRunning);
        assert_eq!(again.event_adapter, CoordinatorThreadStatus::NotRunning);
        assert!(again.all_clean());
    }

    /// Never-started coordinators report `NotRunning` (no thread to
    /// join). The event adapter is spawned in `new`, so it still joins
    /// `Ok`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_without_starting_reports_not_running() {
        let manager = make_manager();

        let status = manager.shutdown().await;
        assert_eq!(
            status.platform_address_sync,
            CoordinatorThreadStatus::NotRunning
        );
        assert_eq!(status.identity_sync, CoordinatorThreadStatus::NotRunning);
        #[cfg(feature = "shielded")]
        assert_eq!(
            status.shielded_sync,
            Some(CoordinatorThreadStatus::NotRunning)
        );
        #[cfg(not(feature = "shielded"))]
        assert_eq!(status.shielded_sync, None);
        assert_eq!(status.event_adapter, CoordinatorThreadStatus::Ok);
        assert!(status.all_clean());
    }

    /// `Stopped` and `Timeout` are NOT clean; `Ok` and `NotRunning` ARE.
    /// Unit-tests the `is_clean` predicate directly so we don't need to
    /// trigger a real timeout (30s) in a deterministic test.
    #[test]
    fn coordinator_thread_status_clean_predicate() {
        assert!(CoordinatorThreadStatus::Ok.is_clean());
        assert!(CoordinatorThreadStatus::NotRunning.is_clean());

        assert!(!CoordinatorThreadStatus::Stopped(None).is_clean());
        assert!(!CoordinatorThreadStatus::Stopped(Some("cancelled".into())).is_clean());
        assert!(!CoordinatorThreadStatus::Panicked("boom".into()).is_clean());
        assert!(!CoordinatorThreadStatus::Timeout.is_clean());
        assert!(!CoordinatorThreadStatus::Error("infra".into()).is_clean());
        // A detached-but-still-live coordinator thread is non-clean: the
        // host must not free its callback context yet.
        assert!(!CoordinatorThreadStatus::Detached.is_clean());
    }

    /// `all_clean()` on `CoordinatorExitStatus` is false whenever any
    /// slot is non-clean.
    #[test]
    fn coordinator_exit_status_all_clean() {
        let clean = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Ok,
            identity_sync: CoordinatorThreadStatus::NotRunning,
            shielded_sync: None,
            event_adapter: CoordinatorThreadStatus::Ok,
            detached_threads: CoordinatorThreadStatus::Ok,
        };
        assert!(clean.all_clean());

        let with_timeout = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Timeout,
            identity_sync: CoordinatorThreadStatus::Ok,
            shielded_sync: None,
            event_adapter: CoordinatorThreadStatus::Ok,
            detached_threads: CoordinatorThreadStatus::Ok,
        };
        assert!(!with_timeout.all_clean());

        let with_stopped = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Ok,
            identity_sync: CoordinatorThreadStatus::Ok,
            shielded_sync: Some(CoordinatorThreadStatus::Stopped(Some("aborted".into()))),
            event_adapter: CoordinatorThreadStatus::Ok,
            detached_threads: CoordinatorThreadStatus::Ok,
        };
        assert!(!with_stopped.all_clean());

        // A still-live detached orphan alone makes the aggregate
        // non-clean — the slot the rest of the teardown can't see.
        let with_detached = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Ok,
            identity_sync: CoordinatorThreadStatus::Ok,
            shielded_sync: None,
            event_adapter: CoordinatorThreadStatus::Ok,
            detached_threads: CoordinatorThreadStatus::Detached,
        };
        assert!(!with_detached.all_clean());
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
            status.platform_address_sync,
            CoordinatorThreadStatus::Ok,
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

            assert_eq!(status.platform_address_sync, CoordinatorThreadStatus::Ok);
            assert_eq!(status.identity_sync, CoordinatorThreadStatus::Ok);
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
    /// parking it, `detached_threads` would be `Ok` and `all_clean()` would
    /// be `true`, failing both assertions.
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
            status.detached_threads,
            CoordinatorThreadStatus::Detached,
            "a still-live detached orphan must surface as Detached"
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
    /// Non-vacuous: against the pre-fix gate (only `!status.is_clean()`),
    /// the clean `NotRunning` quiesce would pass the guard and wipe the
    /// store under the live orphan — `clear_shielded` would return `Ok`.
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

    /// TC-015 (R5): `from_report` maps the registry's [`ShutdownReport`]
    /// onto the FFI-stable `CoordinatorExitStatus` with identical field /
    /// variant shape and `all_clean()` semantics. The full `WorkerStatus`
    /// -> `CoordinatorThreadStatus` variant table is exercised.
    #[test]
    fn from_report_maps_to_ffi_stable_exit_status() {
        use dash_async::WorkerStatus;
        use std::collections::BTreeMap;

        // All Ok, no orphans.
        let per = BTreeMap::from([
            (WalletWorker::PlatformAddressSync, WorkerStatus::Ok),
            (WalletWorker::IdentitySync, WorkerStatus::Ok),
            (WalletWorker::ShieldedSync, WorkerStatus::Ok),
            (WalletWorker::EventAdapter, WorkerStatus::Ok),
        ]);
        let status = CoordinatorExitStatus::from_report(ShutdownReport {
            per_worker: per,
            detached: 0,
        });
        assert_eq!(status.platform_address_sync, CoordinatorThreadStatus::Ok);
        assert_eq!(status.identity_sync, CoordinatorThreadStatus::Ok);
        #[cfg(feature = "shielded")]
        assert_eq!(status.shielded_sync, Some(CoordinatorThreadStatus::Ok));
        #[cfg(not(feature = "shielded"))]
        assert_eq!(status.shielded_sync, None);
        assert_eq!(status.event_adapter, CoordinatorThreadStatus::Ok);
        assert_eq!(status.detached_threads, CoordinatorThreadStatus::Ok);
        assert!(status.all_clean());

        // A surviving orphan -> Detached -> non-clean; absent workers ->
        // NotRunning.
        let status = CoordinatorExitStatus::from_report(ShutdownReport {
            per_worker: BTreeMap::new(),
            detached: 1,
        });
        assert_eq!(status.detached_threads, CoordinatorThreadStatus::Detached);
        assert_eq!(
            status.platform_address_sync,
            CoordinatorThreadStatus::NotRunning
        );
        assert!(!status.all_clean());

        // A per-worker Timeout propagates and is non-clean.
        let per = BTreeMap::from([(WalletWorker::IdentitySync, WorkerStatus::Timeout)]);
        let status = CoordinatorExitStatus::from_report(ShutdownReport {
            per_worker: per,
            detached: 0,
        });
        assert_eq!(status.identity_sync, CoordinatorThreadStatus::Timeout);
        assert!(!status.all_clean());

        // Full variant mapping table.
        assert_eq!(
            CoordinatorThreadStatus::from(WorkerStatus::Stopped(Some("x".into()))),
            CoordinatorThreadStatus::Stopped(Some("x".into()))
        );
        assert_eq!(
            CoordinatorThreadStatus::from(WorkerStatus::Panicked("p".into())),
            CoordinatorThreadStatus::Panicked("p".into())
        );
        assert_eq!(
            CoordinatorThreadStatus::from(WorkerStatus::Error("e".into())),
            CoordinatorThreadStatus::Error("e".into())
        );
        assert_eq!(
            CoordinatorThreadStatus::from(WorkerStatus::NotRunning),
            CoordinatorThreadStatus::NotRunning
        );
        assert_eq!(
            CoordinatorThreadStatus::from(WorkerStatus::Detached),
            CoordinatorThreadStatus::Detached
        );
    }
}
