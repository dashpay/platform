//! Periodic shielded (Orchard) note sync coordinator. Spends are
//! detected during the note scan (scan-based nullifier matching),
//! so there is no separate nullifier-sync pass.
//!
//! Mirrors [`PlatformAddressSyncManager`](super::platform_address_sync::PlatformAddressSyncManager):
//! drives a single
//! [`NetworkShieldedCoordinator::sync`](crate::wallet::shielded::NetworkShieldedCoordinator::sync)
//! pass on a fixed cadence and emits a summary event so UI and
//! persistence layers can react. The coordinator pass itself
//! covers every wallet registered on the network in a single
//! SDK fetch (see the coordinator's module docs).
//!
//! Empty-coordinator handling: if shielded support hasn't been
//! configured (no [`configure_shielded`] call has run yet), sync
//! passes return an empty summary — no wallet on this manager
//! can have shielded state until the coordinator exists, so
//! iterating wallets here would just produce noise.
//!
//! Not auto-started. Call [`ShieldedSyncManager::start`] once
//! shielded support has been configured and at least one wallet
//! has bound.
//!
//! Feature-gated behind `shielded` — when the feature is off,
//! the whole module is omitted from the build.
//!
//! [`configure_shielded`]: crate::manager::PlatformWalletManager::configure_shielded

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use dash_async::ThreadRegistry;

use crate::events::PlatformEventManager;
use crate::manager::sync_loop::{start_sync_loop, Admission, SyncCoordinator, SyncLoop};
use crate::manager::{QuiesceGuard, WalletWorker, COORDINATOR_DRAIN_BUDGET};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::shielded::{NetworkShieldedCoordinator, ShieldedSyncSummary};

/// Default cadence — 60s. Shielded sync is heavier than address sync
/// (chunked at 2048 entries with trial decryption per entry), so this
/// is conservative compared to the 15s address-sync cadence.
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 60;

/// Outcome of syncing a single wallet in a shielded sync pass.
///
/// Not `Clone` because `ShieldedSyncSummary` carries the underlying
/// `dash_sdk` result types that aren't `Clone` either. Consumers
/// receive it by reference through the event-manager dispatch.
#[derive(Debug)]
pub enum WalletShieldedOutcome {
    /// Successful sync. Carries the per-wallet sync summary
    /// (`new_notes`, `total_scanned`, `newly_spent`, current `balance`).
    Ok(ShieldedSyncSummary),
    /// Either the wallet has no bound shielded sub-wallet (skipped) or
    /// the sync failed. The string is empty for "skipped" and carries
    /// an error message otherwise.
    Skipped,
    /// Error message from a failed sync.
    Err(String),
}

impl WalletShieldedOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, WalletShieldedOutcome::Ok(_))
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, WalletShieldedOutcome::Skipped)
    }
}

/// Summary of one full shielded sync pass across every registered
/// wallet.
#[derive(Debug, Default)]
pub struct ShieldedSyncPassSummary {
    /// Per-wallet outcomes keyed by `WalletId`. Wallets without a
    /// bound shielded sub-wallet appear as
    /// [`WalletShieldedOutcome::Skipped`] so consumers can distinguish
    /// "no shielded wallet here" from "sync errored".
    pub wallet_results: BTreeMap<WalletId, WalletShieldedOutcome>,
    /// Unix seconds at which the pass completed. `0` means "no pass
    /// ran" (e.g. a concurrent pass was already in flight and we
    /// skipped).
    pub sync_unix_seconds: u64,
}

impl ShieldedSyncPassSummary {
    pub fn is_empty(&self) -> bool {
        self.wallet_results.is_empty()
    }

    pub fn success_count(&self) -> usize {
        self.wallet_results.values().filter(|o| o.is_ok()).count()
    }

    pub fn skipped_count(&self) -> usize {
        self.wallet_results
            .values()
            .filter(|o| o.is_skipped())
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.wallet_results.len() - self.success_count() - self.skipped_count()
    }
}

/// Periodic shielded sync coordinator.
///
/// Holds a handle to the same `coordinator_slot` owned by
/// [`PlatformWalletManager`](super::PlatformWalletManager) (via
/// `Arc`), so wallets bound after `start` are picked up on the
/// next tick without any re-registration (the network-scoped
/// coordinator iterates its own registry).
///
/// Each pass:
/// 1. Snapshots the coordinator `Arc` (short read lock, no
///    `.await` while held).
/// 2. Calls [`NetworkShieldedCoordinator::sync`] once — the
///    coordinator handles the union of every registered
///    subwallet's IVK in a single SDK fetch.
/// 3. Stores the pass timestamp.
/// 4. Dispatches
///    [`PlatformEventManager::on_shielded_sync_completed`].
///
/// `sync_now` is re-entrant-safe: if a pass is already running,
/// calling `sync_now` again returns an empty summary
/// immediately.
pub struct ShieldedSyncManager {
    event_manager: Arc<PlatformEventManager>,
    /// Network-scoped shielded coordinator slot, shared with the
    /// owning `PlatformWalletManager`. Sync passes route through
    /// `coordinator.sync(force, &pass_cancel)` — under the
    /// manager-owned pass token — whenever the slot is populated;
    /// an empty slot returns an empty pass summary (no wallets
    /// can be shielded-bound without `configure_shielded` having
    /// run first, so an empty slot guarantees no shielded state
    /// exists).
    coordinator_slot: Arc<RwLock<Option<Arc<NetworkShieldedCoordinator>>>>,
    /// The lifecycle shared with the sibling coordinators: the
    /// registry-owned loop thread (whose per-key clearing latch bars a
    /// (re)start mid `clear_shielded`), the `is_syncing` slot, the
    /// last-pass timestamp, and the admission gate. `sync_now` /
    /// `sync_wallet` bail (after taking the slot) when the gate is closed
    /// — by a [`quiesce`](Self::quiesce) draining an in-flight pass, by a
    /// [`QuiesceGuard`] holder mutating state, or terminally once shutdown
    /// seals it — so once a drain observes `is_syncing == false` no
    /// further pass can start, giving Clear / stop a real "no more
    /// host-visible mutations" barrier that cancel-only
    /// [`stop`](Self::stop) does not provide.
    sync: SyncLoop,
    /// Parent of the cancellation token every pass runs under. The
    /// manager owns it rather than reusing the loop's registry token so a
    /// pass dispatched directly from a host thread (`sync_now(true)`,
    /// `sync_wallet`) is cancellable too: [`stop`](Self::stop) cancels the
    /// current parent and rotates in a fresh one, so passes started after
    /// the stop are not born cancelled. Held only for a handful of
    /// instructions, never across an `.await`.
    pass_cancel: std::sync::Mutex<CancellationToken>,
}

impl ShieldedSyncManager {
    pub fn new(
        event_manager: Arc<PlatformEventManager>,
        coordinator_slot: Arc<RwLock<Option<Arc<NetworkShieldedCoordinator>>>>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            event_manager,
            coordinator_slot,
            sync: SyncLoop::new(
                WalletWorker::ShieldedSync,
                registry,
                DEFAULT_SYNC_INTERVAL_SECS,
            ),
            pass_cancel: std::sync::Mutex::new(CancellationToken::new()),
        }
    }

    /// A child of the current pass-cancellation parent. Fires when
    /// [`stop`](Self::stop) runs before the pass that took it finishes.
    pub(crate) fn pass_token(&self) -> CancellationToken {
        self.pass_cancel
            .lock()
            .expect("shielded pass-cancel mutex poisoned")
            .child_token()
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
    /// Runs on a dedicated OS thread, not on a tokio worker, because
    /// the underlying `dash-sdk` shielded-sync future is `!Send` (the
    /// GRPC client state isn't `Send + Sync`). Same trade-off as
    /// [`PlatformAddressSyncManager::start`](super::platform_address_sync::PlatformAddressSyncManager::start).
    ///
    /// **Blocks briefly on restart**: the shared registry synchronously
    /// reaps a still-draining prior-generation thread, spinning up to the
    /// registry reap backstop (default 1 s) before returning. Call it from
    /// the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        // The registry owns the whole lifecycle under one slot lock: it
        // refuses the start if teardown has latched `closing` OR a
        // `clear_shielded` holds this key's clearing latch (so no fresh pass
        // can re-persist notes into the store the clear is about to wipe),
        // installs the cancellation token, spawns the thread, and reaps any
        // prior generation. No check-then-spawn gap to race.
        //
        // A `quiesce` that timed out latched the gate closed. The start
        // clears it if the wedged pass has since drained; otherwise the
        // loop re-checks after each pass, so admission resumes the moment
        // the pass is gone without the host having to stop again.
        start_sync_loop(self, None, "shielded");
    }

    /// Stop the background sync loop and cancel the in-flight pass, if
    /// any. Returns immediately.
    ///
    /// A pass already inside `coordinator.sync()` observes the
    /// cancellation at its next batch or network boundary: a scan cut
    /// short keeps the commitments it appended to the local tree and
    /// publishes nothing else, a scan that had completed still flushes
    /// its result, then the pass releases `is_syncing`. So this is
    /// **not** a barrier: for a real "nothing is running and nothing
    /// more will be persisted" guarantee — required by Clear,
    /// unregister, and rebind — use [`quiesce`](Self::quiesce), which
    /// calls this first and then waits for that release.
    pub fn stop(&self) {
        // Cancel the current pass parent and rotate in a fresh one, so a
        // pass that starts after this stop runs under an uncancelled token.
        let previous = {
            let mut parent = self
                .pass_cancel
                .lock()
                .expect("shielded pass-cancel mutex poisoned");
            std::mem::replace(&mut *parent, CancellationToken::new())
        };
        previous.cancel();
        self.sync.cancel_loop();
    }

    /// Cancel the background loop **and wait for any in-flight sync pass
    /// to fully drain** before returning — a real quiescence barrier,
    /// unlike cancel-only [`stop`](Self::stop).
    ///
    /// After this returns, no sync pass is running and none can start
    /// until the next [`start`](Self::start) / `sync_now`, so a caller
    /// that immediately mutates state a pass touches (Clear's registry
    /// wipe + the host's SwiftData delete; wallet unregister; rebind)
    /// cannot be raced by a pass that re-persists notes after the caller
    /// believed sync had stopped.
    ///
    /// Mechanism: close the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the persister fan-out, so its falling edge (with the
    /// gate closed) is a sound "fully drained" signal. The gate is
    /// reopened before returning so a later start/sync works normally —
    /// a caller that must keep admission shut across a follow-up state
    /// mutation uses [`quiesce_held`](Self::quiesce_held) instead.
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
    /// that path the `quiescing` gate is deliberately **left closed** so
    /// the wedged pass cannot be followed by a fresh one; the caller must
    /// treat the coordinator as non-clean (Clear aborts fail-closed, the
    /// FFI stop surfaces `ErrorShutdownIncomplete`). A later successful
    /// `quiesce` reopens the gate, as does [`start`](Self::start), the
    /// running loop, or a one-shot `sync_now`, once the wedged pass has
    /// released `is_syncing`.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        // The guard drops here, reopening the gate — this is the
        // "drain only" flavor.
        self.quiesce_held_within(budget).await.is_some()
    }

    /// [`quiesce`](Self::quiesce) that **keeps sync admission shut** until
    /// the returned guard drops.
    ///
    /// Required by `clear_shielded`: `quiesce()` alone reopens the gate the
    /// instant it returns, so the wipe that follows runs with admission
    /// already re-opened and a direct `sync_now` / `sync_wallet` on a host
    /// thread can snapshot the old account set and refill the commitment
    /// tree right after `coord.clear()` reset it. Holding the guard across
    /// the whole quiesce → mutate section closes that window.
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

    /// Run one sync pass across every registered wallet.
    ///
    /// `force` is propagated to each wallet's
    /// [`shielded_sync(force)`](crate::wallet::PlatformWallet::shielded_sync):
    /// the background loop passes `false` to honor the per-wallet
    /// caught-up cooldown; user-initiated paths (the manual
    /// "Sync Now" FFI) pass `true` so a tap always re-checks
    /// Platform.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    pub async fn sync_now(&self, force: bool) -> ShieldedSyncPassSummary {
        // Take the pass token BEFORE claiming the slot. A `stop()` that
        // lands after the token is taken cancels this lineage, whichever
        // side of the CAS it falls on; taken after the CAS, a stop that
        // slipped in between would rotate the parent first and hand this
        // pass a fresh, uncancelled child. Losing the CAS just drops the
        // token. It is also before the gate check: `drain_pass` takes its
        // hold (closing the gate) and only then calls `stop()`, so a pass
        // that took its token before the rotation is on the cancelled
        // lineage, and one that took it after sees the closed gate below.
        // Either way no pass runs past a drain.
        let pass_cancel = self.pass_token();
        // Bail without running a pass when another one holds the slot, or
        // when a `quiesce()` has closed the gate — so the drain can complete
        // and Clear/stop get a true barrier. The permit holds `is_syncing`
        // until it drops at the end of this function, on panic unwind too,
        // so a failed pass can never wedge `quiesce()`'s drain.
        let Admission::Admitted(_permit) = self.sync.admit() else {
            return ShieldedSyncPassSummary::default();
        };

        // Snapshot the coordinator Arc and release the slot lock
        // before awaiting so a concurrent `configure_shielded`
        // can't deadlock against our pass.
        //
        // Empty-coordinator handling: if shielded support hasn't
        // been configured yet, return an empty pass summary —
        // `bind_shielded` requires `configure_shielded` to run
        // first (the FFI enforces this), so no wallet on this
        // manager can possibly have shielded state until the
        // coordinator exists.
        let coordinator_snapshot: Option<Arc<NetworkShieldedCoordinator>> = {
            let slot = self.coordinator_slot.read().await;
            slot.as_ref().map(Arc::clone)
        };

        let mut summary = if let Some(coordinator) = coordinator_snapshot {
            coordinator.sync(force, &pass_cancel).await
        } else {
            ShieldedSyncPassSummary::default()
        };

        // A pass whose scan was cut short is not a completed pass: no
        // timestamp, no completion event (same as losing the slot CAS).
        // The coordinator returns the empty summary for exactly that case;
        // a pass stopped only after its scan completed has flushed the
        // scan to the persisters, so it is reported like any other pass
        // and the host applies the deltas.
        if pass_cancel.is_cancelled() && summary.is_empty() {
            return summary;
        }

        let now = SyncLoop::unix_now();
        // Honor the coordinator's own `sync_unix_seconds` stamp
        // when it set one; supply our own otherwise (empty pass).
        if summary.sync_unix_seconds == 0 {
            summary.sync_unix_seconds = now;
        }
        self.sync.record_completed(summary.sync_unix_seconds);

        // Dispatch the completion event BEFORE clearing `is_syncing`.
        // `quiesce()` drains on the falling edge of `is_syncing`, so if
        // we cleared the flag first a stop/clear caller could unblock
        // while this completion event (FFI callback → Swift
        // `handleShieldedSyncCompleted`) is still pending — surfacing a
        // stale post-stop/post-clear event. Holding the flag across the
        // dispatch makes quiesce's barrier cover the event too (`_permit`
        // drops — and clears the flag — only after this dispatch returns).
        self.event_manager.on_shielded_sync_completed(&summary);

        summary
    }

    /// Sync a single wallet on demand.
    ///
    /// Post-Phase-2b shape: since the coordinator's sync pass is
    /// already network-wide (one SDK fetch covers every
    /// registered IVK), "sync this wallet" is implemented as a
    /// full coordinator pass that returns this wallet's slice of
    /// the result. The result map is keyed by `WalletId`; this
    /// method extracts the requested wallet's
    /// [`ShieldedSyncSummary`] before returning.
    ///
    /// Returns `Ok(None)` if `wallet_id` isn't registered on the
    /// coordinator (e.g. shielded support hasn't been configured,
    /// or the wallet has never called `bind_shielded`), if
    /// another sync pass was already in flight, or if the pass's scan
    /// was cut short by [`stop`](Self::stop) (the coordinator then
    /// reports no wallets).
    pub async fn sync_wallet(
        &self,
        wallet_id: &WalletId,
        force: bool,
    ) -> Result<Option<ShieldedSyncSummary>, crate::error::PlatformWalletError> {
        // Token first — before the coordinator read below as well as the
        // slot CAS and the gate check (see `sync_now` for the ordering):
        // a `stop()` that lands while this waits for the coordinator lock
        // must still cancel this pass, not hand it a child of the rotated,
        // uncancelled parent.
        let pass_cancel = self.pass_token();
        let coordinator_snapshot: Option<Arc<NetworkShieldedCoordinator>> = {
            let slot = self.coordinator_slot.read().await;
            slot.as_ref().map(Arc::clone)
        };
        let Some(coordinator) = coordinator_snapshot else {
            return Ok(None);
        };
        // Reuse the manager-wide `is_syncing` flag so a per-wallet
        // `sync_wallet()` can't race the periodic `sync_now()`
        // against the same store — both go through
        // `coordinator.sync()`, which serializes per-coordinator
        // but the manager flag is what the host UI watches.
        // Same admission as `sync_now`, reached only once a coordinator
        // is known to exist: bail when another pass holds the slot or a
        // `quiesce()` has closed the gate, so the drain barrier holds. The
        // permit holds `is_syncing` until it drops at the end of this
        // function, on panic unwind too.
        let Admission::Admitted(_permit) = self.sync.admit() else {
            return Ok(None);
        };

        let pass = coordinator.sync(force, &pass_cancel).await;

        // Extract this wallet's slice from the network-wide pass
        // summary. If the wallet is registered, we'll get back an
        // outcome; otherwise `None`.
        match pass
            .wallet_results
            .into_iter()
            .find(|(id, _)| id == wallet_id)
        {
            Some((_, WalletShieldedOutcome::Ok(summary))) => Ok(Some(summary)),
            Some((_, WalletShieldedOutcome::Skipped)) => Ok(None),
            Some((_, WalletShieldedOutcome::Err(e))) => {
                Err(crate::error::PlatformWalletError::ShieldedSyncFailed(e))
            }
            None => Ok(None),
        }
    }
}

impl SyncCoordinator for ShieldedSyncManager {
    fn sync_loop(&self) -> &SyncLoop {
        &self.sync
    }

    /// Background-loop cadence — honor the per-wallet caught-up cooldown
    /// so a sleepy network doesn't refetch + re-trial-decrypt the partial
    /// buffer chunk every interval. User-initiated syncs pass `force=true`
    /// to `sync_now` from the FFI entry point and bypass this.
    async fn background_pass(&self) {
        self.sync_now(false).await;
    }
}

impl std::fmt::Debug for ShieldedSyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShieldedSyncManager")
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
    use crate::wallet::shielded::FileBackedShieldedStore;

    /// Counts `on_shielded_sync_completed` dispatches, standing in for the
    /// host's FFI handler, and records how many of them arrived while the
    /// coordinator still held its `is_syncing` slot.
    struct CompletionCounter {
        completions: AtomicUsize,
        dispatched_with_slot_held: AtomicUsize,
        manager: OnceLock<Weak<ShieldedSyncManager>>,
    }

    impl EventHandler for CompletionCounter {}

    impl PlatformEventHandler for CompletionCounter {
        fn on_shielded_sync_completed(&self, _summary: &ShieldedSyncPassSummary) {
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

    /// A manager with an empty coordinator slot: `sync_now` still runs the
    /// slot → gate → event protocol, which is all these tests exercise.
    fn make_manager() -> (Arc<ShieldedSyncManager>, Arc<CompletionCounter>) {
        let parts = make_manager_parts();
        (parts.manager, parts.counter)
    }

    /// Everything a test may need to drive: the manager, its completion
    /// counter, the coordinator slot it shares with the wallet manager,
    /// and the registry that owns its loop thread.
    struct ManagerParts {
        manager: Arc<ShieldedSyncManager>,
        counter: Arc<CompletionCounter>,
        coordinator_slot: Arc<RwLock<Option<Arc<NetworkShieldedCoordinator>>>>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    }

    fn make_manager_parts() -> ManagerParts {
        let counter = Arc::new(CompletionCounter {
            completions: AtomicUsize::new(0),
            dispatched_with_slot_held: AtomicUsize::new(0),
            manager: OnceLock::new(),
        });
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            Arc::clone(&counter) as Arc<dyn PlatformEventHandler>
        ]));
        let coordinator_slot = Arc::new(RwLock::new(None));
        let registry = ThreadRegistry::<WalletWorker>::new();
        let manager = Arc::new(ShieldedSyncManager::new(
            event_manager,
            Arc::clone(&coordinator_slot),
            Arc::clone(&registry),
        ));
        counter
            .manager
            .set(Arc::downgrade(&manager))
            .expect("set once");
        ManagerParts {
            manager,
            counter,
            coordinator_slot,
            registry,
        }
    }

    /// A real coordinator over an empty store with no bound wallet: its
    /// pass touches neither the network nor a persister and reports no
    /// wallets.
    fn empty_coordinator(dir: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
        let db_path = dir.join("tree.sqlite");
        let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open file store");
        Arc::new(NetworkShieldedCoordinator::new(
            Arc::new(dash_sdk::Sdk::new_mock()),
            dashcore::Network::Testnet,
            db_path,
            store,
        ))
    }

    fn wedge_sync_slot(mgr: &ShieldedSyncManager) {
        assert!(
            mgr.wedge_sync_slot_for_test(),
            "test should own the is_syncing slot"
        );
    }

    /// Regression for dashpay/platform#5222, traced on this coordinator in
    /// Dash Wallet iOS: Sync Info → Stop during a pass longer than the
    /// drain budget (`ErrorShutdownIncomplete`), then Sync Now / restart.
    /// The loop reported running while every pass bailed on the latched
    /// gate. Now the loop reopens the gate as soon as the wedged pass has
    /// drained, and not before.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_resumes_passes_once_a_timed_out_drains_pass_clears() {
        let (mgr, counter) = make_manager();

        wedge_sync_slot(&mgr);
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
            counter.completions.load(AtomicOrdering::SeqCst),
            0,
            "no pass may run behind a wedged one"
        );

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

    /// The wedged pass is already gone when `start()` runs: the start
    /// itself clears the latch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_reopens_a_latched_gate_when_the_wedged_pass_is_gone() {
        let (mgr, counter) = make_manager();

        wedge_sync_slot(&mgr);
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

    /// A one-shot `sync_now` with no loop running: the latch outlives the
    /// wedged pass, and the direct call clears it itself (a restart is
    /// not required).
    #[tokio::test]
    async fn sync_now_reopens_a_drained_latch() {
        let (mgr, counter) = make_manager();

        wedge_sync_slot(&mgr);
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        assert!(
            mgr.sync_now(true).await.is_empty(),
            "bails behind the wedge"
        );
        assert!(mgr.sync_admission_closed(), "latch stays while wedged");

        mgr.release_sync_slot_for_test();
        mgr.sync_now(true).await;
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

    /// `stop()` cancels the token the in-flight pass runs under and
    /// rotates in a fresh one, so a pass started after the stop is not
    /// born cancelled (dashpay/platform#5200).
    #[tokio::test]
    async fn should_cancel_current_pass_token_on_stop_and_rotate() {
        let (mgr, _counter) = make_manager();

        let in_flight = mgr.pass_token();
        assert!(!in_flight.is_cancelled());

        mgr.stop();
        assert!(in_flight.is_cancelled(), "stop cancels the running pass");
        assert!(
            !mgr.pass_token().is_cancelled(),
            "a pass after the stop runs under a fresh token"
        );
    }

    /// A pass that observes its token lets `quiesce()` drain right away,
    /// instead of after the whole pass or the drain budget: the stop
    /// inside the drain cancels the pass, the pass releases the slot, the
    /// drain sees the falling edge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_quiesce_promptly_when_pass_observes_cancellation() {
        let (mgr, _counter) = make_manager();

        // Stand-in for a pass parked in a network await that checks its
        // token: holds the slot until the manager cancels it.
        let holder = Arc::clone(&mgr);
        let pass = tokio::spawn(async move {
            // Token before the slot, as the production entry points do:
            // taken after, a quiesce that saw the occupied slot first
            // would have rotated the parent and this task would wait on
            // a fresh, uncancelled lineage.
            let pass_cancel = holder.pass_token();
            wedge_sync_slot(&holder);
            pass_cancel.cancelled().await;
            holder.release_sync_slot_for_test();
        });
        while !mgr.is_syncing() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let drained = tokio::time::timeout(Duration::from_secs(2), mgr.quiesce())
            .await
            .expect("quiesce must not wait out the drain budget");
        assert!(drained, "the cancelled pass must drain");
        assert!(!mgr.is_syncing());
        pass.await.expect("pass task");
    }

    fn completions(counter: &CompletionCounter) -> usize {
        counter.completions.load(AtomicOrdering::SeqCst)
    }

    /// Poll until `counter` has seen at least one completion.
    async fn wait_for_a_completion(counter: &CompletionCounter, what: &str) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while completions(counter) == 0 {
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

    /// A pass with no coordinator configured still completes: it stamps
    /// the summary and the manager with its own clock and fires the
    /// completion event exactly once.
    #[tokio::test]
    async fn should_dispatch_completion_once_per_pass() {
        let (mgr, counter) = make_manager();
        assert_eq!(mgr.last_sync_unix_seconds(), None);

        let summary = mgr.sync_now(true).await;

        assert_eq!(completions(&counter), 1);
        assert_ne!(summary.sync_unix_seconds, 0, "a pass that ran is stamped");
        assert_eq!(
            mgr.last_sync_unix_seconds(),
            Some(summary.sync_unix_seconds)
        );
        assert!(!mgr.is_syncing());

        mgr.sync_now(false).await;
        assert_eq!(completions(&counter), 2);
    }

    /// The manager keeps the coordinator's own completion stamp instead
    /// of replacing it with its own clock.
    #[tokio::test]
    async fn should_keep_the_coordinators_stamp_on_a_completed_pass() {
        let parts = make_manager_parts();
        let dir = tempfile::tempdir().expect("tempdir");
        *parts.coordinator_slot.write().await = Some(empty_coordinator(dir.path()));

        let summary = parts.manager.sync_now(true).await;

        assert!(summary.is_empty(), "no wallet is bound");
        assert_ne!(summary.sync_unix_seconds, 0);
        assert_eq!(
            parts.manager.last_sync_unix_seconds(),
            Some(summary.sync_unix_seconds)
        );
        assert_eq!(completions(&parts.counter), 1);
    }

    /// The completion event reaches the host while the pass still holds
    /// `is_syncing`, so a drain waiting on the slot's falling edge also
    /// waits out the host callback.
    #[tokio::test]
    async fn should_dispatch_completion_before_releasing_the_slot() {
        let (mgr, counter) = make_manager();

        mgr.sync_now(true).await;

        assert_eq!(completions(&counter), 1);
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
        wedge_sync_slot(&mgr);

        let summary = mgr.sync_now(true).await;

        assert!(summary.is_empty());
        assert_eq!(summary.sync_unix_seconds, 0, "0 means no pass ran");
        assert_eq!(completions(&counter), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(
            mgr.is_syncing(),
            "the losing call must not release the in-flight pass's slot"
        );
        assert!(!mgr.sync_admission_closed());

        mgr.release_sync_slot_for_test();
        mgr.sync_now(true).await;
        assert_eq!(completions(&counter), 1);
    }

    /// A `sync_now` behind a closed gate runs no pass: empty unstamped
    /// summary, no completion event, slot released again.
    #[tokio::test]
    async fn should_bail_sync_now_while_admission_is_shut() {
        let (mgr, counter) = make_manager();

        let gate_hold = mgr
            .quiesce_held()
            .await
            .expect("an idle coordinator must drain immediately");

        let summary = mgr.sync_now(true).await;

        assert!(summary.is_empty());
        assert_eq!(summary.sync_unix_seconds, 0);
        assert_eq!(completions(&counter), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");
        drop(gate_hold);
    }

    /// A pass that never drains times the drain out and leaves the gate
    /// latched; a later successful quiesce reopens it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_time_out_quiesce_and_leave_gate_latched_when_pass_never_drains() {
        let (mgr, _counter) = make_manager();
        wedge_sync_slot(&mgr);

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

    /// Admission stays shut for as long as the drain's guard is held and
    /// is restored when it drops.
    #[tokio::test]
    async fn should_bar_passes_until_the_quiesce_guard_drops() {
        let (mgr, counter) = make_manager();

        let guard = mgr
            .quiesce_held()
            .await
            .expect("an idle coordinator must drain immediately");
        assert!(mgr.sync_admission_closed());

        assert!(mgr.sync_now(true).await.is_empty());
        assert_eq!(completions(&counter), 0);

        drop(guard);

        assert!(!mgr.sync_admission_closed());
        mgr.sync_now(true).await;
        assert_eq!(completions(&counter), 1);
    }

    /// Overlapping holders compose: the gate reopens only when the last
    /// guard drops.
    #[tokio::test]
    async fn should_reopen_admission_only_after_the_last_quiesce_guard_drops() {
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

    /// The shutdown drain seals the gate: no later pass, drain or restart
    /// reopens it.
    #[tokio::test]
    async fn should_never_reopen_admission_after_a_sealed_quiesce() {
        let (mgr, counter) = make_manager();

        assert!(mgr.quiesce_sealed_within(Duration::from_secs(1)).await);
        assert!(mgr.sync_admission_closed(), "seal must leave the gate shut");

        assert!(mgr.sync_now(true).await.is_empty());
        assert_eq!(completions(&counter), 0);

        assert!(mgr.quiesce().await);
        assert!(mgr.sync_admission_closed());
        assert!(mgr.sync_now(true).await.is_empty());
        assert_eq!(completions(&counter), 0);
        assert!(!mgr.is_syncing());
    }

    /// A drain cancels the pass token before it waits, so a pass that
    /// took its token before the drain is on the cancelled lineage.
    #[tokio::test]
    async fn should_cancel_the_pass_token_when_quiescing() {
        let (mgr, _counter) = make_manager();
        let in_flight = mgr.pass_token();

        assert!(mgr.quiesce().await);

        assert!(in_flight.is_cancelled(), "the drain stops the pass");
        assert!(!mgr.pass_token().is_cancelled());
    }

    /// A pass stopped before its scan produced anything is not a
    /// completed pass: no timestamp and no completion event.
    ///
    /// The pass is parked on the coordinator-slot read (after it took its
    /// token and the slot), stopped there, then released: with no
    /// coordinator it yields the empty summary under a cancelled token.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_not_stamp_or_dispatch_when_cancelled_pass_is_empty() {
        let parts = make_manager_parts();
        let mgr = Arc::clone(&parts.manager);

        let slot_writer = parts.coordinator_slot.write().await;
        let runner = Arc::clone(&mgr);
        let pass = tokio::spawn(async move { runner.sync_now(true).await });
        while !mgr.is_syncing() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        mgr.stop();
        drop(slot_writer);
        let summary = tokio::time::timeout(Duration::from_secs(5), pass)
            .await
            .expect("the pass must finish once the slot lock is released")
            .expect("pass task");

        assert!(summary.is_empty());
        assert_eq!(
            summary.sync_unix_seconds, 0,
            "a cut-short pass is unstamped"
        );
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert_eq!(completions(&parts.counter), 0);
        assert!(!mgr.is_syncing());

        // The stop rotated the token: the next pass is not born cancelled.
        mgr.sync_now(true).await;
        assert_eq!(completions(&parts.counter), 1);
    }

    /// `sync_wallet` behind an in-flight pass reports `None` and leaves
    /// that pass's slot alone.
    #[tokio::test]
    async fn should_return_none_when_slot_is_held() {
        let parts = make_manager_parts();
        let dir = tempfile::tempdir().expect("tempdir");
        *parts.coordinator_slot.write().await = Some(empty_coordinator(dir.path()));
        let mgr = parts.manager;
        wedge_sync_slot(&mgr);

        let outcome = mgr
            .sync_wallet(&[7u8; 32], true)
            .await
            .expect("a refusal is not an error");

        assert!(outcome.is_none());
        assert!(
            mgr.is_syncing(),
            "the refused call must not release the in-flight pass's slot"
        );
        assert!(!mgr.sync_admission_closed());
    }

    /// `sync_wallet` behind a closed gate reports `None` and releases the
    /// slot it briefly claimed.
    #[tokio::test]
    async fn should_return_none_when_admission_is_shut() {
        let parts = make_manager_parts();
        let dir = tempfile::tempdir().expect("tempdir");
        *parts.coordinator_slot.write().await = Some(empty_coordinator(dir.path()));
        let mgr = parts.manager;
        let _guard = mgr.quiesce_held().await.expect("drain");

        let outcome = mgr
            .sync_wallet(&[7u8; 32], true)
            .await
            .expect("a refusal is not an error");

        assert!(outcome.is_none());
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");
        assert!(mgr.sync_admission_closed());
    }

    /// An admitted `sync_wallet` for a wallet the coordinator does not
    /// know reports `None`, fires no completion event, leaves no
    /// timestamp, and clears a drained latch on the way in.
    #[tokio::test]
    async fn should_admit_sync_wallet_and_report_none_for_an_unbound_wallet() {
        let parts = make_manager_parts();
        let dir = tempfile::tempdir().expect("tempdir");
        *parts.coordinator_slot.write().await = Some(empty_coordinator(dir.path()));
        let mgr = parts.manager;

        wedge_sync_slot(&mgr);
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.release_sync_slot_for_test();
        assert!(
            mgr.sync_admission_closed(),
            "latch outlives the wedged pass"
        );

        let outcome = mgr
            .sync_wallet(&[7u8; 32], true)
            .await
            .expect("the pass itself succeeds");

        assert!(outcome.is_none());
        assert!(!mgr.sync_admission_closed(), "the call cleared the latch");
        assert!(!mgr.is_syncing());
        assert_eq!(completions(&parts.counter), 0);
        assert_eq!(mgr.last_sync_unix_seconds(), None);
    }

    /// With no coordinator configured `sync_wallet` returns before it
    /// reaches admission: a drained latch is left as it was.
    #[tokio::test]
    async fn should_leave_admission_untouched_when_sync_wallet_finds_no_coordinator() {
        let (mgr, _counter) = make_manager();

        wedge_sync_slot(&mgr);
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.release_sync_slot_for_test();

        let outcome = mgr
            .sync_wallet(&[7u8; 32], true)
            .await
            .expect("no coordinator is not an error");

        assert!(outcome.is_none());
        assert!(
            mgr.sync_admission_closed(),
            "the coordinator read comes before admission"
        );
        assert!(!mgr.is_syncing());
    }

    /// The first pass runs as soon as the loop starts, not one interval
    /// later (the default interval is far longer than the wait here).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_run_first_pass_immediately_after_start() {
        let (mgr, counter) = make_manager();

        Arc::clone(&mgr).start();
        wait_for_a_completion(&counter, "the loop never ran its first pass").await;
        assert!(mgr.last_sync_unix_seconds().is_some());

        assert!(mgr.quiesce().await);
    }

    /// `start()` on a running loop is a no-op: it neither restarts the loop
    /// (which would run another immediate first pass) nor spawns a second.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_ignore_start_when_loop_is_running() {
        let parts = make_manager_parts();
        let mgr = parts.manager;

        Arc::clone(&mgr).start();
        wait_for_a_completion(&parts.counter, "the loop never ran its first pass").await;

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            completions(&parts.counter),
            1,
            "a second start must not run another first pass"
        );

        mgr.stop();
        assert!(!mgr.is_running(), "one stop cancels the only loop");
        let report = parts.registry.shutdown().await;
        assert!(report.all_clean(), "clean join: {report:?}");
    }

    /// A `stop()` followed at once by `start()` leaves the new loop
    /// running and cancellable, and both generations join cleanly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn should_keep_new_loop_cancellable_after_stop_then_quick_start() {
        let parts = make_manager_parts();
        let mgr = parts.manager;

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        mgr.stop();
        assert!(!mgr.is_running(), "stop() clears the running flag at once");

        Arc::clone(&mgr).start();
        assert!(mgr.is_running(), "loop B must be running after the restart");

        mgr.stop();
        assert!(!mgr.is_running());
        let report = parts.registry.shutdown().await;
        assert!(
            report.all_clean(),
            "clean shutdown after restart: {report:?}"
        );
    }
}
