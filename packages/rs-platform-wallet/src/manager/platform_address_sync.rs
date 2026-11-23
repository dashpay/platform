//! Periodic platform-address balance sync coordinator.
//!
//! Runs [`PlatformAddressWallet::sync_balances`] for every registered
//! wallet on a fixed cadence, and emits a summary event so UI and
//! persistence layers can react.
//!
//! Not auto-started. Call [`PlatformAddressSyncManager::start`] once the
//! wallets are registered and the SPV runtime is up.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::{ArcSwap, ArcSwapOption};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use key_wallet::PlatformP2PKHAddress;

use crate::wallet::PlatformAddressTag;

use dash_async::ThreadRegistry;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::manager::sync_loop::{start_sync_loop, Admission, SyncCoordinator, SyncLoop};
use crate::manager::{QuiesceGuard, WalletWorker, COORDINATOR_DRAIN_BUDGET};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Default cadence.
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 15;

/// Outcome of syncing a single wallet in a pass.
///
/// Not `Clone` because `AddressSyncResult` isn't. Consumers receive it
/// by reference through the event-manager dispatch and are expected to
/// read whatever fields they need without holding onto the value.
#[derive(Debug)]
pub enum WalletSyncOutcome {
    /// Combined sync result across every account on the wallet. The
    /// unified provider performs one trunk/branch scan and returns a
    /// single result per wallet.
    Ok(AddressSyncResult<PlatformAddressTag, PlatformP2PKHAddress>),
    /// Error message from the failed sync.
    Err(String),
}

impl WalletSyncOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, WalletSyncOutcome::Ok(_))
    }
}

/// Summary of one full pass across every registered wallet.
#[derive(Debug, Default)]
pub struct PlatformAddressSyncSummary {
    /// Per-wallet outcomes keyed by `WalletId`.
    pub wallet_results: BTreeMap<WalletId, WalletSyncOutcome>,
    /// Unix seconds at which the pass completed. `0` means "no pass ran"
    /// (e.g. a concurrent pass was already in flight and we skipped).
    pub sync_unix_seconds: u64,
}

impl PlatformAddressSyncSummary {
    pub fn is_empty(&self) -> bool {
        self.wallet_results.is_empty()
    }

    pub fn success_count(&self) -> usize {
        self.wallet_results.values().filter(|o| o.is_ok()).count()
    }

    pub fn error_count(&self) -> usize {
        self.wallet_results.len() - self.success_count()
    }
}

/// Periodic platform-address balance sync coordinator.
///
/// Holds a handle to the same `wallets` map owned by
/// [`PlatformWalletManager`] (via `Arc`), so wallets added after `start`
/// are picked up on the next tick without any re-registration.
///
/// Each pass:
/// 1. Snapshots the wallet map (short read lock, no await while held).
/// 2. Calls [`PlatformAddressWallet::sync_balances`] on each wallet,
///    using the shared `config`.
/// 3. Stores the pass timestamp.
/// 4. Dispatches [`PlatformEventManager::on_platform_address_sync_completed`].
///
/// `sync_now` is re-entrant-safe: if a pass is already running, calling
/// `sync_now` again returns an empty summary immediately (the caller can
/// check `is_syncing()` to distinguish).
pub struct PlatformAddressSyncManager {
    wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    event_manager: Arc<PlatformEventManager>,
    /// The lifecycle shared with the sibling coordinators: the
    /// registry-owned loop thread, the `is_syncing` slot, the last-pass
    /// timestamp, and the admission gate. `sync_now` / `sync_wallet` bail
    /// (after taking the slot) when the gate is closed — by a
    /// [`quiesce`](Self::quiesce) draining an in-flight pass, by a
    /// [`QuiesceGuard`] holder mutating state, or terminally once shutdown
    /// seals it — so once a drain observes `is_syncing == false` no
    /// further pass can start, giving shutdown a real "no more
    /// host-visible sync-completed callbacks" barrier that cancel-only
    /// [`stop`](Self::stop) does not provide.
    sync: SyncLoop,
    /// Shared config applied uniformly across wallets and accounts.
    ///
    /// `ArcSwapOption` instead of a mutex because writes are rare
    /// (user changes sync config), reads fire every pass, and the
    /// loop's async state machine dislikes holding a `MutexGuard`.
    config: ArcSwapOption<AddressSyncConfig>,
}

impl PlatformAddressSyncManager {
    pub fn new(
        wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_manager: Arc<PlatformEventManager>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            wallets,
            event_manager,
            sync: SyncLoop::new(
                WalletWorker::PlatformAddressSync,
                registry,
                DEFAULT_SYNC_INTERVAL_SECS,
            ),
            config: ArcSwapOption::empty(),
        }
    }

    /// Set the polling interval. Clamped to a minimum of 1s.
    ///
    /// The running loop picks this up on its next sleep.
    pub fn set_interval(&self, interval: Duration) {
        self.sync.set_interval(interval);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        self.sync.interval()
    }

    /// Replace the shared [`AddressSyncConfig`] used on every pass.
    ///
    /// `None` means "use SDK defaults" (what the old iOS path did).
    pub fn set_config(&self, config: Option<AddressSyncConfig>) {
        self.config.store(config.map(Arc::new));
    }

    /// Snapshot the current config (cheap — an atomic pointer load).
    fn current_config(&self) -> Option<AddressSyncConfig> {
        self.config.load().as_ref().map(|arc| (**arc).clone())
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.sync.is_running()
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.sync.is_syncing()
    }

    /// Unix seconds of the last completed pass, or `None` if no pass
    /// has ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        self.sync.last_sync_unix_seconds()
    }

    /// Start the background sync loop. Idempotent — calling while
    /// already running is a no-op.
    ///
    /// The loop runs on a dedicated OS thread, not on a tokio worker.
    /// This is forced on us by the fact that
    /// [`Sdk::sync_address_balances`](dash_sdk::Sdk::sync_address_balances)
    /// returns a `!Send` future (the GRPC client state inside the SDK
    /// isn't `Send + Sync`), so it can't ride on `tokio::spawn`, which
    /// demands `Future: Send + 'static`. We use [`Handle::block_on`] so
    /// the future still has access to the main runtime's reactor for
    /// network I/O — only the polling thread is dedicated.
    ///
    /// The first pass runs immediately; subsequent passes fire every
    /// [`interval`](Self::interval).
    ///
    /// **Blocks briefly on restart**: the shared registry synchronously
    /// reaps a still-draining prior-generation thread, spinning up to the
    /// registry reap backstop (default 1 s) before returning. Call it from
    /// the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        // The registry owns the whole lifecycle (see `start_sync_loop`): it
        // takes the teardown latch, installs the cancellation token, spawns
        // the thread, and reaps any prior generation under one slot lock.
        start_sync_loop(self, None, "platform-address");
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: requests cancellation and returns immediately. A
    /// pass already inside `sync_now` keeps running to completion,
    /// including its `on_platform_address_sync_completed` host-callback
    /// dispatch. For a real "nothing is running and nothing more will
    /// fire a host callback" barrier — required by manager shutdown so
    /// the host can free the event-handler context — use
    /// [`quiesce`](Self::quiesce).
    pub fn stop(&self) {
        self.sync.cancel_loop();
    }

    /// Cancel the background loop **and wait for any in-flight sync pass
    /// to fully drain** before returning — a real quiescence barrier,
    /// unlike cancel-only [`stop`](Self::stop).
    ///
    /// After this returns, no sync pass is running and none can start
    /// until the next [`start`](Self::start) / `sync_now`, so a caller
    /// that immediately tears the manager down (and frees the host-owned
    /// event-handler context the FFI handed to us) cannot be raced by a
    /// pass that fires `on_platform_address_sync_completed` through a
    /// now-dangling pointer.
    ///
    /// Mechanism: close the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the completion-event dispatch (`sync_now` clears it only
    /// after `on_platform_address_sync_completed` returns), so its
    /// falling edge (with the gate closed) is a sound "fully drained" signal.
    /// The gate is reopened before returning so a later start/sync works
    /// normally.
    ///
    /// **Bounded** by `COORDINATOR_DRAIN_BUDGET`: returns `false` if
    /// the in-flight pass did not drain in time — see
    /// `quiesce_within` for the timeout contract.
    #[must_use = "a false return means the pass did NOT drain; the caller must fail closed"]
    pub async fn quiesce(&self) -> bool {
        self.quiesce_within(COORDINATOR_DRAIN_BUDGET).await
    }

    /// [`quiesce`](Self::quiesce) with an explicit drain budget.
    ///
    /// Returns `true` when the drain completed. Returns `false` when
    /// `is_syncing` was still held at the deadline — a wedged pass. On
    /// that path the `quiescing` gate is deliberately **left closed** so the
    /// wedged pass cannot be followed by a fresh one; the caller must
    /// treat the coordinator as non-clean. A later successful `quiesce`
    /// reopens the gate, as does [`start`](Self::start), the running
    /// loop, or a one-shot `sync_now`, once the wedged pass has released
    /// `is_syncing`.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        // The guard drops here, reopening the gate — this is the
        // "drain only" flavor.
        self.quiesce_held_within(budget).await.is_some()
    }

    /// [`quiesce`](Self::quiesce) that **keeps sync admission shut** until
    /// the returned guard drops.
    ///
    /// `quiesce()` alone reopens the gate the instant it returns, so a
    /// caller that then mutates state a pass touches (`reset_platform_address_sync_state`'s watermark + balance reset) runs its
    /// mutation with admission already re-opened — a direct pass on a host
    /// thread can snapshot pre-mutation state and re-persist it right
    /// after. Holding the guard across the whole quiesce → mutate section
    /// closes that window.
    ///
    /// `None` means the in-flight pass did not drain within
    /// `COORDINATOR_DRAIN_BUDGET`; the caller must fail closed.
    #[must_use = "None means the pass did NOT drain; the caller must fail closed"]
    pub(crate) async fn quiesce_held(&self) -> Option<QuiesceGuard<'_>> {
        self.quiesce_held_within(COORDINATOR_DRAIN_BUDGET).await
    }

    /// [`quiesce_held`](Self::quiesce_held) with an explicit drain budget.
    pub(crate) async fn quiesce_held_within(&self, budget: Duration) -> Option<QuiesceGuard<'_>> {
        self.sync.drain(|| self.stop(), budget).await
    }

    /// [`quiesce_within`](Self::quiesce_within) that **seals** the gate:
    /// admission never reopens on this coordinator instance.
    ///
    /// Used by manager shutdown. Reopening there would let a direct
    /// `sync_now` that was already dispatched on a host thread — the FFI
    /// resolves the manager under a shared read guard, so it can be
    /// mid-flight while `destroy` runs — start a fresh pass *after* the
    /// drain concluded and fire persister / completion callbacks through
    /// a context the host has since freed.
    pub(crate) async fn quiesce_sealed_within(&self, budget: Duration) -> bool {
        // Seals before the drain's guard drops, so its Drop cannot reopen.
        self.sync.drain_sealed(|| self.stop(), budget).await
    }

    /// Run one sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    pub async fn sync_now(&self) -> PlatformAddressSyncSummary {
        // Bail without running a pass when another one holds the slot, or
        // when a `quiesce()` has closed the gate — so the drain can complete
        // and shutdown gets a true barrier (no further
        // `on_platform_address_sync_completed` host callback after quiesce
        // returns). The permit holds `is_syncing` until it drops at the end
        // of this function, on panic unwind too, so a failed pass can never
        // wedge `quiesce()`'s drain.
        let Admission::Admitted(_permit) = self.sync.admit() else {
            return PlatformAddressSyncSummary::default();
        };

        let snapshot: Vec<(WalletId, Arc<PlatformWallet>)> = {
            let wallets = self.wallets.load();
            wallets.iter().map(|(id, w)| (*id, Arc::clone(w))).collect()
        };

        let config = self.current_config();

        let mut summary = PlatformAddressSyncSummary::default();
        for (wallet_id, wallet) in snapshot {
            let outcome = match wallet.platform().sync_balances(config.clone()).await {
                Ok(result) => WalletSyncOutcome::Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "Platform address sync failed for wallet {}: {}",
                        hex::encode(wallet_id),
                        e
                    );
                    WalletSyncOutcome::Err(e.to_string())
                }
            };
            summary.wallet_results.insert(wallet_id, outcome);
        }

        let now = SyncLoop::unix_now();
        summary.sync_unix_seconds = now;
        self.sync.record_completed(now);

        // Dispatch the completion event BEFORE clearing `is_syncing`.
        // `quiesce()` drains on the falling edge of `is_syncing`, so if
        // we cleared the flag first a shutdown caller could unblock and
        // free the host event-handler context while this completion
        // event (FFI callback → host handler) is still pending — a
        // use-after-free. Holding the flag across the dispatch makes
        // quiesce's barrier cover the host callback too (`_permit` drops —
        // and clears the flag — only after this dispatch returns).
        // Mirrors the ordering in `ShieldedSyncManager::sync_now`.
        self.event_manager
            .on_platform_address_sync_completed(&summary);

        summary
    }

    /// Test-only: claim the `is_syncing` slot and never release it,
    /// standing in for a pass wedged in a network / persister await.
    /// Returns `false` if the slot was already taken.
    #[cfg(test)]
    pub(crate) fn wedge_sync_slot_for_test(&self) -> bool {
        self.sync.wedge_slot_for_test()
    }

    /// Test-only: release a slot taken by
    /// [`wedge_sync_slot_for_test`](Self::wedge_sync_slot_for_test), as the
    /// wedged pass finally draining would.
    #[cfg(test)]
    pub(crate) fn release_sync_slot_for_test(&self) {
        self.sync.release_slot_for_test();
    }

    /// Test-only: whether new sync passes are currently barred — a drain in
    /// flight, a latched timeout, or the terminal seal `shutdown` applies.
    #[cfg(test)]
    pub(crate) fn sync_admission_closed(&self) -> bool {
        self.sync.admission_closed()
    }

    /// Sync a single wallet on demand.
    ///
    /// Goes through the same admission as [`sync_now`](Self::sync_now) —
    /// claim the manager-wide `is_syncing` slot, then honor the quiescing
    /// gate — so a per-wallet sync is covered by the drain barrier too.
    /// Without that, a reset/teardown that only watched `is_syncing` could
    /// conclude "nothing is running" while this call was about to take a
    /// wallet's provider lock and persist a fresh watermark over the state
    /// just cleared.
    ///
    /// Returns [`PlatformWalletError::AddressSync`] when another pass holds
    /// the slot or admission is shut (a reset/Clear is mutating, or the
    /// manager is shutting down) — the caller should retry once sync is
    /// idle rather than treat it as a sync failure.
    pub async fn sync_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<AddressSyncResult<PlatformAddressTag, PlatformP2PKHAddress>, PlatformWalletError>
    {
        // Same admission as `sync_now`. The permit holds `is_syncing` until
        // it drops at the end of this function, on panic unwind too.
        let _permit = match self.sync.admit() {
            Admission::Admitted(permit) => permit,
            Admission::Busy => {
                return Err(PlatformWalletError::AddressSync(
                    "a platform-address sync pass is already in flight; retry once it completes"
                        .to_string(),
                ));
            }
            // A drain closed the gate; bail so it can complete and its
            // caller gets a true barrier.
            Admission::Closed => {
                return Err(PlatformWalletError::AddressSync(
                    "platform-address sync is quiescing; retry once the reset / teardown completes"
                        .to_string(),
                ));
            }
        };

        let wallet = {
            let wallets = self.wallets.load();
            wallets.get(wallet_id).cloned()
        };
        let wallet =
            wallet.ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;

        let config = self.current_config();
        wallet.platform().sync_balances(config).await
    }
}

impl SyncCoordinator for PlatformAddressSyncManager {
    fn sync_loop(&self) -> &SyncLoop {
        &self.sync
    }

    async fn background_pass(&self) {
        self.sync_now().await;
    }
}

impl std::fmt::Debug for PlatformAddressSyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressSyncManager")
            .field("is_running", &self.is_running())
            .field("is_syncing", &self.is_syncing())
            .field("interval_secs", &self.interval().as_secs())
            .field("last_sync_unix", &self.last_sync_unix_seconds())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::{OnceLock, Weak};

    use dash_spv::EventHandler;

    use crate::events::PlatformEventHandler;

    /// Event handler that just counts `on_platform_address_sync_completed`
    /// dispatches. Stands in for the host's FFI handler so we can assert
    /// the quiescing gate suppresses the completion callback. It also
    /// records how many dispatches arrived while the coordinator still
    /// held its `is_syncing` slot.
    struct CompletionCounter {
        completions: AtomicUsize,
        dispatched_with_slot_held: AtomicUsize,
        manager: OnceLock<Weak<PlatformAddressSyncManager>>,
    }

    impl CompletionCounter {
        fn new() -> Self {
            Self {
                completions: AtomicUsize::new(0),
                dispatched_with_slot_held: AtomicUsize::new(0),
                manager: OnceLock::new(),
            }
        }
    }

    impl EventHandler for CompletionCounter {}

    impl PlatformEventHandler for CompletionCounter {
        fn on_platform_address_sync_completed(&self, _summary: &PlatformAddressSyncSummary) {
            self.completions.fetch_add(1, AtomicOrdering::SeqCst);
            let slot_held = self
                .manager
                .get()
                .and_then(Weak::upgrade)
                .is_some_and(|manager| manager.is_syncing());
            if slot_held {
                self.dispatched_with_slot_held
                    .fetch_add(1, AtomicOrdering::SeqCst);
            }
        }
    }

    /// Build a manager over an empty wallet map wired to a completion
    /// counter. No wallets means `sync_now` runs zero per-wallet syncs
    /// but still drives the full flag → gate → completion-event protocol
    /// we're testing here.
    fn make_manager() -> (Arc<PlatformAddressSyncManager>, Arc<CompletionCounter>) {
        let (manager, counter, _registry) = make_manager_with_registry();
        (manager, counter)
    }

    /// [`make_manager`] plus the registry that owns the loop thread, for
    /// tests that join it.
    fn make_manager_with_registry() -> (
        Arc<PlatformAddressSyncManager>,
        Arc<CompletionCounter>,
        Arc<ThreadRegistry<WalletWorker>>,
    ) {
        let wallets = Arc::new(ArcSwap::from_pointee(BTreeMap::new()));
        let counter = Arc::new(CompletionCounter::new());
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            Arc::clone(&counter) as Arc<dyn PlatformEventHandler>
        ]));
        let registry = ThreadRegistry::<WalletWorker>::new();
        let manager = Arc::new(PlatformAddressSyncManager::new(
            wallets,
            event_manager,
            Arc::clone(&registry),
        ));
        counter
            .manager
            .set(Arc::downgrade(&manager))
            .expect("set once");
        (manager, counter, registry)
    }

    /// A normal pass (no gate) fires the completion event and leaves the
    /// flags clean. Baseline for the gated case below.
    #[tokio::test]
    async fn sync_now_fires_completion_when_not_quiescing() {
        let (mgr, counter) = make_manager();
        mgr.sync_now().await;
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 1);
        assert!(!mgr.is_syncing());
    }

    /// `quiesce()` must not return while a pass is in flight, and must
    /// return promptly once the pass drains.
    ///
    /// Drives the real `is_syncing` lifecycle: a background task takes
    /// the slot via the same `compare_exchange` the real `sync_now`
    /// uses, holds it across a sleep (standing in for the pass body +
    /// completion-event dispatch, which `sync_now` keeps the flag set
    /// across), then clears it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_blocks_until_in_flight_pass_drains() {
        let (mgr, _counter) = make_manager();

        let holder = Arc::clone(&mgr);
        let pass = tokio::spawn(async move {
            assert!(
                holder.wedge_sync_slot_for_test(),
                "test should own the is_syncing slot"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
            holder.release_sync_slot_for_test();
        });

        while !mgr.is_syncing() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let quiesce_fut = mgr.quiesce();
        tokio::pin!(quiesce_fut);

        tokio::select! {
            _ = &mut quiesce_fut => panic!("quiesce returned while a pass was in flight"),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
        assert!(mgr.is_syncing(), "pass should still be in flight");

        tokio::time::timeout(Duration::from_secs(2), &mut quiesce_fut)
            .await
            .expect("quiesce did not return after the pass drained");

        assert!(!mgr.sync_admission_closed());
        assert!(!mgr.is_syncing());
        pass.await.unwrap();
    }

    /// A pass that never drains must NOT hang `quiesce_within` forever:
    /// the drain returns `false` at its deadline and deliberately leaves
    /// the `quiescing` gate closed (so the wedged pass cannot be followed by
    /// a fresh one). A later successful quiesce reopens the gate. Sibling
    /// of the dashpay / identity regression tests.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_within_times_out_and_leaves_gate_up_when_pass_never_drains() {
        let (mgr, _counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());

        let drained = tokio::time::timeout(
            Duration::from_secs(2),
            mgr.quiesce_within(Duration::from_millis(100)),
        )
        .await
        .expect("bounded quiesce must return at its deadline");
        assert!(!drained, "a wedged pass must be reported as non-drained");
        assert!(
            mgr.sync_admission_closed(),
            "gate must stay up after a timed-out drain"
        );

        mgr.release_sync_slot_for_test();
        assert!(mgr.quiesce().await);
        assert!(!mgr.sync_admission_closed());
    }

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — in particular, without firing the
    /// `on_platform_address_sync_completed` host callback. This is the
    /// gate that prevents a pass from slipping in between `quiesce`'s
    /// `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let (mgr, counter) = make_manager();

        // Raise the gate as an in-flight `quiesce()` would (a drain holds
        // the gate from its first instruction).
        let gate_hold = mgr
            .quiesce_held()
            .await
            .expect("an idle coordinator must drain immediately");

        let summary = mgr.sync_now().await;

        // Empty summary, no host completion callback, slot released so a
        // later (post-quiesce) pass can still run.
        assert!(summary.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert!(!mgr.is_syncing());
        drop(gate_hold);
    }

    /// The barrier `reset_platform_address_sync_state` needs: admission
    /// must stay shut for as long as the caller holds the guard, not just
    /// until the drain returns.
    ///
    /// RED before the fix — `quiesce()` reopened the gate on return, so a
    /// direct `sync_now` on a host thread could run a full pass (and fire
    /// the completion callback) while the reset was still rewriting
    /// watermarks and balances, then persist state the reset had cleared.
    #[tokio::test]
    async fn quiesce_held_bars_passes_until_the_guard_drops() {
        let (mgr, counter) = make_manager();

        let guard = mgr
            .quiesce_held()
            .await
            .expect("an idle coordinator must drain immediately");

        // Stand-in for the mutation the holder performs (the per-wallet
        // reset): a pass dispatched concurrently must not run.
        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            0,
            "no pass — and so no host callback — may run while the guard is held"
        );

        drop(guard);

        // Admission is restored for ordinary use once the mutation is done.
        assert!(!mgr.sync_admission_closed());
        mgr.sync_now().await;
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 1);
    }

    /// Overlapping holders compose: the gate reopens only when the LAST
    /// guard drops. A per-guard boolean would reopen admission at the
    /// first drop while the outer holder was still mutating.
    #[tokio::test]
    async fn overlapping_quiesce_guards_reopen_only_after_the_last_drop() {
        let (mgr, _counter) = make_manager();

        let outer = mgr.quiesce_held().await.expect("drain");
        let inner = mgr.quiesce_held().await.expect("drain");

        drop(inner);
        assert!(
            mgr.sync_admission_closed(),
            "the outer holder is still mutating; admission must stay shut"
        );

        drop(outer);
        assert!(!mgr.sync_admission_closed());
    }

    /// Shutdown seals the gate: admission must never reopen, because the
    /// FFI frees the host callback context the moment `destroy` returns.
    ///
    /// A `sync_now` already dispatched on a host thread (the FFI resolves
    /// the manager under a shared read guard, so it can be mid-flight
    /// while `destroy` runs) must find the gate shut and bail rather than
    /// run a pass that fires callbacks into freed memory.
    #[tokio::test]
    async fn quiesce_sealed_never_reopens_admission() {
        let (mgr, counter) = make_manager();

        assert!(mgr.quiesce_sealed_within(Duration::from_secs(1)).await);
        assert!(mgr.sync_admission_closed(), "seal must leave the gate shut");

        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);

        // Not even an explicit drain reopens a sealed gate.
        assert!(mgr.quiesce().await);
        assert!(mgr.sync_admission_closed());
        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
    }

    /// `sync_wallet` is a second entry point into the same per-wallet
    /// state, so it must observe the same admission as `sync_now` —
    /// bypassing the `is_syncing` slot or the gate would let a per-wallet
    /// sync take a wallet's provider lock and persist a fresh watermark
    /// right after a reset cleared it.
    #[tokio::test]
    async fn sync_wallet_is_refused_while_admission_is_shut() {
        let (mgr, _counter) = make_manager();

        let _guard = mgr.quiesce_held().await.expect("drain");

        let error = mgr
            .sync_wallet(&[7u8; 32])
            .await
            .expect_err("a per-wallet sync must be refused while a reset holds the gate");
        assert!(
            matches!(error, PlatformWalletError::AddressSync(_)),
            "expected an AddressSync refusal, got {error:?}"
        );
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");
    }

    /// Regression for dashpay/platform#5222: a `quiesce` that timed out
    /// latches the gate, and before this fix a later `start()` spawned a
    /// loop whose every pass bailed on the closed gate — the host saw a
    /// running loop that never synced and never emitted a completion.
    /// Now the loop reopens the gate once the wedged pass has drained,
    /// and not a moment earlier.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_resumes_passes_once_a_timed_out_drains_pass_clears() {
        let (mgr, counter) = make_manager();

        // Wedge a pass, then let a bounded quiesce time out on it.
        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        assert!(
            mgr.sync_admission_closed(),
            "timed-out drain latches the gate"
        );

        // A restart behind the still-wedged pass must not run a pass. The
        // loop's first attempt loses the slot CAS to the wedge; it then
        // polls at `LATCHED_RETRY_INTERVAL`, not the 15 s default.
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            mgr.sync_admission_closed(),
            "gate stays latched while wedged"
        );
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            0,
            "no pass may run behind a wedged one"
        );

        // The wedged pass drains: the running loop reopens the gate and
        // its next pass completes.
        mgr.release_sync_slot_for_test();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions.load(AtomicOrdering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "loop never resumed passes after the wedged pass drained"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!mgr.sync_admission_closed(), "latch is cleared");

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// Same as above, but the wedged pass has already drained by the time
    /// `start()` runs: the start itself clears the latch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_reopens_a_latched_gate_when_the_wedged_pass_is_gone() {
        let (mgr, counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.release_sync_slot_for_test();
        assert!(
            mgr.sync_admission_closed(),
            "latch outlives the wedged pass"
        );

        Arc::clone(&mgr).start();
        assert!(!mgr.sync_admission_closed(), "start clears a stale latch");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions.load(AtomicOrdering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "loop never ran a pass after start cleared the latch"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// A one-shot `sync_now` with no loop running clears a latch whose
    /// wedged pass has drained, instead of bailing until a restart.
    #[tokio::test]
    async fn sync_now_reopens_a_drained_latch() {
        let (mgr, counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.sync_now().await;
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert!(mgr.sync_admission_closed(), "latch stays while wedged");

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            1,
            "a direct pass runs once the wedged pass is gone"
        );
        assert!(
            !mgr.sync_admission_closed(),
            "the direct pass cleared the latch"
        );
    }

    #[tokio::test]
    async fn should_round_trip_interval() {
        let (mgr, _counter) = make_manager();

        assert_eq!(
            mgr.interval(),
            Duration::from_secs(DEFAULT_SYNC_INTERVAL_SECS)
        );

        mgr.set_interval(Duration::from_secs(0));
        assert_eq!(mgr.interval(), Duration::from_secs(1));

        mgr.set_interval(Duration::from_secs(120));
        assert_eq!(mgr.interval(), Duration::from_secs(120));
    }

    /// A pass that ran stamps both the summary and the coordinator.
    #[tokio::test]
    async fn should_stamp_a_completed_pass() {
        let (mgr, _counter) = make_manager();
        assert_eq!(mgr.last_sync_unix_seconds(), None);

        let summary = mgr.sync_now().await;

        assert_ne!(summary.sync_unix_seconds, 0, "a pass that ran is stamped");
        assert_eq!(
            mgr.last_sync_unix_seconds(),
            Some(summary.sync_unix_seconds)
        );
    }

    /// The completion event reaches the host while the pass still holds
    /// `is_syncing`, so a drain waiting on the slot's falling edge also
    /// waits out the host callback.
    #[tokio::test]
    async fn should_dispatch_completion_before_releasing_the_slot() {
        let (mgr, counter) = make_manager();

        mgr.sync_now().await;

        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            counter
                .dispatched_with_slot_held
                .load(AtomicOrdering::SeqCst),
            1,
            "the completion must be dispatched with the slot still held"
        );
        assert!(!mgr.is_syncing(), "the slot is released after the dispatch");
    }

    /// A second `sync_now` while a pass holds the slot does nothing: empty
    /// unstamped summary, no event, and the slot it lost stays with its
    /// owner.
    #[tokio::test]
    async fn should_skip_sync_now_while_slot_is_held() {
        let (mgr, counter) = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());

        let summary = mgr.sync_now().await;

        assert!(summary.is_empty());
        assert_eq!(summary.sync_unix_seconds, 0, "0 means no pass ran");
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(
            mgr.is_syncing(),
            "the losing call must not release the in-flight pass's slot"
        );
        assert!(!mgr.sync_admission_closed());

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 1);
    }

    /// A refused `sync_now` behind a closed gate leaves no timestamp.
    #[tokio::test]
    async fn should_not_stamp_sync_now_refused_by_a_shut_gate() {
        let (mgr, _counter) = make_manager();
        let _guard = mgr.quiesce_held().await.expect("drain");

        let summary = mgr.sync_now().await;

        assert_eq!(summary.sync_unix_seconds, 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
    }

    /// `sync_wallet` behind an in-flight pass is refused with the
    /// "already in flight" message, and leaves that pass's slot alone.
    #[tokio::test]
    async fn should_return_busy_error_when_slot_is_held() {
        let (mgr, _counter) = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());

        let error = mgr
            .sync_wallet(&[7u8; 32])
            .await
            .expect_err("a per-wallet sync must be refused while a pass is in flight");

        match error {
            PlatformWalletError::AddressSync(message) => assert_eq!(
                message,
                "a platform-address sync pass is already in flight; retry once it completes"
            ),
            other => panic!("expected an AddressSync refusal, got {other:?}"),
        }
        assert!(
            mgr.is_syncing(),
            "the refused call must not release the in-flight pass's slot"
        );
    }

    /// `sync_wallet` behind a closed gate is refused with the "quiescing"
    /// message — distinct from the slot-busy one.
    #[tokio::test]
    async fn should_return_quiescing_error_when_admission_is_shut() {
        let (mgr, _counter) = make_manager();
        let _guard = mgr.quiesce_held().await.expect("drain");

        let error = mgr
            .sync_wallet(&[7u8; 32])
            .await
            .expect_err("a per-wallet sync must be refused while the gate is held");

        match error {
            PlatformWalletError::AddressSync(message) => assert_eq!(
                message,
                "platform-address sync is quiescing; retry once the reset / teardown completes"
            ),
            other => panic!("expected an AddressSync refusal, got {other:?}"),
        }
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");
    }

    /// An admitted `sync_wallet` for an unknown wallet reports it and
    /// releases the slot; it clears a drained latch on the way in, like
    /// `sync_now`.
    #[tokio::test]
    async fn should_admit_sync_wallet_and_release_the_slot_for_an_unknown_wallet() {
        let (mgr, counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.release_sync_slot_for_test();
        assert!(
            mgr.sync_admission_closed(),
            "latch outlives the wedged pass"
        );

        let error = mgr
            .sync_wallet(&[7u8; 32])
            .await
            .expect_err("the wallet is not registered");

        assert!(
            matches!(error, PlatformWalletError::WalletNotFound(_)),
            "expected WalletNotFound, got {error:?}"
        );
        assert!(!mgr.sync_admission_closed(), "the call cleared the latch");
        assert!(!mgr.is_syncing());
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            0,
            "a per-wallet sync fires no completion event"
        );
        assert_eq!(mgr.last_sync_unix_seconds(), None);
    }

    /// A sealed drain that times out still seals, and reports non-drained.
    #[tokio::test]
    async fn should_seal_admission_even_when_the_sealed_quiesce_times_out() {
        let (mgr, counter) = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());

        assert!(!mgr.quiesce_sealed_within(Duration::from_millis(50)).await);

        mgr.release_sync_slot_for_test();
        assert!(mgr.sync_now().await.is_empty());
        assert!(
            mgr.sync_admission_closed(),
            "a sealed gate survives the drained-latch reopen"
        );
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
    }

    /// The first pass runs as soon as the loop starts, not one interval
    /// later (the default interval is far longer than the wait here).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_run_first_pass_immediately_after_start() {
        let (mgr, counter) = make_manager();

        Arc::clone(&mgr).start();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions.load(AtomicOrdering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the loop never ran its first pass"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(mgr.last_sync_unix_seconds().is_some());

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// `start()` on a running loop is a no-op: it neither restarts the loop
    /// (which would run another immediate first pass) nor spawns a second.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_ignore_start_when_loop_is_running() {
        let (mgr, counter, registry) = make_manager_with_registry();

        Arc::clone(&mgr).start();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions.load(AtomicOrdering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the loop never ran its first pass"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            1,
            "a second start must not run another first pass"
        );

        mgr.stop();
        assert!(!mgr.is_running(), "one stop cancels the only loop");
        let report = registry.shutdown().await;
        assert!(report.all_clean(), "clean join: {report:?}");
    }

    /// A `stop()` followed at once by `start()` leaves the new loop
    /// running and cancellable, and both generations join cleanly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn should_keep_new_loop_cancellable_after_stop_then_quick_start() {
        let (mgr, _counter, registry) = make_manager_with_registry();

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        mgr.stop();
        assert!(!mgr.is_running(), "stop() clears the running flag at once");

        Arc::clone(&mgr).start();
        assert!(mgr.is_running(), "loop B must be running after the restart");

        mgr.stop();
        assert!(!mgr.is_running());
        let report = registry.shutdown().await;
        assert!(
            report.all_clean(),
            "clean shutdown after restart: {report:?}"
        );
    }
}
