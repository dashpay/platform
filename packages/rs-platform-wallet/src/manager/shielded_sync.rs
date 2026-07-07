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

#[cfg(test)]
use dash_async::AtomicFlagGuard;
use dash_async::{RefcountedFlagGuard, ThreadRegistry};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;

use super::coordinator_lifecycle::CoordinatorLifecycle;
use super::WalletWorker;
use crate::events::PlatformEventManager;
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
    /// `coordinator.sync(force)` whenever the slot is populated;
    /// an empty slot returns an empty pass summary (no wallets
    /// can be shielded-bound without `configure_shielded` having
    /// run first, so an empty slot guarantees no shielded state
    /// exists).
    coordinator_slot: Arc<RwLock<Option<Arc<NetworkShieldedCoordinator>>>>,
    /// Shared lifecycle state + pass-gating protocol under the
    /// [`WalletWorker::ShieldedSync`] key: registry handle, polling
    /// interval, the `is_syncing` / `quiescing` handshake, and the
    /// last-sync stamp. `start` / `stop` / `is_running` / `quiesce` and the
    /// `sync_now` / `sync_wallet` pass gate delegate to it. The `quiescing`
    /// half gives Clear / stop a real "no more host-visible mutations"
    /// barrier that cancel-only [`stop`](Self::stop) does not provide.
    lifecycle: CoordinatorLifecycle,
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
            lifecycle: CoordinatorLifecycle::new(
                registry,
                WalletWorker::ShieldedSync,
                DEFAULT_SYNC_INTERVAL_SECS,
            ),
        }
    }

    /// Set the polling interval. Clamped to a minimum of 1s.
    ///
    /// The running loop picks this up on its next sleep.
    pub fn set_interval(&self, interval: Duration) {
        self.lifecycle.set_interval(interval);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        self.lifecycle.interval()
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.lifecycle.is_running()
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.lifecycle.is_syncing()
    }

    /// Unix seconds of the last completed pass, or `None` if no pass
    /// has ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        self.lifecycle.last_sync_unix_seconds()
    }

    /// Start the background sync loop. Idempotent — calling while
    /// already running is a no-op.
    ///
    /// Runs on a dedicated OS thread, not on a tokio worker, because
    /// the underlying `dash-sdk` shielded-sync future is `!Send` (the
    /// GRPC client state isn't `Send + Sync`). Same trade-off as
    /// [`PlatformAddressSyncManager::start`](super::platform_address_sync::PlatformAddressSyncManager::start).
    pub fn start(self: Arc<Self>) {
        // Background cadence passes `force=false` to honor the per-wallet
        // caught-up cooldown; user-initiated syncs pass `force=true` via the
        // FFI `sync_now`.
        let pass_self = Arc::clone(&self);
        let interval_self = Arc::clone(&self);
        self.lifecycle.spawn_periodic_loop(
            move || {
                let this = Arc::clone(&pass_self);
                async move {
                    let _ = this.sync_now(false).await;
                }
            },
            move || interval_self.interval(),
        );
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: this requests cancellation and returns
    /// immediately. A pass already inside `sync_now` /
    /// `coordinator.sync()` is **cancelled mid-flight** at its next
    /// `.await` (the loop's `biased; cancel-first` select drops the sync
    /// future, see `start`). Persister-callback fan-out for already-
    /// completed stores has fired; any in-flight store is abandoned.
    /// For a real "nothing is running and nothing more will be
    /// persisted" barrier — required by Clear, unregister, and rebind —
    /// use [`quiesce`](Self::quiesce).
    pub fn stop(&self) {
        self.lifecycle.stop();
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
    /// Mechanism: set the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the persister fan-out, so its falling edge (with the
    /// gate up) is a sound "fully drained" signal. The gate is reopened
    /// before returning so a later start/sync works normally.
    ///
    /// Finally **joins** the loop's OS thread (after the drain, so the
    /// thread is on its way out) and returns its terminal status. Joining
    /// while the runtime is still alive is what lets the manager promise
    /// the `!Send` loop has stopped touching `tokio::time` before a
    /// one-shot host drops the runtime.
    pub async fn quiesce(&self) -> dash_async::WorkerStatus {
        self.lifecycle.quiesce().await
    }

    /// Drain + join **without touching the `quiescing` gate**, for a caller
    /// (the Clear flow) that already holds it raised via
    /// [`hold_quiescing_gate`](Self::hold_quiescing_gate) and keeps holding
    /// it across the whole teardown. See
    /// [`CoordinatorLifecycle::quiesce_under_held_gate`].
    pub(crate) async fn quiesce_under_held_gate(&self) -> dash_async::WorkerStatus {
        self.lifecycle.quiesce_under_held_gate().await
    }

    /// Test seam: enter a sync pass directly (claim `is_syncing` via the
    /// pass gate) so a teardown test can stand in for a direct
    /// `sync_now`/`sync_wallet` already in flight, without driving the real
    /// (coordinator-backed) sync path. The returned guard clears the flag
    /// on drop.
    #[cfg(test)]
    pub(crate) fn begin_pass_for_test(&self) -> Option<AtomicFlagGuard<'_>> {
        self.lifecycle.begin_pass()
    }

    /// Raise the `quiescing` gate and hold it raised until the returned
    /// guard drops. Under refcount semantics multiple holders compose, so
    /// this lets a multi-step teardown (Clear) keep new direct `sync_now` /
    /// `sync_wallet` passes off across a check-then-wipe even when a racing
    /// public `quiesce()` lands inside the window — neither party's Drop
    /// can lower the other's barrier.
    pub(crate) fn hold_quiescing_gate(&self) -> RefcountedFlagGuard<'_> {
        self.lifecycle.hold_quiescing_gate()
    }

    /// Test-only read of the underlying `quiescing` flag. Used by
    /// regression tests asserting gate-continuity under concurrent
    /// (re)starts during a Clear.
    #[cfg(test)]
    pub(crate) fn quiescing_load_for_test(&self, ordering: std::sync::atomic::Ordering) -> bool {
        self.lifecycle.quiescing_load_for_test(ordering)
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
        // Claim the pass slot and honour the quiescing gate; bail with an
        // empty summary if a pass is already in flight or a teardown
        // (Clear/stop) raised the gate. The guard clears `is_syncing` on
        // every exit path.
        let Some(_pass) = self.lifecycle.begin_pass() else {
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
            coordinator.sync(force).await
        } else {
            ShieldedSyncPassSummary::default()
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Honor the coordinator's own `sync_unix_seconds` stamp
        // when it set one; supply our own otherwise (empty pass).
        if summary.sync_unix_seconds == 0 {
            summary.sync_unix_seconds = now;
        }
        self.lifecycle
            .store_last_sync_unix(summary.sync_unix_seconds);

        // Dispatch the completion event BEFORE the `_pass` guard drops.
        // `quiesce()` drains on the falling edge of `is_syncing`; if
        // the guard cleared the flag before the dispatch a stop/clear
        // caller could unblock while the callback is still pending —
        // surfacing a stale post-stop/post-clear event.
        self.event_manager.on_shielded_sync_completed(&summary);

        summary
        // `_pass` drops here → `is_syncing = false`
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
    /// or the wallet has never called `bind_shielded`), or if
    /// another sync pass was already in flight.
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

        // Reuse the manager-wide `is_syncing` flag so a per-wallet
        // `sync_wallet()` can't race the periodic `sync_now()` against the
        // same store — both go through `coordinator.sync()`, which
        // serializes per-coordinator, but the manager flag is what the host
        // UI watches. Bail (Ok(None)) if a pass is already in flight or a
        // teardown raised the quiescing gate.
        let Some(_pass) = self.lifecycle.begin_pass() else {
            return Ok(None);
        };

        let pass = coordinator.sync(force).await;

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
            .field("interval_secs", &self.lifecycle.interval_secs())
            .field("last_sync_unix", &self.last_sync_unix_seconds())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::events::PlatformEventHandler;

    /// Build a manager with an empty coordinator slot and a no-handler
    /// event manager. An empty slot makes `sync_now` return an empty
    /// summary, but it still drives the full timestamp + completion
    /// protocol and — crucially for this test — the generation-guarded
    /// background loop, without needing a live `NetworkShieldedCoordinator`.
    fn make_manager() -> Arc<ShieldedSyncManager> {
        let event_manager = Arc::new(PlatformEventManager::new(Vec::<
            Arc<dyn PlatformEventHandler>,
        >::new()));
        let coordinator_slot = Arc::new(RwLock::new(None));
        // merge: port #3953's helper onto our 3-arg new() (registry added by
        // the ThreadRegistry refactor), mirroring the sibling address-sync test.
        let registry = ThreadRegistry::new();
        Arc::new(ShieldedSyncManager::new(
            event_manager,
            coordinator_slot,
            registry,
        ))
    }

    // merge: #3953's generation-guard test ported onto our ThreadRegistry
    // refactor — the registry's per-key clearing latch is the equivalent guard.
    /// Restart-in-place regression: a tight `start()` → `stop()` → `start()`
    /// must leave the manager *running* on the new loop. The cancelled stale
    /// loop races to clear its registry slot as it exits; the registry's
    /// per-key clearing latch must stop it from stripping the freshly
    /// installed loop's running state — otherwise the new loop keeps running
    /// but becomes invisible to `is_running()` / `stop()`.
    ///
    /// Determinism: the only wait is a *bounded* poll. With the latch in
    /// place `is_running()` is true for the whole window, so the test
    /// never fails spuriously on correct code. A regression flips it false
    /// within milliseconds once the stale loop clears the slot, which the
    /// poll catches. Needs the multi-thread flavor because `start()`
    /// drives its loop via `Handle::current().block_on` on a dedicated OS
    /// thread, which would deadlock a single-threaded test runtime.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn restart_in_place_keeps_running_after_stale_loop_exits() {
        let mgr = make_manager();

        // Gen 1. Wait (bounded) for the first pass to land — a real
        // lifecycle signal that the loop is now parked in its interval
        // sleep, so its cleanup is still pending when we stop+restart.
        Arc::clone(&mgr).start();
        let mut waited = 0;
        while mgr.last_sync_unix_seconds().is_none() {
            assert!(waited < 200, "gen-1's first sync pass never completed");
            tokio::time::sleep(Duration::from_millis(10)).await;
            waited += 1;
        }

        // Tight stop→start with no await between: the just-cancelled gen-1
        // loop cannot reach its cleanup before gen 2 is installed, so the
        // race window the guard protects is reliably open.
        mgr.stop();
        Arc::clone(&mgr).start();

        // Give the stale gen-1 loop ample time to run its (guarded)
        // cleanup. `is_running()` must stay true throughout.
        for _ in 0..100 {
            assert!(
                mgr.is_running(),
                "stale gen-1 loop cleared gen-2's cancel token — generation guard regressed"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // The surviving loop is the tracked one: a single `stop()` fully
        // reflects it, so there is no orphaned unreflectable duplicate.
        mgr.stop();
        assert!(!mgr.is_running(), "stop() must reflect the live loop");
    }
}
