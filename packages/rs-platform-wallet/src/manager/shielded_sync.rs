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
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use dash_async::ThreadRegistry;

use crate::events::PlatformEventManager;
use crate::manager::{
    coordinator_worker_config, drain_pass, QuiesceGate, QuiesceGuard, SyncSlotGuard, WalletWorker,
    COORDINATOR_DRAIN_BUDGET,
};
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
    /// Shared registry that owns this loop's lifecycle: it spawns the
    /// OS thread, owns its cancellation token, and joins it at shutdown.
    /// A generation-guarded slot handles a `stop()` + quick `start()`
    /// without a stale loop clobbering the new one, and its per-key
    /// clearing latch bars a (re)start mid `clear_shielded`.
    registry: Arc<ThreadRegistry<WalletWorker>>,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Gates new passes while a [`quiesce`](Self::quiesce) drains an
    /// in-flight one, while a [`QuiesceGuard`] holder mutates state, and
    /// terminally once shutdown seals it. `sync_now` / `sync_wallet` bail
    /// (after taking the `is_syncing` slot) when it is closed, so once a
    /// drain observes `is_syncing == false` no further pass can start —
    /// giving Clear / stop a real "no more host-visible mutations"
    /// barrier that cancel-only [`stop`](Self::stop) does not provide.
    quiescing: QuiesceGate,
    /// Parent of the cancellation token every pass runs under. The
    /// manager owns it rather than reusing the loop's registry token so a
    /// pass dispatched directly from a host thread (`sync_now(true)`,
    /// `sync_wallet`) is cancellable too: [`stop`](Self::stop) cancels the
    /// current parent and rotates in a fresh one, so passes started after
    /// the stop are not born cancelled. Held only for a handful of
    /// instructions, never across an `.await`.
    pass_cancel: std::sync::Mutex<CancellationToken>,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
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
            registry,
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: QuiesceGate::default(),
            pass_cancel: std::sync::Mutex::new(CancellationToken::new()),
            last_sync_unix: AtomicU64::new(0),
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
        let secs = interval.as_secs().max(1);
        self.interval_secs.store(secs, Ordering::Release);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.load(Ordering::Acquire))
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.registry.is_running(WalletWorker::ShieldedSync)
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Acquire)
    }

    /// Unix seconds of the last completed pass, or `None` if no pass
    /// has ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        match self.last_sync_unix.load(Ordering::Acquire) {
            0 => None,
            n => Some(n),
        }
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
        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let this = self;
        // The registry owns the whole lifecycle under one slot lock: it
        // refuses the start if teardown has latched `closing` OR a
        // `clear_shielded` holds this key's clearing latch (so no fresh pass
        // can re-persist notes into the store the clear is about to wipe),
        // installs the cancellation token, spawns the thread, and reaps any
        // prior generation. No check-then-spawn gap to race.
        //
        // A `quiesce` that timed out latched the gate closed. Clear it now
        // if the wedged pass has since drained; otherwise the loop below
        // re-checks before each pass, so admission resumes the moment the
        // pass is gone without the host having to stop again.
        if !this.quiescing.reopen_if_drained(&this.is_syncing) {
            tracing::debug!(
                "shielded sync loop started behind a wedged pass; passes resume once it drains"
            );
        }
        registry.start_thread(
            WalletWorker::ShieldedSync,
            coordinator_worker_config(),
            move |cancel| {
                handle.block_on(async move {
                    loop {
                        if cancel.is_cancelled() {
                            break;
                        }

                        this.quiescing.reopen_if_drained(&this.is_syncing);

                        // Background-loop cadence — honor the
                        // per-wallet caught-up cooldown so a
                        // sleepy network doesn't refetch +
                        // re-trial-decrypt the partial buffer
                        // chunk every interval. User-initiated
                        // syncs pass `force=true` to the FFI
                        // entry point below and bypass this.
                        this.sync_now(false).await;

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }
                });
            },
        );
    }

    /// Stop the background sync loop and cancel the in-flight pass, if
    /// any. Returns immediately.
    ///
    /// A pass already inside `coordinator.sync()` observes the
    /// cancellation at its next batch or network boundary, finalizes the
    /// prefix it consumed and flushes that to the persisters, then
    /// releases `is_syncing`. So this is **not** a barrier: for a real
    /// "nothing is running and nothing more will be persisted" guarantee
    /// — required by Clear, unregister, and rebind — use
    /// [`quiesce`](Self::quiesce), which calls this first and then waits
    /// for that finalize.
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
        self.registry.cancel(WalletWorker::ShieldedSync);
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
    /// `quiesce` reopens the gate, as does [`start`](Self::start) (or the
    /// running loop) once the wedged pass has released `is_syncing`.
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
        drain_pass(&self.quiescing, &self.is_syncing, || self.stop(), budget).await
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
        let guard = self.quiesce_held_within(budget).await;
        let drained = guard.is_some();
        // Seal before the guard drops so its Drop cannot reopen.
        self.quiescing.seal();
        drop(guard);
        drained
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
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return ShieldedSyncPassSummary::default();
        }
        // Clears `is_syncing` on every exit path — including panic unwind —
        // so a failed pass can never wedge `quiesce()`'s drain.
        let _slot = SyncSlotGuard(&self.is_syncing);

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and Clear/stop get a true barrier.
        if self.quiescing.is_closed() {
            return ShieldedSyncPassSummary::default();
        }

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

        // A pass cancelled before it did anything is not a completed pass:
        // no timestamp, no completion event (same as losing the slot CAS).
        // The coordinator returns the empty summary for exactly that case
        // (a cancelled scan that consumed nothing); one that did consume
        // batches has flushed them to the persisters, so it is reported
        // like any other pass and the host applies the deltas.
        if pass_cancel.is_cancelled() && summary.is_empty() {
            return summary;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Honor the coordinator's own `sync_unix_seconds` stamp
        // when it set one; supply our own otherwise (empty pass).
        if summary.sync_unix_seconds == 0 {
            summary.sync_unix_seconds = now;
        }
        self.last_sync_unix
            .store(summary.sync_unix_seconds, Ordering::Release);

        // Dispatch the completion event BEFORE clearing `is_syncing`.
        // `quiesce()` drains on the falling edge of `is_syncing`, so if
        // we cleared the flag first a stop/clear caller could unblock
        // while this completion event (FFI callback → Swift
        // `handleShieldedSyncCompleted`) is still pending — surfacing a
        // stale post-stop/post-clear event. Holding the flag across the
        // dispatch makes quiesce's barrier cover the event too (`_slot`
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
    /// another sync pass was already in flight, or if the pass was
    /// cancelled by [`stop`](Self::stop) before it consumed any
    /// scan data (the coordinator then reports no wallets).
    pub async fn sync_wallet(
        &self,
        wallet_id: &WalletId,
        force: bool,
    ) -> Result<Option<ShieldedSyncSummary>, crate::error::PlatformWalletError> {
        let coordinator_snapshot: Option<Arc<NetworkShieldedCoordinator>> = {
            let slot = self.coordinator_slot.read().await;
            slot.as_ref().map(Arc::clone)
        };
        let Some(coordinator) = coordinator_snapshot else {
            return Ok(None);
        };

        // Token before the slot CAS and the gate check — see `sync_now`
        // for the ordering.
        let pass_cancel = self.pass_token();
        // Reuse the manager-wide `is_syncing` flag so a per-wallet
        // `sync_wallet()` can't race the periodic `sync_now()`
        // against the same store — both go through
        // `coordinator.sync()`, which serializes per-coordinator
        // but the manager flag is what the host UI watches.
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(None);
        }
        // Clears `is_syncing` on every exit path — including panic unwind —
        // so a failed pass can never wedge `quiesce()`'s drain.
        let _slot = SyncSlotGuard(&self.is_syncing);

        // Bail if a `quiesce()` raised the gate after our CAS (see
        // `sync_now`) so the drain barrier holds.
        if self.quiescing.is_closed() {
            return Ok(None);
        }

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

impl std::fmt::Debug for ShieldedSyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShieldedSyncManager")
            .field("is_running", &self.is_running())
            .field("is_syncing", &self.is_syncing())
            .field("interval_secs", &self.interval_secs.load(Ordering::Acquire))
            .field("last_sync_unix", &self.last_sync_unix_seconds())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use dash_spv::EventHandler;

    use crate::events::PlatformEventHandler;

    /// Counts `on_shielded_sync_completed` dispatches, standing in for the
    /// host's FFI handler.
    struct CompletionCounter {
        completions: AtomicUsize,
    }

    impl EventHandler for CompletionCounter {}

    impl PlatformEventHandler for CompletionCounter {
        fn on_shielded_sync_completed(&self, _summary: &ShieldedSyncPassSummary) {
            self.completions.fetch_add(1, AtomicOrdering::SeqCst);
        }
    }

    /// A manager with an empty coordinator slot: `sync_now` still runs the
    /// slot → gate → event protocol, which is all these tests exercise.
    fn make_manager() -> (Arc<ShieldedSyncManager>, Arc<CompletionCounter>) {
        let counter = Arc::new(CompletionCounter {
            completions: AtomicUsize::new(0),
        });
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            Arc::clone(&counter) as Arc<dyn PlatformEventHandler>
        ]));
        (
            Arc::new(ShieldedSyncManager::new(
                event_manager,
                Arc::new(RwLock::new(None)),
                ThreadRegistry::<WalletWorker>::new(),
            )),
            counter,
        )
    }

    fn wedge_sync_slot(mgr: &ShieldedSyncManager) {
        assert!(
            mgr.is_syncing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
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
            mgr.quiescing.is_closed(),
            "timed-out drain latches the gate"
        );

        // The loop's first attempt loses the slot CAS to the wedge and
        // sleeps an interval; keep that short (the minimum is 1 s).
        mgr.set_interval(Duration::from_secs(1));
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(mgr.quiescing.is_closed(), "gate stays latched while wedged");
        assert_eq!(
            counter.completions.load(AtomicOrdering::SeqCst),
            0,
            "no pass may run behind a wedged one"
        );

        mgr.is_syncing.store(false, Ordering::Release);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while counter.completions.load(AtomicOrdering::SeqCst) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "loop never resumed passes after the wedged pass drained"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!mgr.quiescing.is_closed(), "latch is cleared");

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
        mgr.is_syncing.store(false, Ordering::Release);
        assert!(mgr.quiescing.is_closed(), "latch outlives the wedged pass");

        Arc::clone(&mgr).start();
        assert!(!mgr.quiescing.is_closed(), "start clears a stale latch");

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

    /// A manual `sync_now` cannot clear the latch itself: it only ever
    /// runs once a `start()` or the loop has observed the drained pass.
    #[tokio::test]
    async fn sync_now_still_bails_while_latched() {
        let (mgr, counter) = make_manager();

        wedge_sync_slot(&mgr);
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.is_syncing.store(false, Ordering::Release);

        assert!(mgr.sync_now(true).await.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert!(
            mgr.quiescing.is_closed(),
            "a direct pass leaves the latch alone"
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
            wedge_sync_slot(&holder);
            let _slot = SyncSlotGuard(&holder.is_syncing);
            holder.pass_token().cancelled().await;
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
}
