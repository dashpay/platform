//! Periodic DPNS username-marketplace sync coordinator.
//!
//! Folds the marketplace refresh — owned-name sale state (`$price`),
//! newly acquired names, and names that LEFT an identity (sold /
//! transferred away) — into the recurring background loop, alongside the
//! platform-address, identity-token, DashPay, and shielded coordinators.
//! Before this, DPNS state only refreshed when the host explicitly
//! called an FFI sync entry point.
//!
//! **Wallet-driven, not registry-driven — by design.** A sibling of
//! [`DashPaySyncManager`](super::dashpay_sync::DashPaySyncManager): it
//! holds the same `wallets` map, snapshots the wallet `Arc`s from its
//! wait-free map each sweep, and refreshes **every** wallet. It is a
//! separate coordinator (not a seventh DashPay step) because the DashPay
//! pass is contact/profile-scoped and runs at a 15s cadence, while
//! marketplace state changes are rare — this loop defaults to 60s.
//!
//! The per-wallet refresh is one `IdentityWallet` domain operation,
//! [`sync_dpns_marketplace`](crate::wallet::identity::IdentityWallet::sync_dpns_marketplace)
//! (which also has a standalone on-demand FFI caller); the coordinator
//! owns only the sweep, the log-and-continue policy, and the completion
//! event dispatch.
//!
//! Each pass:
//! 1. Snapshots the wallet map (short read lock, no await while held).
//! 2. Runs `sync_dpns_marketplace()` per wallet (log-and-continue).
//! 3. Stores the pass timestamp and dispatches
//!    [`PlatformEventManager::on_dpns_marketplace_sync_completed`].
//!
//! `sync_now` is re-entrant-safe (an in-flight pass makes it return an
//! empty summary immediately) and shutdown drains an in-flight pass via
//! [`quiesce`](DpnsSyncManager::quiesce), exactly like the sibling
//! coordinators.
//!
//! Not auto-started. Call [`DpnsSyncManager::start`] once the wallets
//! are registered and the SDK is connected.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use dash_async::ThreadRegistry;

use crate::events::PlatformEventManager;
use crate::manager::sync_loop::{start_sync_loop, Admission, SyncCoordinator, SyncLoop};
use crate::manager::{QuiesceGuard, WalletWorker, COORDINATOR_DRAIN_BUDGET};
use crate::wallet::identity::network::DpnsMarketplaceSyncSummary;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Default cadence for the DPNS marketplace sync loop.
///
/// Marketplace state (listings, sales) changes far less often than
/// DashPay contact state, and each pass costs one indexed document query
/// per identity — 60s keeps sale/departure detection timely without
/// multiplying DAPI traffic. Tunable at runtime via
/// [`DpnsSyncManager::set_interval`].
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 60;

/// Stack size for the DPNS sync loop's OS thread.
///
/// The pass verifies GroveDB document-query proofs (domain-document and
/// history-contract fetches), whose recursive descent overflows the
/// platform default thread stack — same rationale and size as the
/// DashPay coordinator and the FFI worker convention (`runtime.rs`).
const DPNS_SYNC_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Outcome of syncing a single wallet's marketplace state in a pass.
#[derive(Debug)]
pub enum WalletDpnsSyncOutcome {
    /// `sync_dpns_marketplace()` completed; carries its per-wallet delta.
    Ok(DpnsMarketplaceSyncSummary),
    /// `sync_dpns_marketplace()` returned an error message (logged,
    /// non-fatal to the rest of the pass).
    Err(String),
}

impl WalletDpnsSyncOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, WalletDpnsSyncOutcome::Ok(_))
    }
}

/// Summary of one full DPNS marketplace sync pass across every
/// registered wallet.
#[derive(Debug, Default)]
pub struct DpnsSyncPassSummary {
    /// Per-wallet outcomes keyed by `WalletId`.
    pub wallet_results: BTreeMap<WalletId, WalletDpnsSyncOutcome>,
    /// Unix seconds at which the pass completed. `0` means "no pass ran"
    /// (a concurrent pass was already in flight and we skipped).
    pub sync_unix_seconds: u64,
}

impl DpnsSyncPassSummary {
    pub fn is_empty(&self) -> bool {
        self.wallet_results.is_empty()
    }

    pub fn success_count(&self) -> usize {
        self.wallet_results.values().filter(|o| o.is_ok()).count()
    }

    pub fn error_count(&self) -> usize {
        self.wallet_results.len() - self.success_count()
    }

    /// Whether any wallet reported a marketplace delta (names added,
    /// departed, or re-priced) this pass.
    pub fn has_delta(&self) -> bool {
        self.wallet_results.values().any(|o| match o {
            WalletDpnsSyncOutcome::Ok(s) => !s.is_empty_delta(),
            WalletDpnsSyncOutcome::Err(_) => false,
        })
    }
}

/// Periodic DPNS username-marketplace sync coordinator. See the module
/// docs for the design; the lifecycle (start / stop / quiesce semantics,
/// registry-owned thread, deep stack) mirrors
/// [`DashPaySyncManager`](super::dashpay_sync::DashPaySyncManager)
/// verbatim.
pub struct DpnsSyncManager {
    wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    /// Dispatches `on_dpns_marketplace_sync_completed` after each pass.
    events: Arc<PlatformEventManager>,
    /// Loop thread, `is_syncing` slot, admission gate and last-pass
    /// timestamp — the lifecycle shared with the sibling coordinators.
    sync: SyncLoop,
}

impl DpnsSyncManager {
    pub fn new(
        wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
        events: Arc<PlatformEventManager>,
    ) -> Self {
        Self {
            wallets,
            events,
            sync: SyncLoop::new(WalletWorker::DpnsSync, registry, DEFAULT_SYNC_INTERVAL_SECS),
        }
    }

    /// Set the polling interval. Clamped to a minimum of 1s. The running
    /// loop picks this up on its next sleep.
    pub fn set_interval(&self, interval: Duration) {
        self.sync.set_interval(interval);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        self.sync.interval()
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.sync.is_running()
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.sync.is_syncing()
    }

    /// Unix seconds of the last completed pass, or `None` if no pass has
    /// ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        self.sync.last_sync_unix_seconds()
    }

    /// Start the background sync loop. Idempotent — calling while
    /// already running is a no-op. Runs on a dedicated registry-owned OS
    /// thread with a deep stack, driving the (`!Send`) SDK futures via
    /// `Handle::block_on` — same mechanism and rationale as
    /// `DashPaySyncManager::start`. The first pass runs immediately.
    pub fn start(self: Arc<Self>) {
        start_sync_loop(self, NonZeroUsize::new(DPNS_SYNC_STACK_BYTES), "dpns");
    }

    /// Stop the background sync loop. Cancel-only — a pass already
    /// inside `sync_now` keeps running to completion; use
    /// [`quiesce`](Self::quiesce) for a real drain barrier.
    pub fn stop(&self) {
        self.sync.cancel_loop();
    }

    /// Cancel the loop and wait for any in-flight pass to fully drain —
    /// same contract as `DashPaySyncManager::quiesce`.
    #[must_use = "a false return means the pass did NOT drain; the caller must fail closed"]
    pub async fn quiesce(&self) -> bool {
        self.quiesce_within(COORDINATOR_DRAIN_BUDGET).await
    }

    /// [`quiesce`](Self::quiesce) with an explicit drain budget. On
    /// timeout the admission gate is left closed and the caller must
    /// fail closed. A later successful `quiesce` reopens the gate, as
    /// does [`start`](Self::start), the running loop, or a one-shot
    /// `sync_now`, once the wedged pass has released `is_syncing`.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        self.quiesce_held_within(budget).await.is_some()
    }

    /// Drain variant that keeps sync admission shut until the returned
    /// guard drops.
    #[must_use = "None means the pass did NOT drain; the caller must fail closed"]
    pub(crate) async fn quiesce_held_within(&self, budget: Duration) -> Option<QuiesceGuard<'_>> {
        self.sync.drain(|| self.stop(), budget).await
    }

    /// Drain variant that **seals** admission permanently — used by
    /// manager shutdown so a mid-flight host-thread `sync_now` cannot
    /// start a fresh pass (and fire persister/event callbacks) after the
    /// host freed its context.
    pub(crate) async fn quiesce_sealed_within(&self, budget: Duration) -> bool {
        self.sync.drain_sealed(|| self.stop(), budget).await
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

    /// Run one marketplace sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`Self::is_syncing`] to
    /// distinguish. Per-wallet errors are logged and recorded in the
    /// summary but never abort the sweep. Dispatches the completion
    /// event before returning.
    pub async fn sync_now(&self) -> DpnsSyncPassSummary {
        // Bail when another pass holds the slot or a `quiesce()` has closed
        // the gate, so the drain gets a true barrier. The permit holds
        // `is_syncing` until it drops at the end of this function — after
        // the completion event below, and on panic unwind too.
        let Admission::Admitted(_permit) = self.sync.admit() else {
            return DpnsSyncPassSummary::default();
        };

        let snapshot: Vec<(WalletId, Arc<PlatformWallet>)> = {
            let wallets = self.wallets.load();
            wallets.iter().map(|(id, w)| (*id, Arc::clone(w))).collect()
        };

        let mut summary = DpnsSyncPassSummary::default();
        for (wallet_id, wallet) in snapshot {
            let outcome = match wallet.identity().sync_dpns_marketplace().await {
                Ok(wallet_summary) => WalletDpnsSyncOutcome::Ok(wallet_summary),
                Err(e) => {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "DPNS marketplace sync failed for wallet; continuing with the rest"
                    );
                    WalletDpnsSyncOutcome::Err(e.to_string())
                }
            };
            summary.wallet_results.insert(wallet_id, outcome);
        }

        let now = SyncLoop::unix_now();
        summary.sync_unix_seconds = now;
        self.sync.record_completed(now);

        self.events.on_dpns_marketplace_sync_completed(&summary);

        summary
    }
}

impl SyncCoordinator for DpnsSyncManager {
    fn sync_loop(&self) -> &SyncLoop {
        &self.sync
    }

    async fn background_pass(&self) {
        self.sync_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::{OnceLock, Weak};

    use dash_spv::EventHandler;

    use crate::events::PlatformEventHandler;

    /// Counts `on_dpns_marketplace_sync_completed` dispatches, standing in
    /// for the host's FFI handler, and records how many of them arrived
    /// while the coordinator still held its `is_syncing` slot.
    struct CompletionCounter {
        completions: AtomicUsize,
        dispatched_with_slot_held: AtomicUsize,
        manager: OnceLock<Weak<DpnsSyncManager>>,
    }

    impl EventHandler for CompletionCounter {}

    impl PlatformEventHandler for CompletionCounter {
        fn on_dpns_marketplace_sync_completed(&self, _summary: &DpnsSyncPassSummary) {
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

    impl CompletionCounter {
        fn completions(&self) -> usize {
            self.completions.load(AtomicOrdering::SeqCst)
        }
    }

    /// A coordinator over an empty wallet map, wired to a completion
    /// counter, plus the registry that owns its loop thread. No wallets
    /// means a pass runs zero per-wallet syncs (and no network I/O) but
    /// still drives the whole slot → gate → stamp → event protocol.
    fn make_manager_with_registry() -> (
        Arc<DpnsSyncManager>,
        Arc<CompletionCounter>,
        Arc<ThreadRegistry<WalletWorker>>,
    ) {
        let wallets = Arc::new(ArcSwap::from_pointee(BTreeMap::new()));
        let counter = Arc::new(CompletionCounter {
            completions: AtomicUsize::new(0),
            dispatched_with_slot_held: AtomicUsize::new(0),
            manager: OnceLock::new(),
        });
        let events = Arc::new(PlatformEventManager::new(vec![
            Arc::clone(&counter) as Arc<dyn PlatformEventHandler>
        ]));
        let registry = ThreadRegistry::<WalletWorker>::new();
        let manager = Arc::new(DpnsSyncManager::new(wallets, Arc::clone(&registry), events));
        counter
            .manager
            .set(Arc::downgrade(&manager))
            .expect("set once");
        (manager, counter, registry)
    }

    fn make_manager() -> (Arc<DpnsSyncManager>, Arc<CompletionCounter>) {
        let (manager, counter, _registry) = make_manager_with_registry();
        (manager, counter)
    }

    /// Poll until `counter` has seen at least one completion.
    async fn wait_for_a_completion(counter: &CompletionCounter, what: &str) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions() == 0 {
            assert!(tokio::time::Instant::now() < deadline, "{what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
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

    /// A normal pass stamps the summary and the coordinator, fires the
    /// completion event exactly once and leaves the slot free.
    #[tokio::test]
    async fn should_dispatch_completion_once_per_pass() {
        let (mgr, counter) = make_manager();
        assert_eq!(mgr.last_sync_unix_seconds(), None);

        let summary = mgr.sync_now().await;

        assert_eq!(counter.completions(), 1);
        assert_ne!(summary.sync_unix_seconds, 0, "a pass that ran is stamped");
        assert_eq!(
            mgr.last_sync_unix_seconds(),
            Some(summary.sync_unix_seconds)
        );
        assert!(!mgr.is_syncing());

        mgr.sync_now().await;
        assert_eq!(counter.completions(), 2);
    }

    /// The completion event reaches the host while the pass still holds
    /// `is_syncing`, so a drain waiting on the slot's falling edge also
    /// waits out the host callback.
    #[tokio::test]
    async fn should_dispatch_completion_before_releasing_the_slot() {
        let (mgr, counter) = make_manager();

        mgr.sync_now().await;

        assert_eq!(counter.completions(), 1);
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
        assert_eq!(counter.completions(), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(
            mgr.is_syncing(),
            "the losing call must not release the in-flight pass's slot"
        );
        assert!(!mgr.sync_admission_closed());

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert_eq!(counter.completions(), 1);
    }

    /// `quiesce()` stays pending while a pass holds the slot and returns
    /// once it falls, with admission reopened.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_block_quiesce_until_in_flight_pass_drains() {
        let (mgr, _counter) = make_manager();

        // Hold the slot as an in-flight pass would, for exactly as long as
        // the test decides — no timer the test could miss on a loaded runner.
        assert!(
            mgr.wedge_sync_slot_for_test(),
            "test should own the is_syncing slot"
        );

        let quiesce_fut = mgr.quiesce();
        tokio::pin!(quiesce_fut);

        // While the pass holds the slot, quiesce must stay pending, with
        // admission already shut.
        assert!(
            futures::poll!(quiesce_fut.as_mut()).is_pending(),
            "quiesce returned while a pass was in flight"
        );
        assert!(mgr.sync_admission_closed());
        assert!(mgr.is_syncing(), "pass should still be in flight");

        // Once the pass drains, quiesce must return, with admission reopened.
        mgr.release_sync_slot_for_test();
        let drained = tokio::time::timeout(Duration::from_secs(2), quiesce_fut)
            .await
            .expect("quiesce did not return after the pass drained");
        assert!(drained);
        assert!(!mgr.sync_admission_closed());
        assert!(!mgr.is_syncing());
    }

    /// A pass that never drains times the drain out and leaves the gate
    /// latched; a later successful quiesce reopens it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_time_out_quiesce_and_leave_gate_latched_when_pass_never_drains() {
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

    /// A `sync_now` behind a closed gate runs no pass: empty unstamped
    /// summary, no completion event, slot released again.
    #[tokio::test]
    async fn should_bail_sync_now_while_admission_is_shut() {
        let (mgr, counter) = make_manager();

        let gate_hold = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("an idle coordinator must drain immediately");

        let summary = mgr.sync_now().await;

        assert!(summary.is_empty());
        assert_eq!(summary.sync_unix_seconds, 0);
        assert_eq!(counter.completions(), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");
        drop(gate_hold);
    }

    /// Admission stays shut for as long as the drain's guard is held and
    /// is restored when it drops.
    #[tokio::test]
    async fn should_bar_passes_until_the_quiesce_guard_drops() {
        let (mgr, counter) = make_manager();

        let guard = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("an idle coordinator must drain immediately");
        assert!(mgr.sync_admission_closed());

        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(counter.completions(), 0);

        drop(guard);

        assert!(!mgr.sync_admission_closed());
        mgr.sync_now().await;
        assert_eq!(counter.completions(), 1);
    }

    /// Overlapping holders compose: the gate reopens only when the last
    /// guard drops.
    #[tokio::test]
    async fn should_reopen_admission_only_after_the_last_quiesce_guard_drops() {
        let (mgr, _counter) = make_manager();

        let outer = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("drain");
        let inner = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("drain");

        drop(inner);
        assert!(
            mgr.sync_admission_closed(),
            "the outer holder is still mutating; admission must stay shut"
        );

        drop(outer);
        assert!(!mgr.sync_admission_closed());
    }

    /// The shutdown drain seals the gate: no later pass, drain or restart
    /// reopens it.
    #[tokio::test]
    async fn should_never_reopen_admission_after_a_sealed_quiesce() {
        let (mgr, counter) = make_manager();

        assert!(mgr.quiesce_sealed_within(Duration::from_secs(1)).await);
        assert!(mgr.sync_admission_closed(), "seal must leave the gate shut");

        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(counter.completions(), 0);

        assert!(mgr.quiesce().await);
        assert!(mgr.sync_admission_closed());
        assert!(mgr.sync_now().await.is_empty());
        assert_eq!(counter.completions(), 0);
        assert!(!mgr.is_syncing());
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
        assert_eq!(counter.completions(), 0);
    }

    /// A loop started behind a pass a timed-out drain gave up on runs no
    /// pass while that one is wedged, and resumes once it drains.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_resume_passes_once_the_wedged_pass_drains_after_start() {
        let (mgr, counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        assert!(
            mgr.sync_admission_closed(),
            "timed-out drain latches the gate"
        );

        // The loop's first attempt loses the slot CAS to the wedge; it then
        // polls at `LATCHED_RETRY_INTERVAL`, not the 60 s default.
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            mgr.sync_admission_closed(),
            "gate stays latched while wedged"
        );
        assert_eq!(
            counter.completions(),
            0,
            "no pass may run behind a wedged one"
        );

        mgr.release_sync_slot_for_test();
        wait_for_a_completion(
            &counter,
            "loop never resumed passes after the wedged pass drained",
        )
        .await;
        assert!(!mgr.sync_admission_closed(), "latch is cleared");

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// The wedged pass is already gone when `start()` runs: the start
    /// itself clears the latch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_reopen_a_latched_gate_on_start_when_the_wedged_pass_is_gone() {
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

        wait_for_a_completion(
            &counter,
            "loop never ran a pass after start cleared the latch",
        )
        .await;

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// A one-shot `sync_now` with no loop running clears a latch whose
    /// wedged pass has drained, and not one whose pass is still wedged.
    #[tokio::test]
    async fn should_reopen_a_drained_latch_on_sync_now() {
        let (mgr, counter) = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        assert!(mgr.sync_now().await.is_empty(), "bails behind the wedge");
        assert_eq!(counter.completions(), 0);
        assert!(mgr.sync_admission_closed(), "latch stays while wedged");

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert_eq!(
            counter.completions(),
            1,
            "a direct pass runs once the wedged pass is gone"
        );
        assert!(
            !mgr.sync_admission_closed(),
            "the direct pass cleared the latch"
        );
    }

    /// The first pass runs as soon as the loop starts, not one interval
    /// later (the default interval is far longer than the wait here).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_run_first_pass_immediately_after_start() {
        let (mgr, counter) = make_manager();

        Arc::clone(&mgr).start();
        wait_for_a_completion(&counter, "the loop never ran its first pass").await;
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
        wait_for_a_completion(&counter, "the loop never ran its first pass").await;

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            counter.completions(),
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
