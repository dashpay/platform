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
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwap;
use dash_async::{ThreadRegistry, WorkerConfig};

use crate::events::PlatformEventManager;
use crate::manager::{
    coordinator_worker_config, drain_pass, QuiesceGate, QuiesceGuard, SyncSlotGuard, WalletWorker,
    COORDINATOR_DRAIN_BUDGET,
};
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
    registry: Arc<ThreadRegistry<WalletWorker>>,
    /// Dispatches `on_dpns_marketplace_sync_completed` after each pass.
    events: Arc<PlatformEventManager>,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Gates new passes while a [`quiesce`](Self::quiesce) drains an
    /// in-flight one — same barrier contract as the sibling coordinators.
    quiescing: QuiesceGate,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

impl DpnsSyncManager {
    pub fn new(
        wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
        events: Arc<PlatformEventManager>,
    ) -> Self {
        Self {
            wallets,
            registry,
            events,
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: QuiesceGate::default(),
            last_sync_unix: AtomicU64::new(0),
        }
    }

    /// Set the polling interval. Clamped to a minimum of 1s. The running
    /// loop picks this up on its next sleep.
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
        self.registry.is_running(WalletWorker::DpnsSync)
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Acquire)
    }

    /// Unix seconds of the last completed pass, or `None` if no pass has
    /// ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        match self.last_sync_unix.load(Ordering::Acquire) {
            0 => None,
            n => Some(n),
        }
    }

    /// Start the background sync loop. Idempotent — calling while
    /// already running is a no-op. Runs on a dedicated registry-owned OS
    /// thread with a deep stack, driving the (`!Send`) SDK futures via
    /// `Handle::block_on` — same mechanism and rationale as
    /// `DashPaySyncManager::start`. The first pass runs immediately.
    pub fn start(self: Arc<Self>) {
        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let this = self;
        let cfg = WorkerConfig {
            stack_size: NonZeroUsize::new(DPNS_SYNC_STACK_BYTES),
            ..coordinator_worker_config()
        };
        registry.start_thread(WalletWorker::DpnsSync, cfg, move |cancel| {
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
            });
        });
    }

    /// Stop the background sync loop. Cancel-only — a pass already
    /// inside `sync_now` keeps running to completion; use
    /// [`quiesce`](Self::quiesce) for a real drain barrier.
    pub fn stop(&self) {
        self.registry.cancel(WalletWorker::DpnsSync);
    }

    /// Cancel the loop and wait for any in-flight pass to fully drain —
    /// same contract as `DashPaySyncManager::quiesce`.
    #[must_use = "a false return means the pass did NOT drain; the caller must fail closed"]
    pub async fn quiesce(&self) -> bool {
        self.quiesce_within(COORDINATOR_DRAIN_BUDGET).await
    }

    /// [`quiesce`](Self::quiesce) with an explicit drain budget. On
    /// timeout the admission gate is left closed and the caller must
    /// fail closed.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        self.quiesce_held_within(budget).await.is_some()
    }

    /// Drain variant that keeps sync admission shut until the returned
    /// guard drops.
    #[must_use = "None means the pass did NOT drain; the caller must fail closed"]
    pub(crate) async fn quiesce_held_within(&self, budget: Duration) -> Option<QuiesceGuard<'_>> {
        drain_pass(&self.quiescing, &self.is_syncing, || self.stop(), budget).await
    }

    /// Drain variant that **seals** admission permanently — used by
    /// manager shutdown so a mid-flight host-thread `sync_now` cannot
    /// start a fresh pass (and fire persister/event callbacks) after the
    /// host freed its context.
    pub(crate) async fn quiesce_sealed_within(&self, budget: Duration) -> bool {
        let guard = self.quiesce_held_within(budget).await;
        let drained = guard.is_some();
        // Seal before the guard drops so its Drop cannot reopen.
        self.quiescing.seal();
        drop(guard);
        drained
    }

    /// Run one marketplace sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`Self::is_syncing`] to
    /// distinguish. Per-wallet errors are logged and recorded in the
    /// summary but never abort the sweep. Dispatches the completion
    /// event before returning.
    pub async fn sync_now(&self) -> DpnsSyncPassSummary {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return DpnsSyncPassSummary::default();
        }
        // Clears `is_syncing` on every exit path — including panic
        // unwind — so a failed pass can never wedge `quiesce()`'s drain.
        let _slot = SyncSlotGuard(&self.is_syncing);

        // A `quiesce()` may have raised the gate between our CAS and
        // here; bail so the drain gets a true barrier.
        if self.quiescing.is_closed() {
            return DpnsSyncPassSummary::default();
        }

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

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        summary.sync_unix_seconds = now;
        self.last_sync_unix.store(now, Ordering::Release);

        self.events.on_dpns_marketplace_sync_completed(&summary);

        summary
    }
}
