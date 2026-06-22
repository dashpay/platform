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

/// Terminal status of one background coordinator's OS thread.
///
/// The three periodic coordinators run their loops on dedicated OS
/// threads (the SDK futures are `!Send`, so they ride
/// [`Handle::block_on`](tokio::runtime::Handle::block_on) rather than
/// `tokio::spawn`). [`PlatformWalletManager::shutdown`] joins each
/// thread and reports how it ended so a host can tell a clean wind-down
/// from a panicked loop instead of silently dropping the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorThreadStatus {
    /// No thread was running to join — the loop was never started, or
    /// was already stopped and joined.
    NotRunning,
    /// The loop exited and its OS thread joined cleanly.
    Ok,
    /// The OS thread panicked; carries the best-effort panic message.
    Panicked(String),
    /// The join itself could not complete (the blocking join task
    /// failed). Distinct from the thread panicking.
    Error(String),
}

impl CoordinatorThreadStatus {
    /// `true` for a non-failure outcome (joined cleanly or never ran).
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Ok | Self::NotRunning)
    }
}

/// Per-thread terminal status of every background coordinator, returned
/// by [`PlatformWalletManager::shutdown`].
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
    pub platform_address: CoordinatorThreadStatus,
    /// Per-identity token-state sync loop.
    pub identity: CoordinatorThreadStatus,
    /// Shielded (Orchard) note sync loop. Always
    /// [`CoordinatorThreadStatus::NotRunning`] in builds without the
    /// `shielded` feature.
    pub shielded: CoordinatorThreadStatus,
}

impl CoordinatorExitStatus {
    /// `true` when every coordinator wound down without a panic or join
    /// failure (each is [`Ok`](CoordinatorThreadStatus::Ok) or
    /// [`NotRunning`](CoordinatorThreadStatus::NotRunning)).
    pub fn all_clean(&self) -> bool {
        self.platform_address.is_clean() && self.identity.is_clean() && self.shielded.is_clean()
    }
}

/// Join a coordinator's background OS thread and classify how it ended.
///
/// Awaited by [`quiesce`](IdentitySyncManager::quiesce) *after* the loop
/// is cancelled and its in-flight pass drained, so the thread is already
/// on its way out. The blocking [`JoinHandle::join`](std::thread::JoinHandle::join)
/// runs on the blocking pool (via [`spawn_blocking`](tokio::task::spawn_blocking))
/// to avoid parking a runtime worker. Joining here — while the runtime
/// is still alive — is what guarantees the `!Send` loop has stopped
/// touching `tokio::time` before the host drops the runtime.
pub(crate) async fn join_coordinator_thread(
    handle: Option<std::thread::JoinHandle<()>>,
) -> CoordinatorThreadStatus {
    let Some(handle) = handle else {
        return CoordinatorThreadStatus::NotRunning;
    };
    match tokio::task::spawn_blocking(move || handle.join()).await {
        Ok(Ok(())) => CoordinatorThreadStatus::Ok,
        Ok(Err(payload)) => CoordinatorThreadStatus::Panicked(panic_message(payload)),
        Err(join_err) => CoordinatorThreadStatus::Error(join_err.to_string()),
    }
}

/// Best-effort extraction of a panic message from a joined thread's
/// payload (`&str` and `String` are the common cases).
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

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
        self.shielded_sync_manager.quiesce().await;
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
    /// Each `quiesce()` now also **joins** its coordinator's OS thread,
    /// so when this returns every `!Send` loop has fully exited. A host
    /// that drops the tokio runtime right after `shutdown()` (one-shot /
    /// headless / stdio) is therefore safe — no coordinator can still be
    /// polling `tokio::time` on a shutting-down runtime. The returned
    /// [`CoordinatorExitStatus`] reports per-thread how each loop ended.
    pub async fn shutdown(&self) -> CoordinatorExitStatus {
        let platform_address = self.platform_address_sync_manager.quiesce().await;
        let identity = self.identity_sync_manager.quiesce().await;
        #[cfg(feature = "shielded")]
        let shielded = self.shielded_sync_manager.quiesce().await;
        #[cfg(not(feature = "shielded"))]
        let shielded = CoordinatorThreadStatus::NotRunning;

        self.event_adapter_cancel.cancel();
        if let Some(handle) = self.event_adapter_join.lock().await.take() {
            if let Err(e) = handle.await {
                tracing::warn!(error = ?e, "Wallet event adapter task join error");
            }
        }

        CoordinatorExitStatus {
            platform_address,
            identity,
            shielded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletChangeSet};

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

    /// Start every periodic coordinator's background OS-thread loop.
    fn start_coordinators<P: PlatformWalletPersistence + 'static>(m: &PlatformWalletManager<P>) {
        Arc::clone(&m.platform_address_sync_manager).start();
        Arc::clone(&m.identity_sync_manager).start();
        #[cfg(feature = "shielded")]
        Arc::clone(&m.shielded_sync_manager).start();
    }

    /// (a) `shutdown()` joins all coordinator OS threads and reports an
    /// all-clean status; a second call has nothing left to join.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_joins_all_coordinators_and_reports_ok() {
        let manager = make_manager();
        start_coordinators(&manager);
        // Let the loops enter `block_on` so we exercise the live-loop
        // join path (a thread cancelled before its first poll joins too).
        tokio::time::sleep(Duration::from_millis(50)).await;

        let status = manager.shutdown().await;
        assert_eq!(status.platform_address, CoordinatorThreadStatus::Ok);
        assert_eq!(status.identity, CoordinatorThreadStatus::Ok);
        #[cfg(feature = "shielded")]
        assert_eq!(status.shielded, CoordinatorThreadStatus::Ok);
        #[cfg(not(feature = "shielded"))]
        assert_eq!(status.shielded, CoordinatorThreadStatus::NotRunning);
        assert!(status.all_clean());

        // Handles consumed by the join → nothing left to join.
        let again = manager.shutdown().await;
        assert_eq!(again.platform_address, CoordinatorThreadStatus::NotRunning);
        assert_eq!(again.identity, CoordinatorThreadStatus::NotRunning);
    }

    /// (b) A coordinator thread that panics surfaces in the status rather
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

    /// A cleanly-returning thread joins as `Ok`; an absent handle is
    /// `NotRunning`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_coordinator_thread_clean_and_absent() {
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

    /// (c) Race regression: model the one-shot / headless path — start
    /// the coordinators, `shutdown()`, then **drop the runtime**. Because
    /// `shutdown()` joined every loop while the runtime was still alive
    /// (asserted via the all-`Ok` status), nothing is left polling
    /// `tokio::time`, so the drop raises no "Tokio … being shutdown"
    /// panic. A scoped hook counts only that specific panic so a
    /// concurrent unrelated panic can't trip the assertion.
    #[test]
    fn shutdown_then_drop_runtime_does_not_panic() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        static SHUTDOWN_PANICS: AtomicUsize = AtomicUsize::new(0);
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|info| {
            if info.to_string().contains("being shutdown") {
                SHUTDOWN_PANICS.fetch_add(1, AtomicOrdering::SeqCst);
            }
        }));

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("build runtime");

        let status = runtime.block_on(async {
            let manager = make_manager();
            start_coordinators(&manager);
            tokio::time::sleep(Duration::from_millis(50)).await;
            manager.shutdown().await
        });

        // The headless drop: with every coordinator already joined, this
        // cannot race a loop still touching the timer.
        drop(runtime);
        std::thread::sleep(Duration::from_millis(100));
        let racing_panics = SHUTDOWN_PANICS.load(AtomicOrdering::SeqCst);

        // Restore the hook before asserting so a failure prints normally.
        std::panic::set_hook(prev_hook);

        assert_eq!(status.platform_address, CoordinatorThreadStatus::Ok);
        assert_eq!(status.identity, CoordinatorThreadStatus::Ok);
        assert!(
            status.all_clean(),
            "coordinators did not wind down: {status:?}"
        );
        assert_eq!(
            racing_panics, 0,
            "dropping the runtime after shutdown raced a coordinator thread"
        );
    }
}
