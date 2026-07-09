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

use dash_async::ThreadRegistry;

use crate::events::PlatformEventManager;
use crate::manager::loop_cancel::LoopCancelGuard;
use crate::manager::{coordinator_worker_config, WalletWorker};
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
    /// Generation-guarded cancel-token slot for the background loop —
    /// see [`LoopCancelGuard`] for the stale-loop shutdown invariant.
    cancel_guard: LoopCancelGuard,
    /// Shared registry that owns this loop's OS-thread join handle for a
    /// panic-aware shutdown join. Join-only — cancellation stays with
    /// [`cancel_guard`](Self::cancel_guard).
    registry: Arc<ThreadRegistry<WalletWorker>>,
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
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            event_manager,
            coordinator_slot,
            cancel_guard: LoopCancelGuard::new(),
            registry,
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
        self.cancel_guard.is_running()
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
    /// **Blocks briefly on restart**: handing the loop thread to the shared
    /// registry synchronously reaps a still-draining prior-generation thread,
    /// spinning up to the registry reap backstop (default 1 s) before
    /// returning. Call it from the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        // Refuse to (re)start while a clear is latched (a fresh pass could
        // `persister.store(...)` notes into the store `clear_shielded` is
        // about to wipe) or once teardown has latched `closing` (the registry
        // does not own this loop's cancellation, so it would run uncancelled).
        // The registry gates its own `register_thread` on both latches, but
        // this loop's cancellation lives in `cancel_guard`, so the gate must
        // also be observed here — before `install` spawns a thread.
        if self.registry.is_closing() || self.registry.is_clearing(WalletWorker::ShieldedSync) {
            return;
        }
        let Some((cancel, my_generation)) = self.cancel_guard.install() else {
            return;
        };
        // Re-check AFTER install (check-lock-check): `clear_shielded` /
        // `shutdown` may have latched between the gate above and `install`.
        // Without this a fresh pass could re-persist notes right after the
        // wipe. Cancel the just-installed token and release the slot rather
        // than spawning.
        if self.registry.is_closing() || self.registry.is_clearing(WalletWorker::ShieldedSync) {
            cancel.cancel();
            self.cancel_guard.clear_if_current(my_generation);
            return;
        }

        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let this = self;
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
                        this.sync_now(false).await;

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }

                    this.cancel_guard.clear_if_current(my_generation);
                });
            })
            .expect("failed to spawn shielded-sync thread");

        // Join-only handoff to the shared registry (see `IdentitySyncManager::start`).
        registry.register_thread(
            WalletWorker::ShieldedSync,
            coordinator_worker_config(),
            join,
        );
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
        if let Some(token) = self.cancel_guard.take() {
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
    pub async fn quiesce(&self) {
        self.quiescing.store(true, Ordering::Release);
        self.stop();
        while self.is_syncing.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.quiescing.store(false, Ordering::Release);
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

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and Clear/stop get a true barrier.
        if self.quiescing.load(Ordering::Acquire) {
            self.is_syncing.store(false, Ordering::Release);
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

        // Dispatch the completion event BEFORE clearing `is_syncing`.
        // `quiesce()` drains on the falling edge of `is_syncing`, so if
        // we cleared the flag first a stop/clear caller could unblock
        // while this completion event (FFI callback → Swift
        // `handleShieldedSyncCompleted`) is still pending — surfacing a
        // stale post-stop/post-clear event. Holding the flag across the
        // dispatch makes quiesce's barrier cover the event too.
        self.event_manager.on_shielded_sync_completed(&summary);

        self.is_syncing.store(false, Ordering::Release);

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

        // Bail if a `quiesce()` raised the gate after our CAS (see
        // `sync_now`) so the drain barrier holds.
        if self.quiescing.load(Ordering::Acquire) {
            self.is_syncing.store(false, Ordering::Release);
            return Ok(None);
        }

        let pass = coordinator.sync(force).await;
        self.is_syncing.store(false, Ordering::Release);

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
