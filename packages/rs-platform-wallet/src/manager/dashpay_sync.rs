//! Periodic DashPay (contact-request + profile) sync coordinator.
//!
//! Folds the on-demand `dashpay_sync()` refresh into the recurring
//! background loop, alongside the platform-address, identity-token, and
//! shielded coordinators. Before this, contact requests and DashPay
//! profiles only refreshed when the host explicitly called the FFI
//! sync entry point; there was no background DashPay refresh at all.
//!
//! **Wallet-driven, not registry-driven — by design.** This is a
//! sibling of [`PlatformAddressSyncManager`](super::platform_address_sync::PlatformAddressSyncManager)
//! rather than an extension of
//! [`IdentitySyncManager`](super::identity_sync::IdentitySyncManager).
//! `IdentitySyncManager` walks a per-identity token *registry* and
//! **skips identities with empty token lists**; a DashPay-only identity
//! with no watched tokens would never sync if DashPay rode that
//! registry. So this coordinator instead holds a handle to the same
//! `wallets` map the address-sync manager receives, snapshots the
//! wallet `Arc`s under a read guard each sweep, and calls
//! [`IdentityWallet::dashpay_sync`](crate::wallet::identity::network::IdentityWallet::dashpay_sync)
//! on **every** wallet — coupling DashPay sync to the token registry is
//! the failure mode this avoids.
//!
//! Each pass:
//! 1. Snapshots the wallet map (short read lock, no await while held).
//! 2. Calls `wallet.identity().dashpay_sync()` on each wallet.
//! 3. Stores the pass timestamp.
//!
//! **Error semantics: log-and-continue per wallet.** A failing
//! `dashpay_sync()` for one wallet is logged and recorded in the pass
//! summary; it never aborts the sweep across the other wallets. The
//! per-*identity* continue (so one identity's fetch failure inside a
//! wallet doesn't abort that wallet's other identities) lives inside
//! `sync_contact_requests` / `sync_profiles` themselves.
//!
//! `sync_now` is re-entrant-safe: if a pass is already running, calling
//! `sync_now` again returns an empty summary immediately (the caller
//! can check `is_syncing()` to distinguish). Shutdown drains an
//! in-flight pass via [`quiesce`](DashPaySyncManager::quiesce), exactly
//! like the address-sync coordinator.
//!
//! Not auto-started. Call [`DashPaySyncManager::start`] once the
//! wallets are registered and the SDK is connected. The on-demand FFI
//! entry points stay available for pull-to-refresh.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Default cadence for the DashPay sync loop.
///
/// Contact requests and profiles move slowly relative to UTXO balance,
/// so a 60s default keeps background DAPI traffic modest while still
/// surfacing new requests/profiles inside a minute. Matches the
/// identity-token loop's default. Tunable at runtime via
/// [`DashPaySyncManager::set_interval`].
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 60;

/// Outcome of syncing a single wallet's DashPay state in a pass.
#[derive(Debug)]
pub enum WalletDashPaySyncOutcome {
    /// `dashpay_sync()` completed for this wallet.
    Ok,
    /// `dashpay_sync()` returned an error message (logged, non-fatal to
    /// the rest of the pass).
    Err(String),
}

impl WalletDashPaySyncOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, WalletDashPaySyncOutcome::Ok)
    }
}

/// Summary of one full DashPay sync pass across every registered wallet.
#[derive(Debug, Default)]
pub struct DashPaySyncSummary {
    /// Per-wallet outcomes keyed by `WalletId`.
    pub wallet_results: BTreeMap<WalletId, WalletDashPaySyncOutcome>,
    /// Unix seconds at which the pass completed. `0` means "no pass ran"
    /// (e.g. a concurrent pass was already in flight and we skipped).
    pub sync_unix_seconds: u64,
}

impl DashPaySyncSummary {
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

/// Periodic DashPay (contact-request + profile) sync coordinator.
///
/// Holds a handle to the same `wallets` map owned by
/// [`PlatformWalletManager`](super::PlatformWalletManager) (via `Arc`),
/// so wallets added after `start` are picked up on the next tick
/// without any re-registration — and crucially without consulting the
/// token registry, so DashPay-only identities are never skipped.
pub struct DashPaySyncManager {
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    /// Cancel token for the background loop, if running.
    background_cancel: StdMutex<Option<CancellationToken>>,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Set by [`quiesce`](Self::quiesce) to gate new passes while it
    /// drains an in-flight one. `sync_now` bails (after taking the
    /// `is_syncing` slot) when this is set, so once `quiesce` observes
    /// `is_syncing == false` no further pass can start — giving shutdown
    /// a real "no more host-visible persister stores" barrier that
    /// cancel-only [`stop`](Self::stop) does not provide.
    quiescing: AtomicBool,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

impl DashPaySyncManager {
    pub fn new(wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>) -> Self {
        Self {
            wallets,
            background_cancel: StdMutex::new(None),
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
    /// The loop runs on a dedicated OS thread, not on a tokio worker,
    /// because the SDK futures driven by `dashpay_sync` are `!Send` (the
    /// gRPC client state inside the SDK isn't `Send + Sync`), so they
    /// can't ride on `tokio::spawn`, which demands `Future: Send +
    /// 'static`. We use [`tokio::runtime::Handle::block_on`] so the
    /// future still has access to the main runtime's reactor for network
    /// I/O — only the polling thread is dedicated. Mirrors the address-
    /// and identity-sync coordinators.
    ///
    /// The first pass runs immediately; subsequent passes fire every
    /// [`interval`](Self::interval).
    pub fn start(self: Arc<Self>) {
        let mut guard = self.background_cancel.lock().expect("bg_cancel poisoned");
        if guard.is_some() {
            return;
        }
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());
        drop(guard);

        let handle = tokio::runtime::Handle::current();
        let this = self;
        std::thread::Builder::new()
            .name("dashpay-sync".into())
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

                    if let Ok(mut guard) = this.background_cancel.lock() {
                        *guard = None;
                    }
                });
            })
            .expect("failed to spawn dashpay-sync thread");
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: requests cancellation and returns immediately. A
    /// pass already inside `sync_now` keeps running to completion,
    /// including its per-wallet persister fan-out. For a real "nothing
    /// is running and nothing more will be persisted" barrier — required
    /// by manager shutdown so the host can free the persister context —
    /// use [`quiesce`](Self::quiesce).
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

    /// Cancel the background loop **and wait for any in-flight sync pass
    /// to fully drain** before returning — a real quiescence barrier,
    /// unlike cancel-only [`stop`](Self::stop).
    ///
    /// After this returns, no sync pass is running and none can start
    /// until the next [`start`](Self::start) / `sync_now`, so a caller
    /// that immediately tears the manager down (and frees the host-owned
    /// persister context the FFI handed to us) cannot be raced by a pass
    /// that calls `persister.store(...)` through a now-dangling pointer.
    ///
    /// Mechanism: set the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the per-wallet persister fan-out (`sync_now` clears it
    /// only after every wallet's `dashpay_sync` completes), so its
    /// falling edge (with the gate up) is a sound "fully drained"
    /// signal. The gate is reopened before returning so a later
    /// start/sync works normally.
    pub async fn quiesce(&self) {
        self.quiescing.store(true, Ordering::Release);
        self.stop();
        while self.is_syncing.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.quiescing.store(false, Ordering::Release);
    }

    /// Run one DashPay sync pass across every registered wallet.
    ///
    /// If a pass is already in flight, returns an empty summary and
    /// skips — the caller can inspect [`is_syncing`] to distinguish.
    ///
    /// Iterates **every** wallet in the snapshot (token registry is not
    /// consulted), calling `dashpay_sync()` per wallet. Errors are
    /// logged and recorded in the summary but never abort the sweep.
    pub async fn sync_now(&self) -> DashPaySyncSummary {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return DashPaySyncSummary::default();
        }

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and shutdown gets a true barrier
        // (no further `persister.store(...)` after quiesce returns).
        if self.quiescing.load(Ordering::Acquire) {
            self.is_syncing.store(false, Ordering::Release);
            return DashPaySyncSummary::default();
        }

        let snapshot: Vec<(WalletId, Arc<PlatformWallet>)> = {
            let wallets = self.wallets.read().await;
            wallets.iter().map(|(id, w)| (*id, Arc::clone(w))).collect()
        };

        let mut summary = DashPaySyncSummary::default();
        for (wallet_id, wallet) in snapshot {
            // Log-and-continue per wallet: one wallet's failure must not
            // abort DashPay sync for the others.
            let outcome = match wallet.identity().dashpay_sync().await {
                Ok(()) => WalletDashPaySyncOutcome::Ok,
                Err(e) => {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "DashPay sync failed for wallet; continuing with the rest"
                    );
                    WalletDashPaySyncOutcome::Err(e.to_string())
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

        summary
    }
}

impl std::fmt::Debug for DashPaySyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DashPaySyncManager")
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

    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::PlatformWalletManager;

    // Canonical all-`abandon` BIP-39 test vector. Deterministic, so the
    // wallet id is reproducible across runs.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// No-op persister: these tests don't need the real persistence
    /// pipeline, just a handle satisfying the manager constructor.
    struct NoopPersister;

    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// Build a manager over a mock SDK. None of the tests below reach
    /// the network: the wallets they register carry zero identities, so
    /// each `dashpay_sync()` iterates an empty identity set and returns
    /// `Ok(())` without issuing a query.
    fn make_manager() -> Arc<PlatformWalletManager<NoopPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        Arc::new(PlatformWalletManager::new(sdk, persister, event_handler))
    }

    /// Register a fresh wallet on the manager and return its id.
    /// `Some(0)` birth height skips the SPV-tip lookup so the test never
    /// consults SPV. A fresh wallet carries no managed identities — so
    /// it carries **zero watched tokens** in the
    /// [`IdentitySyncManager`](crate::manager::identity_sync::IdentitySyncManager)
    /// registry, which is exactly the case that registry-driven DashPay
    /// sync would skip.
    async fn register_test_wallet(manager: &Arc<PlatformWalletManager<NoopPersister>>) -> WalletId {
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        let seed_bytes = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet registration should succeed");
        wallet.wallet_id()
    }

    /// **The load-bearing G12 assertion.** A recurring DashPay sync pass
    /// must drive `dashpay_sync()` for **every** registered wallet —
    /// including wallets whose identities watch **zero tokens**.
    ///
    /// `IdentitySyncManager`'s registry skips identities with empty token
    /// lists (`identity_sync.rs` filters `!row.tokens.is_empty()`), so a
    /// DashPay coordinator driven off that registry would never sync a
    /// token-less wallet. This test pins that the sweep is **wallet-driven,
    /// not registry-driven**: the wallet is present in the `wallets` map
    /// but absent from the token registry, and the sweep must still visit
    /// it.
    #[tokio::test]
    async fn recurring_pass_syncs_every_wallet_including_zero_token_identities() {
        let manager = make_manager();
        let wallet_id = register_test_wallet(&manager).await;

        // Precondition: the token registry is empty (the wallet has no
        // identities, hence no watched tokens). A registry-driven sweep
        // would have nothing to iterate and would skip this wallet.
        assert_eq!(
            manager.identity_sync().try_queue_depth().unwrap_or(0),
            0,
            "token registry must be empty — this is the case registry-driven DashPay would skip"
        );

        // Run one recurring DashPay sweep.
        let summary = manager.dashpay_sync().sync_now().await;

        // The wallet was swept despite watching zero tokens.
        assert!(
            summary.wallet_results.contains_key(&wallet_id),
            "recurring DashPay sweep must visit every wallet, including zero-token ones"
        );
        assert_eq!(
            summary.success_count(),
            1,
            "the wallet's sync should succeed"
        );
        assert_eq!(summary.error_count(), 0);
        assert!(
            manager.dashpay_sync().last_sync_unix_seconds().is_some(),
            "a completed pass stamps last_sync_unix"
        );
    }

    /// `sync_now` is re-entrant-safe: a second concurrent call returns an
    /// empty summary and does no work while a pass is in flight. We drive
    /// the real `is_syncing` lifecycle directly (the pass body itself is
    /// network-bound and not easily held open in a unit test).
    #[tokio::test]
    async fn sync_now_is_reentrant_safe() {
        let manager = make_manager();
        let mgr = manager.dashpay_sync_arc();

        // Take the `is_syncing` slot exactly as a real pass does, so the
        // concurrent `sync_now` below observes a pass in flight.
        assert!(
            mgr.is_syncing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            "test should own the is_syncing slot"
        );

        // While the slot is held, `sync_now` must bail with an empty
        // summary rather than running a second overlapping pass.
        let summary = mgr.sync_now().await;
        assert!(
            summary.is_empty(),
            "re-entrant sync_now must return an empty summary"
        );

        // Release the slot — a subsequent pass works normally.
        mgr.is_syncing.store(false, Ordering::Release);
    }

    /// `quiesce()` must not return while a pass is in flight, and must
    /// return promptly once the pass drains — the shutdown barrier that
    /// guarantees no further `dashpay_sync()`/persister fan-out runs
    /// after the manager is torn down.
    ///
    /// Drives the real `is_syncing` lifecycle: a background task takes the
    /// slot via the same `compare_exchange` the real `sync_now` uses,
    /// holds it across a sleep (standing in for the pass body), then
    /// clears it. We assert `quiesce()` is still pending while the flag is
    /// held and completes after it falls.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_blocks_until_in_flight_pass_drains() {
        let manager = make_manager();
        let mgr = manager.dashpay_sync_arc();

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

        // While the pass holds the flag, quiesce must stay pending.
        tokio::select! {
            _ = &mut quiesce_fut => panic!("quiesce returned while a pass was in flight"),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
        assert!(mgr.is_syncing(), "pass should still be in flight");

        // Once the pass drains, quiesce must return.
        tokio::time::timeout(Duration::from_secs(2), &mut quiesce_fut)
            .await
            .expect("quiesce did not return after the pass drained");

        assert!(!mgr.quiescing.load(Ordering::Acquire));
        assert!(!mgr.is_syncing());
        pass.await.unwrap();
    }

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — the gate that prevents a pass slipping in
    /// between `quiesce`'s `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let manager = make_manager();
        let _wallet_id = register_test_wallet(&manager).await;
        let mgr = manager.dashpay_sync_arc();

        // Raise the gate as `quiesce()` would.
        mgr.quiescing.store(true, Ordering::Release);

        let summary = mgr.sync_now().await;

        // Empty summary (the registered wallet was NOT swept), slot
        // released so a later (post-quiesce) pass can still run.
        assert!(summary.is_empty());
        assert!(!mgr.is_syncing());
    }

    /// `set_interval` clamps to >=1s and round-trips through `interval`.
    /// The default matches the documented constant.
    #[tokio::test]
    async fn interval_round_trip() {
        let manager = make_manager();
        let mgr = manager.dashpay_sync_arc();

        assert_eq!(
            mgr.interval(),
            Duration::from_secs(DEFAULT_SYNC_INTERVAL_SECS)
        );

        mgr.set_interval(Duration::from_secs(0));
        assert_eq!(mgr.interval(), Duration::from_secs(1));

        mgr.set_interval(Duration::from_secs(120));
        assert_eq!(mgr.interval(), Duration::from_secs(120));
    }
}
