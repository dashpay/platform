//! Periodic shielded (Orchard) note + nullifier sync coordinator.
//!
//! Mirrors [`PlatformAddressSyncManager`](super::platform_address_sync::PlatformAddressSyncManager):
//! runs [`PlatformWallet::shielded_sync`] for every wallet that has a
//! bound [`ShieldedWallet`] on a fixed cadence, and emits a summary
//! event so UI and persistence layers can react.
//!
//! Wallets without a bound shielded sub-wallet are silently skipped
//! — `bind_shielded` is the host's responsibility (it requires
//! mnemonic access via the keychain resolver), so the manager
//! shouldn't error out passes just because some wallets aren't yet
//! shielded-aware.
//!
//! Not auto-started. Call [`ShieldedSyncManager::start`] once the
//! shielded sub-wallets are bound.
//!
//! Feature-gated behind `shielded` — when the feature is off, the
//! whole module is omitted from the build.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::events::PlatformEventManager;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::shielded::ShieldedSyncSummary;
use crate::wallet::PlatformWallet;

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
/// Holds a handle to the same `wallets` map owned by
/// [`PlatformWalletManager`](super::PlatformWalletManager) (via
/// `Arc`), so wallets bound after `start` are picked up on the next
/// tick without any re-registration.
///
/// Each pass:
/// 1. Snapshots the wallet map (short read lock, no await while
///    held).
/// 2. Calls [`PlatformWallet::shielded_sync`] on each wallet
///    sequentially. Returns
///    [`WalletShieldedOutcome::Skipped`] for unbound wallets.
/// 3. Stores the pass timestamp.
/// 4. Dispatches
///    [`PlatformEventManager::on_shielded_sync_completed`].
///
/// `sync_now` is re-entrant-safe: if a pass is already running,
/// calling `sync_now` again returns an empty summary immediately.
pub struct ShieldedSyncManager {
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    event_manager: Arc<PlatformEventManager>,
    /// Cancel token for the background loop, if running.
    background_cancel: StdMutex<Option<CancellationToken>>,
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
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

impl ShieldedSyncManager {
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_manager: Arc<PlatformEventManager>,
    ) -> Self {
        Self {
            wallets,
            event_manager,
            background_cancel: StdMutex::new(None),
            background_generation: AtomicU64::new(0),
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
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
        let mut guard = self.background_cancel.lock().expect("bg_cancel poisoned");
        if guard.is_some() {
            return;
        }
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());
        // Bump the generation while we still hold the slot lock so
        // the load below in any prior thread's cleanup observes
        // `current_gen != my_gen` ordered against this token swap.
        let my_gen = self.background_generation.fetch_add(1, Ordering::AcqRel) + 1;
        drop(guard);

        let handle = tokio::runtime::Handle::current();
        let this = self;
        std::thread::Builder::new()
            .name("shielded-sync".into())
            .spawn(move || {
                handle.block_on(async move {
                    loop {
                        if cancel.is_cancelled() {
                            break;
                        }

                        this.sync_now().await;

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }

                    // Only clear `background_cancel` if the active
                    // generation is still ours. Without this guard a
                    // tight `stop()` → `start()` reschedule has the
                    // exiting thread overwrite the *new* generation's
                    // token, leaving the new loop running but
                    // unreflectable via `is_running()` / `stop()`.
                    if this.background_generation.load(Ordering::Acquire) == my_gen {
                        if let Ok(mut guard) = this.background_cancel.lock() {
                            *guard = None;
                        }
                    }
                });
            })
            .expect("failed to spawn shielded-sync thread");
    }

    /// Stop the background sync loop. No-op if not running.
    pub fn stop(&self) {
        if let Some(token) = self
            .background_cancel
            .lock()
            .expect("bg_cancel poisoned")
            .take()
        {
            token.cancel();
        }
    }

    /// Run one sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    pub async fn sync_now(&self) -> ShieldedSyncPassSummary {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return ShieldedSyncPassSummary::default();
        }

        let snapshot: Vec<(WalletId, Arc<PlatformWallet>)> = {
            let wallets = self.wallets.read().await;
            wallets.iter().map(|(id, w)| (*id, Arc::clone(w))).collect()
        };

        let mut summary = ShieldedSyncPassSummary::default();
        for (wallet_id, wallet) in snapshot {
            let outcome = match wallet.shielded_sync().await {
                Ok(Some(result)) => WalletShieldedOutcome::Ok(result),
                Ok(None) => WalletShieldedOutcome::Skipped,
                Err(e) => {
                    tracing::warn!(
                        "Shielded sync failed for wallet {}: {}",
                        hex::encode(wallet_id),
                        e
                    );
                    WalletShieldedOutcome::Err(e.to_string())
                }
            };
            summary.wallet_results.insert(wallet_id, outcome);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        summary.sync_unix_seconds = now;
        self.last_sync_unix.store(now, Ordering::Release);
        self.is_syncing.store(false, Ordering::Release);

        self.event_manager.on_shielded_sync_completed(&summary);

        summary
    }

    /// Sync a single wallet on demand.
    ///
    /// Acquires the manager's `is_syncing` exclusion before
    /// touching the wallet's shielded sub-wallet, mirroring
    /// [`sync_now`]. If a pass is already in flight this returns
    /// `Ok(None)` immediately rather than serializing — the caller
    /// got told "no" without their request also blocking the
    /// running periodic pass. Inspect [`is_syncing`] beforehand if
    /// you need to distinguish "wallet has no shielded sub-wallet"
    /// from "another pass was running".
    ///
    /// Returns `Ok(None)` if the wallet has no bound shielded
    /// sub-wallet, or if another sync pass was already in flight.
    pub async fn sync_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Option<ShieldedSyncSummary>, crate::error::PlatformWalletError> {
        let wallet = {
            let wallets = self.wallets.read().await;
            wallets.get(wallet_id).cloned()
        };
        let wallet = wallet.ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(hex::encode(wallet_id))
        })?;

        // Reuse the manager-wide `is_syncing` flag so a per-wallet
        // sync_wallet() can't race the periodic sync_now() against
        // the same `ShieldedWallet` / store. PlatformWallet's
        // `shielded_sync` only takes a read lock on the optional
        // shielded slot, so without this gate two passes can step
        // on each other's commitment-tree appends and
        // last-synced-index updates.
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(None);
        }

        let result = wallet.shielded_sync().await;

        self.is_syncing.store(false, Ordering::Release);
        result
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
