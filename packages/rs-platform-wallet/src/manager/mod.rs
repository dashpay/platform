//! Multi-wallet manager with SPV coordination.

pub mod accessors;
pub mod dashpay_sync;
pub mod identity_sync;
mod load;
pub mod platform_address_sync;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
mod wallet_lifecycle;

use std::sync::Arc;

use dash_async::{
    ShutdownReport, ShutdownWeight, ThreadRegistry, WorkerConfig, DEFAULT_JOIN_BUDGET,
};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use key_wallet_manager::WalletManager;

use crate::changeset::{spawn_wallet_event_adapter, PlatformWalletPersistence};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::manager::dashpay_sync::DashPaySyncManager;
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::asset_lock::LockNotifyHandler;
use crate::wallet::core::BalanceUpdateHandler;
use crate::wallet::identity::network::DashPayPaymentHandler;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Registry key identifying each background worker the manager joins at
/// shutdown.
///
/// The four periodic sync coordinators run their `!Send` loops on OS
/// threads the shared [`ThreadRegistry`] spawns and owns end to end: it
/// installs each loop's cancellation token, and
/// [`shutdown`](PlatformWalletManager::shutdown) cancels and joins them —
/// surfacing a panicked loop — before the host drops the tokio runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WalletWorker {
    /// Platform-address (BLAST / DIP-17) balance sync coordinator.
    PlatformAddressSync,
    /// Per-identity token-state sync coordinator.
    IdentitySync,
    /// DashPay (contact requests + profiles) sync coordinator.
    DashPaySync,
    /// Shielded (Orchard) note sync coordinator.
    ShieldedSync,
}

// `dash_async::RegistryKey` is a blanket impl over
// `Copy + Ord + Eq + Debug + Send + Sync + 'static`, which the derives above
// satisfy — no explicit impl needed.

/// Teardown tier for the periodic coordinators. All four share one tier so
/// [`ThreadRegistry::shutdown`] drains them concurrently.
pub(crate) const COORDINATOR_WEIGHT: ShutdownWeight = ShutdownWeight(0);

/// Base [`WorkerConfig`] each coordinator starts its loop thread with — one
/// shared tier, no drain hook, the registry's default managed-join budget
/// ([`DEFAULT_JOIN_BUDGET`]) so a wedged loop pass surfaces as
/// [`WorkerStatus::Timeout`](dash_async::WorkerStatus::Timeout) instead of
/// hanging shutdown forever, and the platform default OS-thread stack. A
/// coordinator that needs a deeper stack (e.g. DashPay's GroveDB proof
/// descent) overrides `stack_size` on top of this.
pub(crate) fn coordinator_worker_config() -> WorkerConfig {
    WorkerConfig {
        weight: COORDINATOR_WEIGHT,
        drain: None,
        join_budget: DEFAULT_JOIN_BUDGET,
        stack_size: None,
    }
}

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
    /// Periodic DashPay sync coordinator. Drives `dashpay_sync()`
    /// (contact requests + profiles) on **every** registered wallet
    /// each sweep — wallet-driven, not token-registry-driven, so
    /// DashPay-only identities are never skipped. Shares the same
    /// `wallets` map as [`PlatformAddressSyncManager`]. Not
    /// auto-started — call `start` after wallets are registered. See
    /// [`DashPaySyncManager`].
    pub(super) dashpay_sync_manager: Arc<DashPaySyncManager>,
    /// Tracks asynchronous payment hooks so manager shutdown can close
    /// admission and drain every task before host callback contexts are freed.
    pub(super) dashpay_payment_handler: Arc<DashPayPaymentHandler>,
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
    /// Shared lifecycle registry for the periodic coordinator threads.
    /// Each coordinator spawns its loop through `registry.start_thread` at
    /// `start`, handing the registry ownership of the OS thread and its
    /// cancellation token; [`shutdown`](Self::shutdown) cancels, joins, and
    /// reports per-worker terminal status.
    pub(super) registry: Arc<ThreadRegistry<WalletWorker>>,
    /// Host-visible hard sync-fault latch (dashpay/platform#4069). Set
    /// (and never cleared) by the wallet-event adapter the first time it
    /// freezes a durable watermark after a persistence `store()` rejection
    /// or a dropped-event broadcast lag. Poll via
    /// [`Self::sync_fault_detected`] to surface a "verification failed /
    /// rescan pending" state rather than re-freezing silently on the next
    /// launch.
    pub(super) sync_fault: Arc<std::sync::atomic::AtomicBool>,
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
        // Subscribe to the wallet-event broadcast BEFORE the manager is
        // wrapped in the shared `Arc<RwLock>` and handed to any producer,
        // so no event emitted during startup is lost without a `Lagged`
        // marker (a `broadcast::Receiver` only sees messages sent after its
        // `subscribe()` — see `run_wallet_event_adapter`'s
        // subscribe-before-publish note). The receiver is created here,
        // synchronously, and moved into the adapter task below.
        let wallet_manager_inner = WalletManager::new(sdk.network);
        let event_receiver = wallet_manager_inner.subscribe_events();
        let wallet_manager = Arc::new(RwLock::new(wallet_manager_inner));
        let wallets = Arc::new(RwLock::new(std::collections::BTreeMap::new()));
        let lock_notify = Arc::new(Notify::new());
        // Shared registry that owns the coordinators' loop-thread join
        // handles for a clean, panic-aware shutdown join.
        let registry = ThreadRegistry::<WalletWorker>::new();

        // Host-visible hard sync-fault latch (dashpay/platform#4069). The
        // adapter raises it the first time it freezes a durable watermark.
        let sync_fault = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Spawn the wallet-event adapter that translates upstream
        // `WalletEvent`s into `CoreChangeSet`s and forwards them to
        // the persister.
        let event_adapter_cancel = CancellationToken::new();
        let event_adapter_join = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::clone(&persister),
            event_receiver,
            Arc::clone(&sync_fault),
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
        // DashPayPaymentHandler records incoming DashPay payments and
        // confirms sent ones off the wallet-event fan-out, keeping that
        // domain logic out of the generic core-changeset bridge. It holds
        // the wallet-manager (for the in-memory payment state it mutates)
        // and the persister (to write the resulting payment rows).
        let dashpay_payment_handler = Arc::new(DashPayPaymentHandler::new(
            Arc::clone(&wallet_manager),
            Arc::clone(&persister) as Arc<dyn PlatformWalletPersistence>,
        ));
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            app_handler,
            lock_handler,
            balance_handler,
            Arc::clone(&dashpay_payment_handler) as Arc<dyn PlatformEventHandler>,
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
        // DashPay sync shares the `wallets` map (not the token
        // registry) so DashPay-only identities sync on every sweep.
        let dashpay_sync = Arc::new(DashPaySyncManager::new(
            Arc::clone(&wallets),
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
            dashpay_sync_manager: dashpay_sync,
            dashpay_payment_handler,
            #[cfg(feature = "shielded")]
            shielded_sync_manager: shielded_sync,
            #[cfg(feature = "shielded")]
            shielded_coordinator,
            #[cfg(feature = "shielded")]
            event_manager,
            persister,
            event_adapter_cancel,
            event_adapter_join: tokio::sync::Mutex::new(Some(event_adapter_join)),
            registry,
            sync_fault,
        }
    }

    /// Whether the wallet-event adapter has frozen a durable sync
    /// watermark this session (dashpay/platform#4069).
    ///
    /// Returns `true` once — and stays `true` for the manager's lifetime
    /// — after the adapter drops record-bearing events (a broadcast lag)
    /// or a persistence `store()` is rejected, meaning the persisted
    /// `syncedHeight` is deliberately held behind the chain tip and a
    /// rescan is pending on the next launch. Integrators poll this to
    /// surface a hard "verification failed / rescan pending" state instead
    /// of the fault being visible only in error logs. It is intentionally
    /// a coarse, latch-once, all-or-nothing signal (the per-wallet vs.
    /// global scoping lives inside the adapter); a host that needs
    /// per-wallet granularity should re-derive it from the persisted
    /// watermark vs. chain tip.
    pub fn sync_fault_detected(&self) -> bool {
        self.sync_fault.load(std::sync::atomic::Ordering::Relaxed)
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
    /// the network coordinator's per-subwallet registries AND reset the
    /// on-disk commitment-tree SQLite store. The per-network
    /// commitment-tree SQLite file stays on disk but its contents are
    /// reset to empty so the next bind cold-resyncs from index 0.
    ///
    /// # The missing-coordinator case is an ERROR, not a silent no-op
    ///
    /// This used to `Ok(())` when `shielded_coordinator()` was `None`,
    /// treating "no coordinator" as "nothing to clear". That masked the exact
    /// on-device failure this fix targets: the host taps Clear on a manager
    /// whose coordinator is **not installed on this instance** — e.g. an SDK
    /// rebuild handed the host a fresh `PlatformWalletManager` whose
    /// `configure_shielded` never ran (or ran on a different instance than the
    /// one currently syncing). The quiesce runs (sync loop stops), the call
    /// returns `Ok`, and the host then wipes its own Room/SwiftData rows —
    /// while the **on-disk commitment tree is never touched** (file mtime
    /// unchanged on device, no `reset_commitment_tree` call). The next bind
    /// reloads the still-full tree + its persisted watermark and re-freezes
    /// everything.
    ///
    /// The FFI only exposes this call behind a bound, shielded-enabled host
    /// surface (the "Clear" button), so reaching it with no coordinator is a
    /// genuine wiring fault, not a benign "shielded was never used" case.
    /// Returning an error makes the host fail closed (keep its rows) and
    /// surfaces the real problem instead of a phantom success.
    ///
    /// Returns an error if the coordinator is absent, or if the coordinator's
    /// store reset (which resets the on-disk tree to 0 leaves and verifies it)
    /// fails; the host must not commit its own persistence wipe in that case.
    #[cfg(feature = "shielded")]
    pub async fn clear_shielded(&self) -> Result<(), crate::error::PlatformWalletError> {
        // Hold the registry's per-key clearing latch across the WHOLE
        // quiesce -> wipe. While it is up, `ShieldedSyncManager::start`
        // (and any registry (re)start) is a no-op, so no fresh pass can
        // slip between the quiesce and the wipe and re-persist notes into
        // the store `coord.clear()` is about to reset. The guard's Drop
        // releases the latch on every exit path (including `?` and panic).
        let _clearing = self.registry.hold_clearing(WalletWorker::ShieldedSync);
        self.shielded_sync_manager.quiesce().await;
        match self.shielded_coordinator().await {
            Some(coord) => coord.clear().await,
            None => {
                tracing::error!(
                    "clear_shielded: no shielded coordinator installed on this manager — the \
                     on-disk commitment tree cannot be reset. configure_shielded never ran on \
                     THIS manager instance (or ran on a different one)."
                );
                Err(crate::error::PlatformWalletError::ShieldedStoreError(
                    "shielded clear requested but no coordinator is configured on this manager — \
                     on-disk tree not reset"
                        .to_string(),
                ))
            }
        }
    }

    /// Reset the platform-address (BLAST/DIP-17) incremental-sync
    /// watermark and drop every cached balance across **all**
    /// registered wallets, forcing a full rescan on the next sync.
    ///
    /// Backs the SwiftExampleApp "Clear" button. Manager-level (not
    /// per-wallet) to match [`clear_shielded`](Self::clear_shielded):
    /// the host's persistence delete is global, so a per-wallet reset
    /// would leave sibling wallets' in-memory watermarks to
    /// re-populate the deleted rows on the next sync.
    ///
    /// Quiesces the platform-address sync manager first so no in-flight
    /// pass can call `update_sync_state` and re-write the watermark (or
    /// re-seed balances) *after* the reset. Does NOT restart the loop —
    /// manual "Sync Now" works without it, and leaving it stopped is
    /// the desired UX: data stays cleared until the user explicitly
    /// resyncs. `quiesce` leaves the manager stopped-but-restartable.
    pub async fn reset_platform_address_sync_state(
        &self,
    ) -> Result<(), crate::error::PlatformWalletError> {
        self.platform_address_sync_manager.quiesce().await;

        // Snapshot Arc clones under a short read lock; never hold the
        // `wallets` read guard across the per-wallet `.await`s below —
        // that would block registration and invite lock-ordering
        // issues against each wallet's `wallet_manager` lock.
        let wallets: Vec<Arc<PlatformWallet>> = {
            let guard = self.wallets.read().await;
            guard.values().cloned().collect()
        };

        for wallet in wallets {
            wallet.platform().reset_sync_state().await;
        }
        Ok(())
    }

    /// Stop all background tasks, join their threads, and report how each
    /// one ended.
    ///
    /// Stops SPV and **quiesces** the periodic coordinators
    /// (`PlatformAddressSyncManager`, `IdentitySyncManager`,
    /// `DashPaySyncManager`, `ShieldedSyncManager`) — cancelling each loop
    /// *and draining any in-flight pass to completion*, including its
    /// persister / host-callback fan-out — then **joins** their loop OS
    /// threads through the shared [`ThreadRegistry`] and finally drains the
    /// wallet-event adapter task. Idempotent.
    ///
    /// Ordering matters and is fourfold:
    /// 1. SPV is stopped and joined FIRST so it cannot dispatch more wallet
    ///    events, then payment-task admission is closed and all admitted
    ///    DashPay payment-hook work is joined.
    /// 2. `quiesce()` each coordinator. Cancel-only `stop()` would
    ///    let a pass already inside `sync_now` keep running and call
    ///    `persister.store(...)` / fire a host completion callback after
    ///    the FFI's `destroy` returned and the host freed the persister /
    ///    event-handler context — a use-after-free.
    /// 3. `registry.shutdown()` then JOINS the coordinator OS threads.
    ///    `quiesce`'s `is_syncing` barrier only proves no pass is *in
    ///    flight*; the detached thread may still be unwinding out of
    ///    `Handle::block_on`, touching `tokio::time` on a runtime the host
    ///    is about to drop. Joining guarantees it has fully exited, and
    ///    surfaces a panicked loop as a non-clean [`WorkerStatus`] rather
    ///    than silently dropping it.
    /// 4. The event adapter — the sink those stores feed into — drains
    ///    LAST.
    ///
    /// Returns a [`ShutdownReport`] keyed by [`WalletWorker`]; inspect
    /// [`ShutdownReport::all_clean`] before freeing the host callback
    /// context. A non-clean status flags a still-live worker or orphan.
    ///
    /// [`WorkerStatus`]: dash_async::WorkerStatus
    pub async fn shutdown(&self) -> ShutdownReport<WalletWorker> {
        if let Err(error) = self.spv_manager.stop().await {
            tracing::warn!(?error, "SPV shutdown failed");
        }

        self.dashpay_payment_handler.quiesce().await;
        self.platform_address_sync_manager.quiesce().await;
        self.identity_sync_manager.quiesce().await;
        self.dashpay_sync_manager.quiesce().await;
        #[cfg(feature = "shielded")]
        self.shielded_sync_manager.quiesce().await;

        // Hard-join the coordinator loop threads now that every in-flight
        // pass has drained. This is the barrier `quiesce` cannot give:
        // it waits for the actual OS thread to terminate before the host
        // drops the runtime.
        let report = self.registry.shutdown().await;

        // The wallet-event adapter is the sink the coordinators' stores
        // feed into, so it drains AFTER them. It is a plain tokio task, not
        // a registry worker, so it is joined here rather than in the report.
        self.event_adapter_cancel.cancel();
        if let Some(handle) = self.event_adapter_join.lock().await.take() {
            if let Err(e) = handle.await {
                tracing::warn!(error = ?e, "Wallet event adapter task join error");
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dash_async::WorkerStatus;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};

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

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    fn make_manager() -> Arc<PlatformWalletManager<NoopPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(NoopPersister),
            Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>,
        ))
    }

    /// `shutdown()` joins every started coordinator through the shared
    /// [`ThreadRegistry`], reports each as cleanly joined, and is
    /// idempotent — a second call finds nothing running and still reports
    /// clean. This is the barrier the previous discard-the-handle `start`
    /// could not give: proof the loop OS threads have terminated before the
    /// host drops the runtime.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_joins_started_coordinators_and_is_idempotent() {
        let mgr = make_manager();
        // Empty wallet/identity state, so each coordinator's first pass is a
        // no-op and no network I/O happens; the point is thread lifecycle.
        Arc::clone(&mgr.identity_sync_manager).start();
        Arc::clone(&mgr.platform_address_sync_manager).start();
        Arc::clone(&mgr.dashpay_sync_manager).start();

        let report = mgr.shutdown().await;
        assert!(report.all_clean(), "clean shutdown: {report:?}");
        for worker in [
            WalletWorker::IdentitySync,
            WalletWorker::PlatformAddressSync,
            WalletWorker::DashPaySync,
        ] {
            assert_eq!(
                report.per_worker.get(&worker),
                Some(&WorkerStatus::Ok),
                "{worker:?} must join cleanly"
            );
        }

        // Second shutdown: the coordinators already joined, so the registry
        // reports them NotRunning and the report stays clean.
        let again = mgr.shutdown().await;
        assert!(again.all_clean(), "idempotent shutdown: {again:?}");
    }
}
