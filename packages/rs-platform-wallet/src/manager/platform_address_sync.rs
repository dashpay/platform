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
use std::sync::Arc;

use dash_async::ThreadRegistry;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwapOption;
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use key_wallet::PlatformP2PKHAddress;

use super::coordinator_lifecycle::CoordinatorLifecycle;
use super::WalletWorker;
use crate::wallet::PlatformAddressTag;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
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
    /// Shared lifecycle state + pass-gating protocol under the
    /// [`WalletWorker::PlatformAddressSync`] key: registry handle, polling
    /// interval, the `is_syncing` / `quiescing` handshake, and the
    /// last-sync stamp. `start` / `stop` / `is_running` / `quiesce` and the
    /// `sync_now` pass gate delegate to it. The `quiescing` half gives
    /// shutdown a real "no more host-visible sync-completed callbacks"
    /// barrier that cancel-only [`stop`](Self::stop) does not provide.
    lifecycle: CoordinatorLifecycle,
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
            lifecycle: CoordinatorLifecycle::new(
                registry,
                WalletWorker::PlatformAddressSync,
                DEFAULT_SYNC_INTERVAL_SECS,
            ),
            config: ArcSwapOption::empty(),
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
    pub fn start(self: Arc<Self>) {
        // Reopen the quiescing gate so this (re)start's passes can run.
        self.lifecycle.reopen_quiescing_gate();

        let cfg = self.lifecycle.worker_config();

        // The loop drives `!Send` SDK futures via `Handle::block_on` on a
        // dedicated OS thread (spawned by the registry). `biased` polls the
        // cancel arm first so a pass stalled on a hung SDK fetch is dropped
        // at its `.await` the instant the registry cancels.
        let handle = tokio::runtime::Handle::current();
        let this = Arc::clone(&self);
        self.lifecycle
            .registry()
            .start_thread(self.lifecycle.worker(), cfg, move |cancel| {
                handle.block_on(async move {
                    loop {
                        if cancel.is_cancelled() {
                            break;
                        }
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => break,
                            _ = this.sync_now() => {}
                        }

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }
                });
            });
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: requests cancellation and returns immediately.
    /// The signal is delivered, but `stop()` does not wait for any
    /// in-flight pass to settle. Because [`start`](Self::start)'s outer
    /// `select!` polls the cancel arm `biased` first, an in-flight
    /// `sync_now()` future may be dropped at its next `.await` (the SDK
    /// fetch or the `on_platform_address_sync_completed` dispatch), in
    /// which case the host callback for that pass may or may not have
    /// fired. It may equally have already completed before the signal
    /// arrived. Either way, there is no post-condition that no further
    /// host callback fires, and a fresh pass started after this returns
    /// is still possible — for a real "nothing is running and nothing
    /// more will fire a host callback" barrier — required by manager
    /// shutdown so the host can free the event-handler context — use
    /// [`quiesce`](Self::quiesce) (or the manager-wide
    /// [`shutdown`](super::PlatformWalletManager::shutdown)).
    pub fn stop(&self) {
        self.lifecycle.stop();
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
    ///
    /// Finally **joins** the loop's OS thread (after the drain, so the
    /// thread is on its way out) and returns its terminal status. Joining
    /// while the runtime is still alive is what lets the manager promise
    /// the `!Send` loop has stopped touching `tokio::time` before a
    /// one-shot host drops the runtime.
    pub async fn quiesce(&self) -> super::CoordinatorThreadStatus {
        self.lifecycle.quiesce().await
    }

    /// Run one sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    pub async fn sync_now(&self) -> PlatformAddressSyncSummary {
        // Claim the pass slot and honour the quiescing gate; bail with an
        // empty summary (and without a host completion callback after
        // quiesce returns) if a pass is already in flight or a teardown
        // raised the gate. The guard clears `is_syncing` on every exit path.
        let Some(_pass) = self.lifecycle.begin_pass() else {
            return PlatformAddressSyncSummary::default();
        };

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
        self.lifecycle.store_last_sync_unix(now);

        // Dispatch the completion event BEFORE the `_pass` guard drops.
        // `quiesce()` drains on the falling edge of `is_syncing`; if the
        // guard cleared the flag before the dispatch a shutdown caller
        // could unblock and free the host event-handler context while
        // the callback is still pending — a use-after-free. The guard
        // drops (clearing `is_syncing`) after this call returns, when
        // the function frame unwinds.
        self.event_manager
            .on_platform_address_sync_completed(&summary);

        summary
        // `_pass` drops here → `is_syncing = false`
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
            .field("interval_secs", &self.lifecycle.interval_secs())
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
        let registry = ThreadRegistry::new();
        (
            Arc::new(PlatformAddressSyncManager::new(
                wallets,
                event_manager,
                registry,
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

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — in particular, without firing the
    /// `on_platform_address_sync_completed` host callback. This is the
    /// gate that prevents a pass from slipping in between `quiesce`'s
    /// `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let (mgr, counter) = make_manager();

        // Raise the gate as `quiesce()` would, held across the pass.
        let _gate = mgr.lifecycle.hold_quiescing_gate();

        let summary = mgr.sync_now().await;

        // Empty summary, no host completion callback, slot released so a
        // later (post-quiesce) pass can still run.
        assert!(summary.is_empty());
        assert_eq!(counter.completions.load(AtomicOrdering::SeqCst), 0);
        assert!(!mgr.is_syncing());
    }
}
