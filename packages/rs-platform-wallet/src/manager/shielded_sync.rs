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
    Arc, Mutex as StdMutex,
};

use dash_async::AtomicFlagGuard;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

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
    /// Cancel token for the background loop, if running.
    background_cancel: StdMutex<Option<CancellationToken>>,
    /// Join handle for the background loop's OS thread, if running.
    /// Taken and joined by [`quiesce`](Self::quiesce) so shutdown can
    /// confirm the `!Send` loop fully exited before the host drops the
    /// runtime.
    background_join: StdMutex<Option<std::thread::JoinHandle<()>>>,
    /// Monotonically increasing generation counter. Bumped on every
    /// `start()` so the exiting thread can tell whether its
    /// generation is still the active one before clearing
    /// `background_cancel`. Without this, a `stop()` → `start()`
    /// overlap lets the prior thread's cleanup strip the new
    /// generation's token, leaving the new loop running but
    /// untrackable via `is_running()`.
    background_generation: AtomicU64,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Set by [`quiesce`](Self::quiesce) to gate new passes while it
    /// drains an in-flight one. `sync_now` / `sync_wallet` bail (after
    /// taking the `is_syncing` slot) when this is set, so once `quiesce`
    /// observes `is_syncing == false` no further pass can start — giving
    /// Clear / stop a real "no more host-visible mutations" barrier that
    /// cancel-only [`stop`](Self::stop) does not provide.
    quiescing: AtomicBool,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

impl ShieldedSyncManager {
    pub fn new(
        event_manager: Arc<PlatformEventManager>,
        coordinator_slot: Arc<RwLock<Option<Arc<NetworkShieldedCoordinator>>>>,
    ) -> Self {
        Self {
            event_manager,
            coordinator_slot,
            background_cancel: StdMutex::new(None),
            background_join: StdMutex::new(None),
            background_generation: AtomicU64::new(0),
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            last_sync_unix: AtomicU64::new(0),
        }
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
        self.background_cancel
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
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
    pub fn start(self: Arc<Self>) {
        let mut cancel_guard = self
            .background_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if cancel_guard.is_some() {
            return;
        }

        // Take any handle left by a prior stop() call so we can reap it — but
        // DON'T join it here, while we still hold background_cancel. stop()
        // takes-and-cancels the token but never touches background_join, so a
        // stop()→start() sequence would otherwise overwrite (detach) the old
        // handle and shutdown() would miss that thread. Joining it under
        // background_cancel would DEADLOCK the reap into its 1 s backstop: the
        // exiting prior thread's epilogue also locks background_cancel (to
        // clear its slot), so it would block on the lock we hold → never
        // finish → get detached on the exact stop()→start() path the reap
        // exists for. We install the new token + bump the generation below,
        // release the lock, and only THEN reap (after this fn's tail).
        let prior = self
            .background_join
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        let cancel = CancellationToken::new();
        *cancel_guard = Some(cancel.clone());
        // Bump the generation while we still hold the slot lock so
        // the load below in any prior thread's cleanup observes
        // `current_gen != my_gen` ordered against this token swap.
        let my_gen = self.background_generation.fetch_add(1, Ordering::AcqRel) + 1;

        let handle = tokio::runtime::Handle::current();
        let this = Arc::clone(&self);
        let join = std::thread::Builder::new()
            .name("shielded-sync".into())
            .spawn(move || {
                handle.block_on(async move {
                    loop {
                        if cancel.is_cancelled() {
                            break;
                        }

                        // Background-loop cadence — honor the
                        // per-wallet caught-up cooldown so a
                        // sleepy network doesn't refetch +
                        // re-trial-decrypt the partial buffer
                        // chunk every interval. User-initiated
                        // syncs pass `force=true` to the FFI
                        // entry point below and bypass this.
                        //
                        // Race the pass against cancellation. `stop()` /
                        // `quiesce()` cancel the token; with `biased` the
                        // cancel arm is polled first, so a pass stalled on
                        // a hung SDK fetch is dropped at its `.await` the
                        // instant we cancel. Dropping the `sync_now` future
                        // unwinds to the `is_syncing` `AtomicFlagGuard` it
                        // holds, clearing the flag promptly — so the drain
                        // loop in `quiesce()` frees and the join lands well
                        // inside `shutdown()`'s timeout. A stalled pass can
                        // no longer strand a live `!Send` thread.
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => break,
                            _ = this.sync_now(false) => {}
                        }

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }

                    // Only clear `background_cancel` if the active
                    // generation is still ours. Acquire the lock FIRST,
                    // then read/compare `background_generation` under it
                    // (matching identity_sync / platform_address_sync).
                    // Reading the generation BEFORE locking opens a
                    // stale-read TOCTOU: this exiting thread could observe
                    // a pre-bump generation, then block on the lock until a
                    // concurrent `start()` released it, and null the
                    // freshly-installed token — leaving the new loop
                    // running but unreflectable via `is_running()` /
                    // `stop()`. `start()` bumps the generation while it
                    // holds this same lock, so comparing under the lock
                    // guarantees we observe the post-swap value.
                    if let Ok(mut guard) = this.background_cancel.lock() {
                        if this.background_generation.load(Ordering::Acquire) == my_gen {
                            *guard = None;
                        }
                    }
                });
            })
            .expect("failed to spawn shielded-sync thread");
        // Store the join handle while still holding cancel_guard — a
        // concurrent quiesce() must wait for this lock before calling
        // stop(), so the handle is always stored before it can be taken.
        *self
            .background_join
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(join);

        // Release background_cancel BEFORE reaping the prior thread, so its
        // epilogue can observe the bumped generation (and skip clearing our
        // freshly-installed token) without contending the lock we hold.
        // Holding the lock across the join below is what would block the
        // prior thread, spin the full 1 s deadline, and detach — the very
        // stall this ordering removes.
        drop(cancel_guard);

        // Now reap the prior thread. It was already cancellation-signalled by
        // stop(), and with the lock released its epilogue completes promptly,
        // so is_finished() trips within a few milliseconds and the join is
        // near-instant. The 1 s deadline survives only as a genuine-wedge
        // backstop (e.g. a pass wedged in a Drop that never yields); if it
        // fires we detach the already-cancelled thread to unblock start().
        if let Some(h) = prior {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while !h.is_finished() {
                if std::time::Instant::now() >= deadline {
                    tracing::warn!(
                        "shielded-sync prior thread did not finish within 1 s \
                         after cancellation; detaching to unblock start()"
                    );
                    break; // Drop h — detaches; thread was already cancelled.
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            if h.is_finished() {
                let _ = h.join(); // Reap resources; near-instant since finished.
            }
        }
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: this requests cancellation and returns
    /// immediately. A pass already inside `sync_now` /
    /// `coordinator.sync()` keeps running to completion (including its
    /// persister-callback fan-out). For a real "nothing is running and
    /// nothing more will be persisted" barrier — required by Clear,
    /// unregister, and rebind — use [`quiesce`](Self::quiesce).
    pub fn stop(&self) {
        if let Some(token) = self
            .background_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            token.cancel();
        }
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
    pub async fn quiesce(&self) -> super::CoordinatorThreadStatus {
        self.quiescing.store(true, Ordering::Release);
        // RAII gate: resets `quiescing` on *every* exit path — a normal
        // return, a timed-out `shutdown()` / Clear dropping this future,
        // or a panic. Without it a quiesce that doesn't run to completion
        // leaves the gate latched `true`, silently bailing every future
        // pass. Reopening on drop is safe because `stop()` (below) has
        // already cancelled the loop, so no new pass can start.
        let _quiescing_gate = AtomicFlagGuard::new(&self.quiescing);
        self.stop();
        while self.is_syncing.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let handle = self
            .background_join
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        super::join_coordinator_thread(handle).await
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
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return ShieldedSyncPassSummary::default();
        }

        // RAII guard: clears `is_syncing` on every exit path, including
        // panics. Without this a panic inside the pass would leave
        // `is_syncing=true` forever and wedge `quiesce()`'s drain loop.
        let _is_syncing_guard = AtomicFlagGuard::new(&self.is_syncing);

        // A `quiesce()` may have raised the gate between our CAS and
        // here; bail so the drain can complete and Clear/stop get a
        // true barrier. Guard clears `is_syncing` on return.
        if self.quiescing.load(Ordering::Acquire) {
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
        self.last_sync_unix
            .store(summary.sync_unix_seconds, Ordering::Release);

        // Dispatch the completion event BEFORE `_is_syncing_guard` drops.
        // `quiesce()` drains on the falling edge of `is_syncing`; if
        // the guard cleared the flag before the dispatch a stop/clear
        // caller could unblock while the callback is still pending —
        // surfacing a stale post-stop/post-clear event.
        self.event_manager.on_shielded_sync_completed(&summary);

        summary
        // `_is_syncing_guard` drops here → `is_syncing = false`
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

        // RAII guard clears `is_syncing` on every exit path including panics.
        let _is_syncing_guard = AtomicFlagGuard::new(&self.is_syncing);

        // Bail if a `quiesce()` raised the gate after our CAS (see
        // `sync_now`) so the drain barrier holds.
        if self.quiescing.load(Ordering::Acquire) {
            return Ok(None);
        }

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
            .field("interval_secs", &self.interval_secs.load(Ordering::Acquire))
            .field("last_sync_unix", &self.last_sync_unix_seconds())
            .finish()
    }
}

// The whole module is already `#[cfg(feature = "shielded")]`-gated at its
// `mod` declaration (manager/mod.rs), so these tests compile only under that
// feature — no extra per-test gate needed.
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a manager over an **empty** coordinator slot wired to a
    /// handler-less event manager. An empty slot makes every `sync_now`
    /// pass a no-op (empty-coordinator handling returns immediately), so
    /// the background loop parks in its interval sleep — exactly where
    /// cancellation lands cleanly — without needing a live SDK / network.
    /// That is all the start/stop/restart thread-lifecycle tests below
    /// exercise.
    fn make_manager() -> Arc<ShieldedSyncManager> {
        let coordinator_slot = Arc::new(RwLock::new(None));
        let event_manager = Arc::new(PlatformEventManager::new(vec![]));
        Arc::new(ShieldedSyncManager::new(event_manager, coordinator_slot))
    }

    /// Regression: a tight `stop()` → `start()` must reap the prior loop's
    /// OS thread promptly, NOT stall on the 1 s detach backstop.
    ///
    /// The prior thread's exit epilogue locks `background_cancel` to
    /// conditionally clear its slot. The earlier ordering held
    /// `background_cancel` across the prior-handle join inside `start()`, so
    /// on a back-to-back `stop()` → `start()` the exiting thread blocked on
    /// that lock, never finished, and the reap spin-waited the full second
    /// before detaching — a 1 s stall plus a transient untracked thread. The
    /// fix installs the new token + generation, releases `background_cancel`,
    /// and only then reaps, so the prior thread's epilogue runs and the join
    /// lands in milliseconds. Mirrors the identity-sync and
    /// platform-address-sync siblings.
    ///
    /// `stop()` and `start()` run back-to-back in one blocking closure
    /// (mirroring the real call site) so `start()` re-acquires the lock
    /// microseconds after `stop()` frees it — before the async-woken prior
    /// thread can reach its epilogue. Against the old lock-held ordering this
    /// reliably stalls ~1 s and fails the bound below.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn restart_after_stop_reaps_prior_thread() {
        let mgr = make_manager();

        // Launch the first loop and let its immediate (no-op, empty
        // coordinator) pass complete so the thread parks in the interval
        // sleep, where cancellation lands cleanly.
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Back-to-back cancel-only stop + restart, off the runtime so the
        // synchronous reap can't starve a worker. `start()` re-grabs
        // background_cancel right after `stop()` frees it.
        let restart = Arc::clone(&mgr);
        let elapsed = tokio::task::spawn_blocking(move || {
            restart.stop();
            let started = std::time::Instant::now();
            Arc::clone(&restart).start();
            started.elapsed()
        })
        .await
        .unwrap();

        assert!(
            elapsed < Duration::from_millis(500),
            "stop()→start() stalled for {elapsed:?}: prior thread was not \
             reaped promptly (background_cancel held across the join?)"
        );
        assert!(mgr.is_running(), "restart must leave the new loop tracked");

        // Wind the new loop down so the test leaves no live !Send thread.
        mgr.quiesce().await;
        assert!(!mgr.is_running());
    }
}
