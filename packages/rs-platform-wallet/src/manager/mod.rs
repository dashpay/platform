//! Multi-wallet manager with SPV coordination.

pub mod accessors;
pub mod identity_sync;
mod load;
pub mod platform_address_sync;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
mod wallet_lifecycle;

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use key_wallet_manager::WalletManager;

use crate::changeset::{spawn_wallet_event_adapter, PlatformWalletPersistence};
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
    /// Cancellation token + join handle for the wallet-event adapter
    /// task. Held so [`shutdown`] can stop it cleanly when the manager
    /// is torn down.
    pub(super) event_adapter_cancel: CancellationToken,
    pub(super) event_adapter_join: tokio::sync::Mutex<Option<JoinHandle<()>>>,
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
}

impl CoordinatorThreadStatus {
    /// `true` only for a fully clean outcome: joined normally (`Ok`) or
    /// never ran (`NotRunning`). `Stopped`, `Panicked`, `Timeout`, and
    /// `Error` are all considered non-clean.
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Ok | Self::NotRunning)
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
}

impl CoordinatorExitStatus {
    /// `true` only when every worker is
    /// [`Ok`](CoordinatorThreadStatus::Ok) or
    /// [`NotRunning`](CoordinatorThreadStatus::NotRunning); any
    /// `Stopped`, `Panicked`, `Timeout`, or `Error` slot makes it
    /// `false`.
    pub fn all_clean(&self) -> bool {
        self.platform_address_sync.is_clean()
            && self.identity_sync.is_clean()
            && self.shielded_sync.as_ref().is_none_or(|s| s.is_clean())
            && self.event_adapter.is_clean()
    }
}

/// Join a coordinator's background OS thread and classify how it ended.
///
/// Called from each coordinator's `quiesce()` after cancelling the
/// loop and draining any in-flight pass, so the thread is already on
/// its way out and the join is near-instant. Joining while the runtime
/// is still alive guarantees the `!Send` loop has stopped touching
/// `tokio::time` before the host drops the runtime.
///
/// **Polling approach**: we poll [`JoinHandle::is_finished`] in 5 ms
/// steps rather than wrapping `handle.join()` in
/// [`spawn_blocking`](tokio::task::spawn_blocking). The
/// `spawn_blocking` approach spawns a blocking-pool task that cannot be
/// cancelled once started — so dropping the timeout future that wraps
/// `quiesce()` would leave the blocking task alive and `handle.join()`
/// still running, defeating the timeout boundary. Polling lets the
/// executor yield at each `.await` step so `tokio::time::timeout`
/// wrapping `quiesce()` can truly interrupt this call.
///
/// **Requires a multi-thread runtime.** Each coordinator's OS thread
/// drives its loop via [`Handle::block_on`](tokio::runtime::Handle::block_on)
/// and needs the runtime's timer/IO driver; a `current_thread` runtime
/// can only service one `block_on` at a time, so joining one coordinator
/// while the others (and `shutdown()` itself) are mid-`block_on` would
/// deadlock. `shutdown()` asserts the multi-thread flavor up front.
pub(crate) async fn join_coordinator_thread(
    handle: Option<std::thread::JoinHandle<()>>,
) -> CoordinatorThreadStatus {
    let Some(handle) = handle else {
        return CoordinatorThreadStatus::NotRunning;
    };
    // Poll until the thread exits. The coordinator was already cancelled
    // (stop() fires before quiesce() calls us), so is_finished() becomes
    // true nearly immediately — typically within a single 5 ms step.
    loop {
        if handle.is_finished() {
            return match handle.join() {
                Ok(()) => CoordinatorThreadStatus::Ok,
                Err(payload) => CoordinatorThreadStatus::Panicked(panic_message(payload)),
            };
        }
        // Yield to the executor so the outer tokio::time::timeout wrapping
        // quiesce() can fire if the deadline has passed. Without this yield
        // the loop would busy-spin and block the task.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

/// Best-effort extraction of a panic message from a joined thread/task
/// payload (`&str` and `String` are the common cases).
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
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

        // Spawn the wallet-event adapter that translates upstream
        // `WalletEvent`s into `CoreChangeSet`s and forwards them to
        // the persister.
        let event_adapter_cancel = CancellationToken::new();
        let event_adapter_join = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::clone(&persister),
            event_adapter_cancel.clone(),
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
        ));
        let identity_sync = Arc::new(IdentitySyncManager::new(
            Arc::clone(&sdk),
            Arc::clone(&persister),
        ));
        #[cfg(feature = "shielded")]
        let shielded_coordinator: Arc<
            RwLock<Option<Arc<crate::wallet::shielded::NetworkShieldedCoordinator>>>,
        > = Arc::new(RwLock::new(None));
        #[cfg(feature = "shielded")]
        let shielded_sync = Arc::new(ShieldedSyncManager::new(
            Arc::clone(&event_manager),
            Arc::clone(&shielded_coordinator),
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
            event_adapter_cancel,
            event_adapter_join: tokio::sync::Mutex::new(Some(event_adapter_join)),
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
    /// Returns an error if the coordinator's store reset fails; the host
    /// must not commit its own persistence wipe in that case.
    #[cfg(feature = "shielded")]
    pub async fn clear_shielded(&self) -> Result<(), crate::error::PlatformWalletError> {
        // Bound the quiesce with the same backstop `shutdown()` uses so a
        // stalled in-flight pass can't hang Clear forever — cancellation
        // makes the drain prompt; this timeout only matters if a pass's
        // drop wedges. The terminal status isn't surfaced on the Clear
        // path (the coordinator reset below is what can fail), so the
        // timeout result is intentionally discarded.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS),
            self.shielded_sync_manager.quiesce(),
        )
        .await;
        if let Some(coord) = self.shielded_coordinator().await {
            coord.clear().await?;
        }
        Ok(())
    }

    /// Stop all background tasks and wait for them to exit.
    ///
    /// **Quiesces** the periodic coordinators
    /// (`PlatformAddressSyncManager`, `IdentitySyncManager`,
    /// `ShieldedSyncManager`) — cancelling each loop *and draining any
    /// in-flight pass to completion*, including its persister /
    /// host-callback fan-out — then drains the wallet-event adapter task.
    /// Idempotent. Call before dropping the manager when a clean
    /// shutdown is required (e.g. on app termination); a dirty drop
    /// simply leaks the tasks until the runtime exits.
    ///
    /// Ordering matters: cancel-only `stop()` would let a pass already
    /// inside `sync_now` keep running and call `persister.store(...)` /
    /// fire a host completion callback after the FFI's `destroy`
    /// returned and the host freed the persister / event-handler
    /// context — a use-after-free. So we `quiesce()` the sync managers
    /// FIRST (so no further persister store or host callback can start),
    /// and only THEN cancel + join the event adapter, which is the sink
    /// those stores feed into.
    ///
    /// After each coordinator's `quiesce()` drains its in-flight pass,
    /// this also **joins** the loop's OS thread, so when `shutdown()`
    /// returns every `!Send` loop has fully exited. A host that drops the
    /// tokio runtime right after `shutdown()` (one-shot / headless /
    /// stdio) is therefore safe — no coordinator can still be polling
    /// `tokio::time` on a shutting-down runtime. The returned
    /// [`CoordinatorExitStatus`] reports per-thread how each worker ended.
    ///
    /// **Precondition: must be called from a multi-thread Tokio runtime.**
    /// Each coordinator's OS thread drives its loop via
    /// [`Handle::block_on`](tokio::runtime::Handle::block_on) and needs
    /// the runtime's timer/IO driver; a `current_thread` runtime can only
    /// service one `block_on` at a time, so the join would deadlock. This
    /// is asserted in both debug and release builds.
    ///
    /// Each coordinator quiesce+join is bounded by
    /// [`SHUTDOWN_JOIN_TIMEOUT_SECS`] as a backstop. `quiesce()` cancels
    /// the loop, which aborts any in-flight pass at its `.await` point, so
    /// the `is_syncing` drain clears promptly and the join normally lands
    /// far inside the window — the deadline fires only if a pass's *drop*
    /// itself wedges. On timeout the coordinator slot reports
    /// [`CoordinatorThreadStatus::Timeout`] rather than hanging forever.
    ///
    /// The clear-on-panic half of that guarantee rides on unwinding, so
    /// it holds under `panic = "unwind"`. Under the iOS `panic = "abort"`
    /// release profiles a pass panic aborts the process outright (no
    /// `Drop`, no status) — there is no live manager left to read a
    /// status from.
    pub async fn shutdown(&self) -> CoordinatorExitStatus {
        assert!(
            matches!(
                tokio::runtime::Handle::current().runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ),
            "shutdown() requires a multi-thread Tokio runtime: each \
             coordinator's OS thread drives its sync loop via \
             Handle::block_on and needs the runtime's timer/IO driver, but \
             a current_thread runtime can only drive one block_on at a time"
        );

        let timeout = std::time::Duration::from_secs(SHUTDOWN_JOIN_TIMEOUT_SECS);

        // Each quiesce() drains any in-flight pass AND joins the thread.
        let platform_address_sync =
            tokio::time::timeout(timeout, self.platform_address_sync_manager.quiesce())
                .await
                .unwrap_or(CoordinatorThreadStatus::Timeout);

        let identity_sync = tokio::time::timeout(timeout, self.identity_sync_manager.quiesce())
            .await
            .unwrap_or(CoordinatorThreadStatus::Timeout);

        #[cfg(feature = "shielded")]
        let shielded_sync = {
            let r = tokio::time::timeout(timeout, self.shielded_sync_manager.quiesce())
                .await
                .unwrap_or(CoordinatorThreadStatus::Timeout);
            Some(r)
        };
        #[cfg(not(feature = "shielded"))]
        let shielded_sync = None;

        // The event adapter is a tokio task (it sinks the coordinators'
        // stores), so cancel + join it last — after the loops feeding it
        // are gone.
        self.event_adapter_cancel.cancel();
        // Take the handle out into a local first so the `tokio::Mutex`
        // guard doesn't stay held across the (up-to-30s) join `.await`
        // below — a match scrutinee temporary would otherwise keep the
        // guard alive for the whole match.
        let event_adapter_handle = self.event_adapter_join.lock().await.take();
        let event_adapter = match event_adapter_handle {
            None => CoordinatorThreadStatus::NotRunning,
            Some(handle) => match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => CoordinatorThreadStatus::Ok,
                // The returned status already carries this failure, and the
                // FFI `destroy` adapter logs the aggregate once at the host
                // layer — so don't double-log here.
                Ok(Err(e)) => {
                    if e.is_panic() {
                        CoordinatorThreadStatus::Panicked(panic_message(e.into_panic()))
                    } else {
                        // Non-panic JoinError: task was cancelled or aborted —
                        // not a clean exit, but also not a panic.
                        CoordinatorThreadStatus::Stopped(Some(format!("{e}")))
                    }
                }
                Err(_) => CoordinatorThreadStatus::Timeout,
            },
        };

        CoordinatorExitStatus {
            platform_address_sync,
            identity_sync,
            shielded_sync,
            event_adapter,
        }
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

    /// A coordinator thread that panics surfaces as `Panicked` rather
    /// than being silently dropped.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_coordinator_thread_surfaces_panic() {
        let handle = std::thread::spawn(|| panic!("boom in coordinator"));
        match join_coordinator_thread(Some(handle)).await {
            CoordinatorThreadStatus::Panicked(msg) => {
                assert!(msg.contains("boom in coordinator"), "msg was {msg:?}");
            }
            other => panic!("expected Panicked, got {other:?}"),
        }
    }

    /// A non-panic `JoinError` on the event adapter maps to `Stopped`, not
    /// `Ok`, and is NOT considered clean. This covers the case where the
    /// tokio task is cancelled or aborted rather than completing normally.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn event_adapter_non_panic_join_error_maps_to_stopped_and_is_not_clean() {
        // Replace the real adapter handle with a guaranteed-pending task, then
        // abort it. A `pending::<()>()` future can never complete on its own,
        // so abort() always produces a non-panic JoinError — deterministically
        // exercising the Stopped branch regardless of scheduler timing.
        // (The original approach aborted the real adapter handle, which could
        // race the task's own completion and silently yield `Ok` instead.)
        let manager = make_manager();

        // Drain and discard the real adapter (may already be finished).
        let original = {
            let mut guard = manager.event_adapter_join.lock().await;
            guard.take()
        };
        if let Some(h) = original {
            h.abort();
            let _ = h.await;
        }

        // Install a permanently-pending task and abort it so the JoinError
        // path in shutdown() is 100 % deterministic.
        let pending = tokio::spawn(std::future::pending::<()>());
        pending.abort();
        *manager.event_adapter_join.lock().await = Some(pending);

        let status = manager.shutdown().await;

        // The aborted pending task always yields a non-panic JoinError →
        // shutdown() maps it to Stopped.
        assert!(
            matches!(status.event_adapter, CoordinatorThreadStatus::Stopped(_)),
            "expected Stopped from a non-panic JoinError, got {:?}",
            status.event_adapter
        );
        assert!(
            !status.event_adapter.is_clean(),
            "Stopped must not count as clean"
        );
        // Coordinators were never started → their slots are clean.
        assert_eq!(
            status.platform_address_sync,
            CoordinatorThreadStatus::NotRunning
        );
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
        };
        assert!(clean.all_clean());

        let with_timeout = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Timeout,
            identity_sync: CoordinatorThreadStatus::Ok,
            shielded_sync: None,
            event_adapter: CoordinatorThreadStatus::Ok,
        };
        assert!(!with_timeout.all_clean());

        let with_stopped = CoordinatorExitStatus {
            platform_address_sync: CoordinatorThreadStatus::Ok,
            identity_sync: CoordinatorThreadStatus::Ok,
            shielded_sync: Some(CoordinatorThreadStatus::Stopped(Some("aborted".into()))),
            event_adapter: CoordinatorThreadStatus::Ok,
        };
        assert!(!with_stopped.all_clean());
    }

    /// A cleanly-returning thread joins as `Ok`; an absent handle is
    /// `NotRunning`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_coordinator_thread_ok_and_absent() {
        let handle = std::thread::spawn(|| {});
        assert_eq!(
            join_coordinator_thread(Some(handle)).await,
            CoordinatorThreadStatus::Ok
        );
        assert_eq!(
            join_coordinator_thread(None).await,
            CoordinatorThreadStatus::NotRunning
        );
    }

    /// `join_coordinator_thread` uses `is_finished()` polling. Verify
    /// it completes within a bounded time on a multi-thread runtime, as
    /// `shutdown()` requires (and that it doesn't busy-spin indefinitely).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_coordinator_thread_completes_within_deadline() {
        let handle = std::thread::spawn(|| {});
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            join_coordinator_thread(Some(handle)),
        )
        .await
        .expect("join_coordinator_thread must complete within 5 s");
        assert_eq!(result, CoordinatorThreadStatus::Ok);
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
    /// could swallow diagnostics from concurrently-running tests (e.g.
    /// `join_coordinator_thread_surfaces_panic`).
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
}
