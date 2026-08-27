//! Multi-wallet manager with SPV coordination.

pub mod accessors;
pub mod dashpay_sync;
pub mod dpns_sync;
pub mod identity_sync;
mod load;
pub mod platform_address_sync;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
pub mod startup;
mod wallet_lifecycle;

use std::sync::Arc;
use std::time::Duration;

use dash_async::{ShutdownReport, ThreadRegistry, WorkerConfig, WorkerStatus, DEFAULT_JOIN_BUDGET};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use key_wallet_manager::WalletManager;

use crate::changeset::{spawn_wallet_event_adapter, PlatformWalletPersistence};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::manager::dashpay_sync::DashPaySyncManager;
use crate::manager::dpns_sync::DpnsSyncManager;
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::asset_lock::LockNotifyHandler;
use crate::wallet::core::{BalanceUpdateHandler, SpendObservationHandler};
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
    /// DPNS username-marketplace sync coordinator.
    DpnsSync,
    /// Shielded (Orchard) note sync coordinator.
    ShieldedSync,
    /// SPV runtime — the network event source feeding every persister-
    /// visible wallet event. Not a registry worker: `SpvRuntime::stop`
    /// owns its (bounded, abort-escalating) join, and
    /// [`shutdown`](PlatformWalletManager::shutdown) folds the stop
    /// outcome into the report so a failed SPV stop can never hide
    /// behind a clean coordinator join.
    Spv,
    /// DashPay payment-hook tasks spawned by `DashPayPaymentHandler` in
    /// response to SPV wallet events. Not a registry worker: the
    /// handler's own tracker closes admission and joins the admitted
    /// tasks; its drain outcome is folded into the report because those
    /// tasks clone the FFI persister and can fire host callbacks.
    DashPayPayments,
    /// The wallet-event adapter task — the sink coordinator stores feed
    /// into. Not a registry worker: joined by
    /// [`shutdown`](PlatformWalletManager::shutdown) under a bounded
    /// budget, with the live handle re-parked on timeout so a destroy
    /// retry can re-join it.
    EventAdapter,
}

// `dash_async::RegistryKey` is a blanket impl over
// `Copy + Ord + Eq + Debug + Send + Sync + 'static`, which the derives above
// satisfy — no explicit impl needed.

/// Deadline for a coordinator `quiesce()` drain — how long we wait for an
/// in-flight pass (its `is_syncing` slot) to fall before giving up and
/// reporting the coordinator non-clean. Without a bound, a pass wedged in
/// a network / persister / host-callback await blocks `shutdown()` (and
/// therefore the FFI's `destroy`) forever, *before* the registry's
/// per-worker join budget ever gets a chance to run. A timed-out drain is
/// surfaced as [`WorkerStatus::Timeout`](dash_async::WorkerStatus::Timeout)
/// so `all_clean()` fails and the host keeps its callback context alive.
pub(crate) const COORDINATOR_DRAIN_BUDGET: Duration = Duration::from_secs(10);

/// Deadline for draining the DashPay payment-hook tasks at shutdown.
/// After it lapses each straggler is aborted and given
/// [`PAYMENT_ABORT_GRACE`] to confirm termination; anything still alive
/// is kept tracked and reported non-clean.
pub(crate) const PAYMENT_DRAIN_BUDGET: Duration = Duration::from_secs(10);

/// Post-abort confirmation grace for one payment-hook task. An abort only
/// takes effect at the task's next await point, so a task stuck inside a
/// synchronous persister call cannot be interrupted — after this grace it
/// is left tracked (for a retry to re-join) and reported non-clean.
pub(crate) const PAYMENT_ABORT_GRACE: Duration = Duration::from_secs(1);

/// Deadline for joining the wallet-event adapter task at shutdown. The
/// adapter exits promptly on cancellation; the bound exists so a persister
/// `store` it is blocked in cannot hang `destroy`. On timeout the live
/// handle is re-parked so a destroy retry re-joins it, and the report
/// carries [`WorkerStatus::Timeout`](dash_async::WorkerStatus::Timeout).
const EVENT_ADAPTER_JOIN_BUDGET: Duration = Duration::from_secs(10);

/// RAII holder for a coordinator's `is_syncing` slot: clears the flag on
/// drop, **including panic unwind out of a pass body**. Every pass must
/// hold one of these instead of storing `false` manually — a panicking
/// pass that leaves `is_syncing` latched would wedge `quiesce()`'s drain
/// until its budget lapses on every subsequent teardown.
pub(crate) struct SyncSlotGuard<'a>(pub(crate) &'a std::sync::atomic::AtomicBool);

impl Drop for SyncSlotGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Sync-pass admission gate shared by all four coordinators.
///
/// A pass claims its coordinator's `is_syncing` slot and then checks this
/// gate; when the gate is closed it releases the slot and bails without
/// touching any state. The gate is what turns "no pass is in flight right
/// now" into "no pass is in flight *and none can start*", which is the
/// barrier Clear / reset / shutdown need before they mutate or free the
/// state a pass would touch.
///
/// Four independent reasons close it, deliberately kept apart:
/// - an active drain ([`drain_pass`]) — the drain **counts as a holder
///   from its first instruction**, so overlapping drains keep the gate
///   shut for each other (see the race note on [`GateBookkeeping::holds`]),
/// - a live [`QuiesceGuard`] holder — a caller that drained and is still
///   mutating; the gate reopens when the last hold drops,
/// - the latch — a drain that timed out leaves the gate stuck closed with
///   no holder, so the wedged pass cannot be followed by a fresh one; the
///   next *successful* drain clears it,
/// - [`seal`](Self::seal) — terminal, set by
///   [`shutdown`](PlatformWalletManager::shutdown); never reopens, so a
///   direct `sync_now` that was already dispatched on a host thread
///   cannot start a fresh pass after the drain concluded and the FFI
///   freed the callback context.
#[derive(Default)]
pub(crate) struct QuiesceGate {
    /// The single flag every pass reads — one atomic load on the hot path
    /// instead of taking `bookkeeping`. Only ever written while holding
    /// that lock, so it is always consistent with the state below.
    closed: std::sync::atomic::AtomicBool,
    /// Serializes every transition. Without it, a guard dropping (reopen)
    /// can interleave with another caller closing + taking a hold, and the
    /// stale reopen wins — leaving admission open under a live holder.
    /// Held for a handful of instructions and never across an `.await`.
    bookkeeping: std::sync::Mutex<GateBookkeeping>,
}

#[derive(Default)]
struct GateBookkeeping {
    /// Live [`QuiesceGuard`]s — including every drain still in flight,
    /// which takes its hold at [`drain_pass`] entry rather than after its
    /// final `is_syncing` observation. The early hold is load-bearing:
    /// were a drain not counted until it finished, a concurrent holder's
    /// drop could reopen the gate in the window between the drain's last
    /// `is_syncing` load and its own hold, letting a direct sync claim
    /// the slot and pass the gate check — and the drain would then return
    /// "success" to a caller about to wipe state under that live pass.
    holds: usize,
    /// A drain timed out with the pass still holding `is_syncing`; keeps
    /// the gate closed with no holder until a later drain succeeds.
    latched: bool,
    /// Terminal close. Wins over everything.
    sealed: bool,
}

impl GateBookkeeping {
    fn should_close(&self) -> bool {
        self.sealed || self.latched || self.holds > 0
    }
}

impl QuiesceGate {
    /// Whether new sync passes are currently barred.
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Acquire)
    }

    fn bookkeeping(&self) -> std::sync::MutexGuard<'_, GateBookkeeping> {
        // The critical sections are straight-line counter updates that
        // cannot panic, so the lock cannot actually be poisoned.
        self.bookkeeping
            .lock()
            .expect("quiesce gate mutex poisoned")
    }

    /// Recompute the hot-path flag from the bookkeeping — the ONLY writer
    /// of `closed`, always under the lock.
    fn publish_locked(&self, bookkeeping: &GateBookkeeping) {
        self.closed.store(
            bookkeeping.should_close(),
            std::sync::atomic::Ordering::Release,
        );
    }

    /// Take a hold, closing the gate. Called at [`drain_pass`] entry (the
    /// drain itself is a holder) — there is no close-without-hold except
    /// the timeout latch and the seal.
    fn hold(&self) -> QuiesceGuard<'_> {
        let mut bookkeeping = self.bookkeeping();
        bookkeeping.holds += 1;
        self.publish_locked(&bookkeeping);
        QuiesceGuard(self)
    }

    /// A drain observed the pass fully drained while holding the gate:
    /// clear any latch left by a previously timed-out drain. The caller
    /// still holds its guard, so the gate stays closed until that drops.
    fn drain_succeeded(&self) {
        let mut bookkeeping = self.bookkeeping();
        bookkeeping.latched = false;
        self.publish_locked(&bookkeeping);
    }

    /// A drain gave up with the pass still holding `is_syncing`: latch the
    /// gate closed so dropping the drain's own hold cannot reopen it.
    fn latch_closed(&self) {
        let mut bookkeeping = self.bookkeeping();
        bookkeeping.latched = true;
        self.publish_locked(&bookkeeping);
    }

    /// Drop a hold, reopening the gate only if nothing else closes it.
    fn release(&self) {
        let mut bookkeeping = self.bookkeeping();
        bookkeeping.holds = bookkeeping.holds.saturating_sub(1);
        self.publish_locked(&bookkeeping);
    }

    /// Close the gate permanently. Used by manager shutdown, after which
    /// no pass may ever start again on this manager instance.
    pub(crate) fn seal(&self) {
        let mut bookkeeping = self.bookkeeping();
        bookkeeping.sealed = true;
        self.publish_locked(&bookkeeping);
    }
}

/// RAII hold on a closed [`QuiesceGate`]: keeps new passes barred for as
/// long as the holder is mutating state a pass would touch, and reopens
/// the gate on drop — including `?` early-return and panic unwind.
///
/// Without this, `quiesce()` reopened the gate the instant it returned, so
/// `clear_shielded` / `reset_platform_address_sync_state` ran their wipe
/// with admission already re-opened: a direct `sync_now` on a host thread
/// could snapshot pre-wipe state and re-persist it right after the wipe.
#[must_use = "dropping the guard immediately reopens sync admission, which defeats the barrier"]
pub(crate) struct QuiesceGuard<'a>(&'a QuiesceGate);

impl Drop for QuiesceGuard<'_> {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// Shared drain body behind every coordinator's `quiesce*` family: take a
/// hold on the gate so no new pass can start, cancel the loop, then wait
/// for the in-flight pass (if any) to release `is_syncing`.
///
/// `is_syncing` is held across a pass's persister / host-callback fan-out,
/// so its falling edge *with the gate closed* is a sound "fully drained,
/// nothing more will fire" signal. The hold is taken at ENTRY — before the
/// first `is_syncing` observation — so the gate is closed continuously
/// from here to the returned guard's drop, and no concurrent holder's
/// release can open an admission window mid-drain (the race a
/// close-then-hold-at-the-end sequence has).
///
/// Returns a [`QuiesceGuard`] that keeps the gate closed until it drops.
/// Returns `None` when the pass was still holding `is_syncing` at the
/// deadline; the gate is latched closed on that path (the wedged pass
/// must not be followed by a fresh one — a later successful drain clears
/// the latch) and the caller must fail closed.
pub(crate) async fn drain_pass<'a>(
    gate: &'a QuiesceGate,
    is_syncing: &std::sync::atomic::AtomicBool,
    stop: impl FnOnce(),
    budget: Duration,
) -> Option<QuiesceGuard<'a>> {
    let guard = gate.hold();
    stop();
    let deadline = tokio::time::Instant::now() + budget;
    while is_syncing.load(std::sync::atomic::Ordering::Acquire) {
        if tokio::time::Instant::now() >= deadline {
            // Latch BEFORE the guard drops so there is no instant in
            // which the gate is open on the timeout path.
            gate.latch_closed();
            drop(guard);
            return None;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    gate.drain_succeeded();
    Some(guard)
}

/// Base [`WorkerConfig`] each coordinator starts its loop thread with — the
/// registry's default managed-join budget ([`DEFAULT_JOIN_BUDGET`]) so a
/// wedged loop pass surfaces as
/// [`WorkerStatus::Timeout`](dash_async::WorkerStatus::Timeout) instead of
/// hanging shutdown forever, and the platform default OS-thread stack. A
/// coordinator that needs a deeper stack (e.g. DashPay's GroveDB proof
/// descent) overrides `stack_size` on top of this.
pub(crate) fn coordinator_worker_config() -> WorkerConfig {
    WorkerConfig {
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
    /// Periodic DPNS username-marketplace sync coordinator. Drives
    /// `sync_dpns_marketplace()` (owned-name sale state + departure
    /// detection) on **every** registered wallet each sweep; shares the
    /// same `wallets` map as [`DashPaySyncManager`]. Not auto-started —
    /// call `start` after wallets are registered. See [`DpnsSyncManager`].
    pub(super) dpns_sync_manager: Arc<DpnsSyncManager>,
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
    /// own clones already, so `configure_shielded` is the only
    /// production reader of this retained handle — hence it is gated to
    /// `shielded`, plus `test` so the handler-wiring test can dispatch
    /// an event through the manager's own fan-out.
    #[cfg(any(test, feature = "shielded"))]
    pub(super) event_manager: Arc<PlatformEventManager>,
    pub(super) persister: Arc<P>,
    /// Tracked (wallet-independent) masternodes for this manager's
    /// network, keyed by wire proTxHash, plus the per-node gates that
    /// serialize their refreshes. Hydrated from the persister at
    /// `load_from_persistor`; every mutation writes the whole set back
    /// (see `masternode::tracked`).
    pub(crate) tracked_masternodes:
        std::sync::Arc<crate::masternode::tracked::TrackedMasternodeRegistry>,
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
    /// (and never cleared for this manager instance's lifetime) by the
    /// wallet-event adapter the first time it freezes a durable watermark
    /// after a persistence `store()` rejection — the one remaining fault
    /// trigger; the lossless persistence channel cannot drop or lag events.
    /// Poll via [`Self::sync_fault_detected`] to surface a "verification
    /// failed / rescan pending" state rather than re-freezing silently on
    /// the next launch.
    pub(super) sync_fault: Arc<std::sync::atomic::AtomicBool>,
    /// Per-WALLET in-broadcast fence maps, handed to every
    /// [`WalletGeneration`](crate::wallet::core::WalletGeneration) registered
    /// under each id (`dashpay/platform#4309`, review round 8).
    ///
    /// A fence describes a signed transaction that may be live on the network.
    /// That fact outlives the wallet *instance* that dispatched it: removing a
    /// wallet and re-creating it under the same id used to mint a generation
    /// with an empty map, so the re-created wallet restored the persisted UTXO
    /// with nothing holding it — not the fence, not key-wallet's memory-only
    /// reservation — and could sign a conflicting spend of an outpoint the
    /// original transaction still spends. Keying the map here instead makes the
    /// replacement inherit it.
    ///
    /// **Deliberately never pruned.** A removed wallet's entry stays, because a
    /// removal is exactly when the protection must survive; dropping it on
    /// removal would restore the bug for the recreate-after-remove path this
    /// exists to close. Growth is bounded by the number of distinct wallet ids
    /// this process has registered, and each entry reaps its own cleared rows
    /// on read.
    ///
    /// A `std::sync::Mutex`: touched only at wallet registration and load, for
    /// one map lookup, and never held across an await.
    pub(super) in_broadcast_fences: std::sync::Mutex<
        std::collections::BTreeMap<WalletId, Arc<crate::wallet::core::InBroadcastFences>>,
    >,
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
        // Take the manager's lossless, unbounded persistence receiver BEFORE
        // the manager is wrapped in the shared `Arc<RwLock>` and handed to any
        // producer. Unlike the old broadcast subscription, an
        // `mpsc::UnboundedReceiver` buffers events emitted during startup
        // rather than dropping them, so there is no subscribe-before-publish
        // race and — being unbounded — it can never `Lagged` and freeze the
        // durable sync watermark (dashpay/platform#4069). The receiver is
        // taken here, once, and moved into the adapter task below.
        let mut wallet_manager_inner = WalletManager::new(sdk.network);
        let event_receiver = wallet_manager_inner
            .take_persistence_receiver()
            .expect("persistence receiver is available exactly once on a fresh WalletManager");
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
        // SpendObservationHandler releases in-broadcast input fences when the
        // wallet observes the fenced outpoints spent — the evidence that ends
        // the fence a dispatch installs (`dashpay/platform#4309`). It takes the
        // same `wallets` map, and for the same lock reason as the balance
        // handler: the event fires inside SPV's block-processing write section,
        // so the generation cannot be resolved through the wallet-manager lock.
        let spend_observation_handler =
            Arc::new(SpendObservationHandler::new(Arc::clone(&wallets)));
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
            spend_observation_handler,
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
        // DPNS marketplace sync also sweeps the `wallets` map; it takes
        // the event manager to dispatch its pass-completion event.
        let dpns_sync = Arc::new(DpnsSyncManager::new(
            Arc::clone(&wallets),
            Arc::clone(&registry),
            Arc::clone(&event_manager),
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
            dpns_sync_manager: dpns_sync,
            dashpay_payment_handler,
            #[cfg(feature = "shielded")]
            shielded_sync_manager: shielded_sync,
            #[cfg(feature = "shielded")]
            shielded_coordinator,
            #[cfg(any(test, feature = "shielded"))]
            event_manager,
            persister,
            tracked_masternodes: std::sync::Arc::new(Default::default()),
            event_adapter_cancel,
            event_adapter_join: tokio::sync::Mutex::new(Some(event_adapter_join)),
            registry,
            sync_fault,
            in_broadcast_fences: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    /// The in-broadcast fence map for `wallet_id`, creating it on first use.
    ///
    /// Every [`WalletGeneration`](crate::wallet::core::WalletGeneration) this
    /// manager mints for a wallet is built from this, so a generation that
    /// replaces another under the same id inherits its pending-spend fences —
    /// see the [`in_broadcast_fences`](Self#structfield.in_broadcast_fences)
    /// field docs (`dashpay/platform#4309`).
    pub(super) fn in_broadcast_fences_for(
        &self,
        wallet_id: &WalletId,
    ) -> Arc<crate::wallet::core::InBroadcastFences> {
        Arc::clone(
            self.in_broadcast_fences
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .entry(*wallet_id)
                .or_default(),
        )
    }

    /// Whether the wallet-event adapter has frozen a durable sync
    /// watermark this manager's lifetime (dashpay/platform#4069).
    ///
    /// Returns `true` once — and stays `true` for THIS manager instance's
    /// lifetime (a destroyed-and-recreated manager starts unlatched) —
    /// after a persistence `store()` was rejected, the one remaining fault
    /// trigger: the lossless persistence channel cannot drop or lag events,
    /// so the old broadcast-lag trigger no longer exists. A latch means the
    /// persisted `syncedHeight` is deliberately held behind the chain tip
    /// for the affected wallet and a rescan is pending on the next launch.
    /// Integrators poll this to
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
        // Hold sync admission shut for the WHOLE quiesce -> wipe as well.
        // The clearing latch only bars registry (re)starts; a direct
        // `sync_now` / `sync_wallet` on a host thread does not consult it,
        // and a plain `quiesce()` reopens admission the instant it returns
        // — so such a pass could snapshot the old account set and refill
        // the commitment tree right after `coord.clear()` reset it. The
        // guard's Drop reopens admission on every exit path (`?`, panic).
        let Some(_quiesced) = self.shielded_sync_manager.quiesce_held().await else {
            // Fail closed: a pass is still holding `is_syncing` after the
            // drain budget, so wiping the store now would race its
            // persister fan-out. The host must NOT commit its own wipe.
            return Err(crate::error::PlatformWalletError::ShutdownIncomplete(
                "shielded sync pass did not drain within the quiesce budget; \
                 clear aborted — retry once sync is idle"
                    .to_string(),
            ));
        };
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
        // Same two-part exclusion as `clear_shielded`: the registry's
        // clearing latch bars a loop (re)start, and the held quiesce guard
        // bars a direct `sync_now` / `sync_wallet` for the whole
        // quiesce -> reset section. Both Drops run on every exit path.
        let _clearing = self
            .registry
            .hold_clearing(WalletWorker::PlatformAddressSync);
        let Some(_quiesced) = self.platform_address_sync_manager.quiesce_held().await else {
            // Fail closed, mirroring `clear_shielded`: resetting the
            // watermark while a wedged pass still holds `is_syncing`
            // would let its tail re-write the state this reset clears.
            return Err(crate::error::PlatformWalletError::ShutdownIncomplete(
                "platform-address sync pass did not drain within the quiesce budget; \
                 reset aborted — retry once sync is idle"
                    .to_string(),
            ));
        };

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
    /// **Every phase is bounded.** SPV stop owns its own abort-escalating
    /// join; the payment-hook drain is bounded by `PAYMENT_DRAIN_BUDGET`;
    /// the coordinator drains run concurrently under
    /// `COORDINATOR_DRAIN_BUDGET`; the registry join uses each worker's
    /// join budget; the adapter join is bounded too (its live handle is
    /// re-parked on timeout so a retry re-joins it). A wedged await
    /// therefore surfaces as a non-clean report instead of hanging the
    /// FFI's `destroy` forever.
    ///
    /// Returns a [`ShutdownReport`] keyed by [`WalletWorker`] — including
    /// the non-registry workers [`WalletWorker::Spv`],
    /// [`WalletWorker::DashPayPayments`], and
    /// [`WalletWorker::EventAdapter`], so no callback-capable background
    /// work is excluded from the verdict. Inspect
    /// [`ShutdownReport::all_clean`] before freeing the host callback
    /// context. A non-clean status flags a still-live worker or orphan.
    ///
    /// [`WorkerStatus`]: dash_async::WorkerStatus
    pub async fn shutdown(&self) -> ShutdownReport<WalletWorker> {
        // SPV first: it is the event source feeding everything below, and
        // its `stop` owns a bounded, abort-escalating join of the run-loop
        // task. Its outcome lands in the report — a failed stop must not
        // hide behind a clean coordinator join.
        let spv_status = match self.spv_manager.stop().await {
            Ok(()) => WorkerStatus::Ok,
            Err(error) => {
                tracing::warn!(?error, "SPV shutdown failed");
                WorkerStatus::Error(error.to_string())
            }
        };

        // Close payment-hook admission and join the admitted tasks —
        // they clone the FFI persister, so a straggler is exactly the
        // callback-after-destroy hazard the report exists to catch.
        let payments_drained = self
            .dashpay_payment_handler
            .quiesce_within(PAYMENT_DRAIN_BUDGET)
            .await;

        // Drain the coordinators concurrently against one shared budget so
        // the drain phase as a whole is bounded (a wedged pass surfaces as
        // `Timeout` in the report instead of hanging destroy forever).
        //
        // `_sealed_` (not plain `quiesce_within`): shutdown is terminal, so
        // sync admission must NOT reopen when the drain returns. The FFI
        // resolves the manager under a shared read guard, so a `sync_now`
        // dispatched on a host thread can still be between its slot CAS and
        // its gate check while `destroy` runs; a reopened gate would let it
        // run a full pass — and fire persister / completion callbacks —
        // after `destroy` returned and the host freed those contexts.
        #[cfg(feature = "shielded")]
        let (pa_drained, id_drained, dp_drained, dpns_drained, sh_drained) = tokio::join!(
            self.platform_address_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.identity_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.dashpay_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.dpns_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.shielded_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
        );
        #[cfg(not(feature = "shielded"))]
        let (pa_drained, id_drained, dp_drained, dpns_drained) = tokio::join!(
            self.platform_address_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.identity_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.dashpay_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
            self.dpns_sync_manager
                .quiesce_sealed_within(COORDINATOR_DRAIN_BUDGET),
        );

        // Hard-join the coordinator loop threads now that every in-flight
        // pass has drained. This is the barrier `quiesce` cannot give:
        // it waits for the actual OS thread to terminate before the host
        // drops the runtime.
        let mut report = self.registry.shutdown().await;

        // Fold drain timeouts in. A timed-out drain means a pass —
        // possibly a *direct* `sync_now` running on a host FFI thread the
        // registry never sees — may still hold `is_syncing` and fire
        // persister callbacks, so a clean registry join must not mask it.
        let drains = [
            (WalletWorker::PlatformAddressSync, pa_drained),
            (WalletWorker::IdentitySync, id_drained),
            (WalletWorker::DashPaySync, dp_drained),
            (WalletWorker::DpnsSync, dpns_drained),
            #[cfg(feature = "shielded")]
            (WalletWorker::ShieldedSync, sh_drained),
        ];
        for (worker, drained) in drains {
            if !drained {
                let status = report
                    .per_worker
                    .entry(worker)
                    .or_insert(WorkerStatus::Timeout);
                if status.is_clean() {
                    *status = WorkerStatus::Timeout;
                }
            }
        }

        report.per_worker.insert(WalletWorker::Spv, spv_status);
        report.per_worker.insert(
            WalletWorker::DashPayPayments,
            if payments_drained {
                WorkerStatus::Ok
            } else {
                WorkerStatus::Timeout
            },
        );

        // The wallet-event adapter is the sink the coordinators' stores
        // feed into, so it drains AFTER them. It is a plain tokio task,
        // not a registry worker; on a join timeout the live handle is
        // re-parked so a destroy retry can re-join it rather than
        // silently detaching the task.
        self.event_adapter_cancel.cancel();
        let adapter_status = {
            let mut slot = self.event_adapter_join.lock().await;
            match slot.take() {
                None => WorkerStatus::NotRunning,
                Some(mut handle) => {
                    match tokio::time::timeout(EVENT_ADAPTER_JOIN_BUDGET, &mut handle).await {
                        Ok(Ok(())) => WorkerStatus::Ok,
                        Ok(Err(e)) if e.is_panic() => WorkerStatus::Panicked(e.to_string()),
                        Ok(Err(e)) => WorkerStatus::Stopped(Some(e.to_string())),
                        Err(_) => {
                            tracing::warn!(
                                "wallet event adapter did not join within {:?}; re-parking",
                                EVENT_ADAPTER_JOIN_BUDGET
                            );
                            *slot = Some(handle);
                            WorkerStatus::Timeout
                        }
                    }
                }
            }
        };
        report
            .per_worker
            .insert(WalletWorker::EventAdapter, adapter_status);

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

    /// The constructor must register [`SpendObservationHandler`] on the event
    /// fan-out, over the LIVE wallets map (`dashpay/platform#4309`, review
    /// round 6): a spend-bearing wallet event dispatched through the manager's
    /// own `event_manager` must release a registered wallet's in-broadcast
    /// fence. Dropping the handler from the constructor's handler list — the
    /// accidental-omission regression this pins — fails the final assertion,
    /// because nothing else on the fan-out calls `observe_spent`.
    #[tokio::test]
    async fn constructor_wires_spend_observation_into_the_event_fanout() {
        use dashcore::hashes::Hash as _;

        let mgr = make_manager();

        // A funded wallet registered in the manager's live wallets map — the
        // same map the constructor handed to its handlers.
        let (wallet_manager, wallet_id, generation, _signer) =
            crate::test_support::funded_wallet_manager(
                key_wallet::account::account_type::StandardAccountType::BIP44Account,
            )
            .await;
        let spv = Arc::new(SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::new(PlatformEventManager::new(Vec::new())),
        ));
        let wallet = Arc::new(PlatformWallet::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
            wallet_id,
            wallet_manager,
            Arc::clone(&generation),
            Arc::new(Notify::new()),
            Arc::new(NoopPersister) as Arc<dyn PlatformWalletPersistence>,
            Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
        ));
        mgr.wallets.write().await.insert(wallet_id, wallet);

        // Fence an outpoint the way a dispatch does: pin, then settle into the
        // pending-spend phase that only an observed spend may end.
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![dashcore::TxIn {
                previous_output: dashcore::OutPoint {
                    txid: dashcore::Txid::from_slice(&[9u8; 32]).expect("txid"),
                    vout: 0,
                },
                script_sig: dashcore::ScriptBuf::new(),
                sequence: 0xffff_ffff,
                witness: dashcore::Witness::new(),
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        generation.pin_in_broadcast(&tx).settle_pending_spend();
        assert!(
            generation.in_broadcast_conflict(&tx).is_some(),
            "the settled pin must leave the pending-spend fence up"
        );

        // The spend event, dispatched through the manager's OWN fan-out — not
        // a hand-built handler — so the assertion covers registration itself.
        mgr.event_manager
            .on_wallet_event(&crate::test_support::observed_spend_event(wallet_id, &tx));

        assert!(
            generation.in_broadcast_conflict(&tx).is_none(),
            "a spend event through the manager's event fan-out must release \
             the registered wallet's fence — is SpendObservationHandler still \
             in the constructor's handler list?"
        );
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
        // The verdict must also cover the non-registry workers: SPV, the
        // DashPay payment-hook tracker, and the wallet-event adapter. A
        // report that omitted them could pass `all_clean()` while
        // callback-capable background work stayed live.
        for worker in [
            WalletWorker::Spv,
            WalletWorker::DashPayPayments,
            WalletWorker::EventAdapter,
        ] {
            assert!(
                report
                    .per_worker
                    .get(&worker)
                    .is_some_and(WorkerStatus::is_clean),
                "{worker:?} must be present and clean in the report: {report:?}"
            );
        }

        // Second shutdown: the coordinators already joined, so the registry
        // reports them NotRunning and the report stays clean.
        let again = mgr.shutdown().await;
        assert!(again.all_clean(), "idempotent shutdown: {again:?}");
    }

    /// `reset_platform_address_sync_state` must fail closed when the
    /// in-flight pass does not drain: resetting watermarks and balances
    /// under a live pass would let that pass's tail re-persist the state
    /// the reset just cleared.
    ///
    /// It must also leave no lifecycle latch stuck on the failure path —
    /// the registry's clearing latch is released by its guard's `Drop`, so
    /// a later retry (or a normal `start`) is not permanently barred.
    #[tokio::test(start_paused = true)]
    async fn reset_platform_address_state_fails_closed_on_a_wedged_pass() {
        let mgr = make_manager();

        // Wedge a pass: take the slot and never release it, as a pass stuck
        // in a network / persister await would.
        assert!(mgr.platform_address_sync().wedge_sync_slot_for_test());

        let error = tokio::time::timeout(
            Duration::from_secs(30),
            mgr.reset_platform_address_sync_state(),
        )
        .await
        .expect("the reset must be bounded by the drain budget, not hang")
        .expect_err("a wedged pass must abort the reset");
        assert!(
            matches!(
                error,
                crate::error::PlatformWalletError::ShutdownIncomplete(_)
            ),
            "expected ShutdownIncomplete so the FFI surfaces the typed code, got {error:?}"
        );

        assert!(
            !mgr.registry.is_clearing(WalletWorker::PlatformAddressSync),
            "the clearing latch must be released on the failure path"
        );
    }

    /// A concurrent holder's drop must NOT reopen admission while another
    /// drain is still in flight.
    ///
    /// RED against the close-then-hold-at-the-end gate: drain B closed the
    /// gate but only became a *holder* after its final `is_syncing`
    /// observation, so holder A dropping in that window stored
    /// `closed = false` — a direct `sync_now` could then claim the slot,
    /// pass the gate check, and run a full pass that B's caller (a
    /// clear/reset about to wipe state) believed was impossible. With the
    /// hold taken at drain entry, the gate is closed continuously from
    /// B's first instruction to its guard's drop.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_drain_keeps_gate_closed_across_another_holders_drop() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let gate = Arc::new(QuiesceGate::default());
        let is_syncing = Arc::new(AtomicBool::new(false));

        // Holder A: a completed drain (idle coordinator) holding its guard.
        let guard_a = drain_pass(&gate, &is_syncing, || {}, Duration::from_secs(1))
            .await
            .expect("idle drain must succeed");

        // Drain B: in flight against a wedged pass, parked in its poll
        // loop. The guard cannot cross the task boundary (it borrows the
        // task-local gate Arc), so B holds it in-task and is driven over
        // channels.
        is_syncing.store(true, Ordering::Release);
        let gate_b = Arc::clone(&gate);
        let is_syncing_b = Arc::clone(&is_syncing);
        let (b_drained_tx, b_drained_rx) = tokio::sync::oneshot::channel::<bool>();
        let (b_release_tx, b_release_rx) = tokio::sync::oneshot::channel::<()>();
        let b = tokio::spawn(async move {
            let guard = drain_pass(&gate_b, &is_syncing_b, || {}, Duration::from_secs(5)).await;
            let _ = b_drained_tx.send(guard.is_some());
            // Keep the guard held (B's caller "is mutating") until driven.
            let _ = b_release_rx.await;
            drop(guard);
        });
        // Let B take its entry hold and enter the poll loop.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // A finishes its mutation and drops. The gate must STAY closed:
        // B's drain is still deciding whether the pass has drained.
        drop(guard_a);
        assert!(
            gate.is_closed(),
            "a holder's drop must not reopen admission while a drain is in flight"
        );

        // Release the wedge; B's drain completes and its guard keeps the
        // gate closed until B's caller is done mutating.
        is_syncing.store(false, Ordering::Release);
        let b_drained = tokio::time::timeout(Duration::from_secs(2), b_drained_rx)
            .await
            .expect("drain B must complete once the pass drains")
            .expect("channel");
        assert!(b_drained, "drain B must succeed");
        assert!(gate.is_closed());

        b_release_tx.send(()).expect("drive B's guard drop");
        tokio::time::timeout(Duration::from_secs(2), b)
            .await
            .expect("B must finish")
            .expect("join");
        assert!(!gate.is_closed(), "last hold gone — admission restored");
    }

    /// The timeout latch composes with the entry-hold: a timed-out drain
    /// leaves the gate closed even though its own hold is gone, and only
    /// a later successful drain clears the latch.
    #[tokio::test(start_paused = true)]
    async fn timed_out_drain_latches_gate_closed_until_a_successful_drain() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let gate = QuiesceGate::default();
        let is_syncing = AtomicBool::new(true);

        assert!(
            drain_pass(&gate, &is_syncing, || {}, Duration::from_millis(50))
                .await
                .is_none(),
            "a wedged pass must time the drain out"
        );
        assert!(gate.is_closed(), "timed-out drain leaves the gate latched");

        // The wedge clears; the next drain succeeds, clears the latch, and
        // its guard's drop restores admission.
        is_syncing.store(false, Ordering::Release);
        let guard = drain_pass(&gate, &is_syncing, || {}, Duration::from_millis(50))
            .await
            .expect("drain must succeed once the pass drained");
        assert!(gate.is_closed());
        drop(guard);
        assert!(!gate.is_closed(), "successful drain clears the latch");
    }

    /// `SyncSlotGuard` must clear the `is_syncing` slot on panic unwind,
    /// not just on normal fall-through. Without this, a pass that panics
    /// leaves the flag latched and every subsequent `quiesce()` drain
    /// burns its full budget before reporting non-clean — turning one
    /// panicked pass into a permanently wedged (slow, never-clean)
    /// teardown.
    #[test]
    fn sync_slot_guard_clears_flag_on_panic_unwind() {
        let flag = std::sync::atomic::AtomicBool::new(true);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _slot = SyncSlotGuard(&flag);
            panic!("pass body panicked");
        }));
        assert!(result.is_err(), "the pass body must have panicked");
        assert!(
            !flag.load(std::sync::atomic::Ordering::Acquire),
            "guard must clear the slot during unwind"
        );
    }
}
