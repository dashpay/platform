//! Periodic platform-address balance sync coordinator.
//!
//! Mirrors what iOS used to do in `PlatformBalanceSyncService`: run
//! [`PlatformAddressWallet::sync_balances`] for every registered wallet
//! on a fixed cadence, and emit a summary event so UI and persistence
//! layers can react.
//!
//! Not auto-started. Call [`PlatformAddressSyncManager::start`] once the
//! wallets are registered and the SPV runtime is up.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwapOption;
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use key_wallet::PlatformP2PKHAddress;

use crate::wallet::PlatformAddressTag;
use tokio::sync::RwLock;

use dash_async::ThreadRegistry;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::manager::loop_cancel::LoopCancelGuard;
use crate::manager::{coordinator_worker_config, WalletWorker};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Default cadence — matches the 15s BLAST loop we previously ran in Swift.
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
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    event_manager: Arc<PlatformEventManager>,
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
    /// drains an in-flight one. `sync_now` bails (after taking the
    /// `is_syncing` slot) when this is set, so once `quiesce` observes
    /// `is_syncing == false` no further pass can start — giving shutdown
    /// a real "no more host-visible sync-completed callbacks" barrier
    /// that cancel-only [`stop`](Self::stop) does not provide.
    quiescing: AtomicBool,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
    /// Shared config applied uniformly across wallets and accounts.
    ///
    /// `ArcSwapOption` instead of a mutex because writes are rare
    /// (user changes sync config), reads fire every pass, and the
    /// loop's async state machine dislikes holding a `MutexGuard`.
    config: ArcSwapOption<AddressSyncConfig>,
}

impl PlatformAddressSyncManager {
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_manager: Arc<PlatformEventManager>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            wallets,
            event_manager,
            cancel_guard: LoopCancelGuard::new(),
            registry,
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            last_sync_unix: AtomicU64::new(0),
            config: ArcSwapOption::empty(),
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
    /// **Blocks briefly on restart**: handing the loop thread to the shared
    /// registry synchronously reaps a still-draining prior-generation thread,
    /// spinning up to the registry reap backstop (default 1 s) before
    /// returning. Call it from the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        // Refuse to (re)start once the registry has latched closed for
        // teardown (see `IdentitySyncManager::start`).
        if self.registry.is_closing() {
            return;
        }
        let Some((cancel, my_generation)) = self.cancel_guard.install() else {
            return;
        };
        // Check-lock-check: bail if a shutdown latched `closing` between the
        // gate above and install.
        if self.registry.is_closing() {
            cancel.cancel();
            self.cancel_guard.clear_if_current(my_generation);
            return;
        }

        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let this = self;
        let join = std::thread::Builder::new()
            .name("platform-address-sync".into())
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

                    this.cancel_guard.clear_if_current(my_generation);
                });
            })
            .expect("failed to spawn platform-address-sync thread");

        // Join-only handoff to the shared registry (see `IdentitySyncManager::start`).
        registry.register_thread(
            WalletWorker::PlatformAddressSync,
            coordinator_worker_config(),
            join,
        );
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
    /// that immediately tears the manager down (and frees the host-owned
    /// event-handler context the FFI handed to us) cannot be raced by a
    /// pass that fires `on_platform_address_sync_completed` through a
    /// now-dangling pointer.
    ///
    /// Mechanism: set the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the completion-event dispatch (`sync_now` clears it only
    /// after `on_platform_address_sync_completed` returns), so its
    /// falling edge (with the gate up) is a sound "fully drained" signal.
    /// The gate is reopened before returning so a later start/sync works
    /// normally.
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
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    pub async fn sync_now(&self) -> PlatformAddressSyncSummary {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return PlatformAddressSyncSummary::default();
        }

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and shutdown gets a true barrier
        // (no further `on_platform_address_sync_completed` host callback
        // after quiesce returns).
        if self.quiescing.load(Ordering::Acquire) {
            self.is_syncing.store(false, Ordering::Release);
            return PlatformAddressSyncSummary::default();
        }

        let snapshot: Vec<(WalletId, Arc<PlatformWallet>)> = {
            let wallets = self.wallets.read().await;
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

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        summary.sync_unix_seconds = now;
        self.last_sync_unix.store(now, Ordering::Release);

        // Dispatch the completion event BEFORE clearing `is_syncing`.
        // `quiesce()` drains on the falling edge of `is_syncing`, so if
        // we cleared the flag first a shutdown caller could unblock and
        // free the host event-handler context while this completion
        // event (FFI callback → host handler) is still pending — a
        // use-after-free. Holding the flag across the dispatch makes
        // quiesce's barrier cover the host callback too. Mirrors the
        // ordering in `ShieldedSyncManager::sync_now`.
        self.event_manager
            .on_platform_address_sync_completed(&summary);

        self.is_syncing.store(false, Ordering::Release);

        summary
    }

    /// Sync a single wallet on demand. Does not set the global
    /// `is_syncing` flag — callers that care about exclusion should
    /// gate on [`is_syncing`] themselves.
    pub async fn sync_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<AddressSyncResult<PlatformAddressTag, PlatformP2PKHAddress>, PlatformWalletError>
    {
        let wallet = {
            let wallets = self.wallets.read().await;
            wallets.get(wallet_id).cloned()
        };
        let wallet =
            wallet.ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;

        let config = self.current_config();
        wallet.platform().sync_balances(config).await
    }
}

impl std::fmt::Debug for PlatformAddressSyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressSyncManager")
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

    /// Event handler that just counts `on_platform_address_sync_completed`
    /// dispatches. Stands in for the host's FFI handler so we can assert
    /// the quiescing gate suppresses the completion callback.
    struct CompletionCounter {
        completions: AtomicUsize,
    }

    impl CompletionCounter {
        fn new() -> Self {
            Self {
                completions: AtomicUsize::new(0),
            }
        }
    }

    impl EventHandler for CompletionCounter {}

    impl PlatformEventHandler for CompletionCounter {
        fn on_platform_address_sync_completed(&self, _summary: &PlatformAddressSyncSummary) {
            self.completions.fetch_add(1, AtomicOrdering::SeqCst);
        }
    }

    /// Build a manager over an empty wallet map wired to a completion
    /// counter. No wallets means `sync_now` runs zero per-wallet syncs
    /// but still drives the full flag → gate → completion-event protocol
    /// we're testing here.
    fn make_manager() -> (Arc<PlatformAddressSyncManager>, Arc<CompletionCounter>) {
        let wallets = Arc::new(RwLock::new(BTreeMap::new()));
        let counter = Arc::new(CompletionCounter::new());
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            Arc::clone(&counter) as Arc<dyn PlatformEventHandler>
        ]));
        (
            Arc::new(PlatformAddressSyncManager::new(
                wallets,
                event_manager,
                ThreadRegistry::<WalletWorker>::new(),
            )),
            counter,
        )
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
                holder
                    .is_syncing
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok(),
                "test should own the is_syncing slot"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
            holder.is_syncing.store(false, Ordering::Release);
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

        assert!(!mgr.quiescing.load(Ordering::Acquire));
        assert!(!mgr.is_syncing());
        pass.await.unwrap();
    }

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — in particular, without firing the
    /// `on_platform_address_sync_completed` host callback. This is the
    /// gate that prevents a pass from slipping in between `quiesce`'s
    /// `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let (mgr, counter) = make_manager();

        // Raise the gate as `quiesce()` would.
        mgr.quiescing.store(true, Ordering::Release);

        let summary = mgr.sync_now().await;

        // Empty summary, no host completion callback, slot released so a
        // later (post-quiesce) pass can still run.
        assert!(summary.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert!(!mgr.is_syncing());
    }
}
