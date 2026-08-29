//! Periodic DashPay (contact-request + profile) sync coordinator.
//!
//! Folds the DashPay refresh into the recurring background loop, alongside
//! the platform-address, identity-token, and shielded coordinators. Before
//! this, contact requests and DashPay profiles only refreshed when the host
//! explicitly called the FFI sync entry point; there was no background DashPay
//! refresh at all.
//!
//! **Wallet-driven, not registry-driven — by design.** This coordinator is a
//! sibling of [`PlatformAddressSyncManager`](super::platform_address_sync::PlatformAddressSyncManager):
//! it holds the same `wallets` map, snapshots the wallet `Arc`s under a read
//! guard each sweep, and refreshes **every** wallet. It deliberately does NOT
//! extend [`IdentitySyncManager`](super::identity_sync::IdentitySyncManager):
//! that one is token-registry-driven and skips identities with no watched
//! tokens, so a DashPay-only identity would never sync under its gating.
//!
//! **The DashPay sync orchestration lives here**, in the coordinator
//! ([`DashPaySyncManager::sync_wallet_dashpay`]): the per-wallet refresh
//! sequences the six DashPay steps (contact requests → own profiles → contact
//! profiles → contactInfo → incoming/sent payment reconciles). Each step is an
//! `IdentityWallet` domain operation (which also has standalone on-demand FFI
//! callers); the coordinator owns only the *sequencing* and the log-and-continue
//! policy.
//!
//! Each pass:
//! 1. Snapshots the wallet map (short read lock, no await while held).
//! 2. Runs [`sync_wallet_dashpay`](DashPaySyncManager::sync_wallet_dashpay) per wallet.
//! 3. Stores the pass timestamp.
//!
//! **Error semantics: log-and-continue per wallet.** A failing per-wallet
//! refresh is logged and recorded in the pass summary; it never aborts the
//! sweep across the other wallets. Within a wallet the six steps run
//! independently — one step's failure doesn't skip the rest — and the
//! per-*identity* continue (so one identity's fetch failure doesn't abort the
//! others within a step) lives inside the steps themselves.
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
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;

use dash_async::{ThreadRegistry, WorkerConfig};

use crate::error::PlatformWalletError;
use crate::manager::{
    coordinator_worker_config, drain_pass, QuiesceGate, QuiesceGuard, SyncSlotGuard, WalletWorker,
    COORDINATOR_DRAIN_BUDGET,
};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Default cadence for the DashPay sync loop.
///
/// Matches Android's `PlatformSyncService` 15s ticker so new contact
/// requests, profiles, and payments surface within ~15s. The fetch is
/// incremental (high-water cursor + overlap) and profiles are throttled by
/// their own refresh window, so the tighter cadence does not multiply DAPI
/// traffic by 4. Tunable at runtime via [`DashPaySyncManager::set_interval`].
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 15;

/// Stack size for the DashPay sync loop's OS thread.
///
/// DashPay sync verifies GroveDB *document-query* proofs (contactRequest /
/// profile fetches), whose recursive `verify_layer_proof_v1` descent
/// overflows the platform default thread stack (SIGBUS on the stack guard,
/// observed on-device 2026-06-12). The sibling sync loops survive on the
/// default only because their proofs are shallower. Matches the FFI worker
/// convention (`runtime.rs` WORKER_STACK_BYTES) since `Handle::block_on`
/// polls the future on the registry's worker thread.
const DASHPAY_SYNC_STACK_BYTES: usize = 8 * 1024 * 1024;

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
    /// Shared registry that owns this loop's lifecycle: it spawns the
    /// OS thread (with the deep-stack config below), owns its cancellation
    /// token, and joins it at shutdown. A generation-guarded slot handles a
    /// `stop()` + quick `start()` without a stale loop clobbering the new one.
    registry: Arc<ThreadRegistry<WalletWorker>>,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Gates new passes while a [`quiesce`](Self::quiesce) drains an
    /// in-flight one, while a [`QuiesceGuard`] holder mutates state, and
    /// terminally once shutdown seals it. `sync_now` bails (after taking the
    /// `is_syncing` slot) when it is closed, so once a drain observes
    /// `is_syncing == false` no further pass can start — giving shutdown
    /// a real "no more host-visible persister stores" barrier that
    /// cancel-only [`stop`](Self::stop) does not provide.
    quiescing: QuiesceGate,
    /// Unix seconds of the last completed pass. `0` = never.
    last_sync_unix: AtomicU64,
}

impl DashPaySyncManager {
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            wallets,
            registry,
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: QuiesceGate::default(),
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
        self.registry.is_running(WalletWorker::DashPaySync)
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
    ///
    /// **Blocks briefly on restart**: the shared registry synchronously
    /// reaps a still-draining prior-generation thread, spinning up to the
    /// registry reap backstop (default 1 s) before returning. Call it from
    /// the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        let handle = tokio::runtime::Handle::current();
        let registry = Arc::clone(&self.registry);
        let this = self;
        // Deep stack for the GroveDB proof descent — see
        // [`DASHPAY_SYNC_STACK_BYTES`]. The registry spawns the OS thread
        // with this size and owns the whole lifecycle (see
        // `IdentitySyncManager::start`): teardown latch, cancellation token,
        // thread spawn, and prior-generation reap under one slot lock.
        let cfg = WorkerConfig {
            stack_size: NonZeroUsize::new(DASHPAY_SYNC_STACK_BYTES),
            ..coordinator_worker_config()
        };
        registry.start_thread(WalletWorker::DashPaySync, cfg, move |cancel| {
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

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: requests cancellation and returns immediately. A
    /// pass already inside `sync_now` keeps running to completion,
    /// including its per-wallet persister fan-out. For a real "nothing
    /// is running and nothing more will be persisted" barrier — required
    /// by manager shutdown so the host can free the persister context —
    /// use [`quiesce`](Self::quiesce).
    pub fn stop(&self) {
        self.registry.cancel(WalletWorker::DashPaySync);
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
    /// Mechanism: close the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the per-wallet persister fan-out (`sync_now` clears it
    /// only after every wallet's `dashpay_sync` completes), so its
    /// falling edge (with the gate up) is a sound "fully drained"
    /// signal. The gate is reopened before returning so a later
    /// start/sync works normally.
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
    /// Returns `true` when the drain completed: no pass is running and
    /// none can start until the next `start`/`sync_now`. Returns `false`
    /// when `is_syncing` was still held at the deadline — a pass is
    /// wedged (stalled network / persister / host-callback await). On
    /// that path the `quiescing` gate is deliberately **left closed** so the
    /// wedged pass cannot be followed by a fresh one; the caller must
    /// treat the coordinator as non-clean (shutdown reports it, clear /
    /// reset paths abort fail-closed). A later successful `quiesce`
    /// reopens the gate.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        // The guard drops here, reopening the gate — this is the
        // "drain only" flavor.
        self.quiesce_held_within(budget).await.is_some()
    }

    /// [`quiesce_within`](Self::quiesce_within) that **keeps sync admission
    /// shut** until the returned guard drops — the barrier a caller needs
    /// when it mutates state a pass touches right after draining.
    ///
    /// `None` means the in-flight pass did not drain within `budget`; the
    /// gate is left closed and the caller must fail closed.
    #[must_use = "None means the pass did NOT drain; the caller must fail closed"]
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
        // Clears `is_syncing` on every exit path — including panic unwind —
        // so a failed pass can never wedge `quiesce()`'s drain.
        let _slot = SyncSlotGuard(&self.is_syncing);

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and shutdown gets a true barrier
        // (no further `persister.store(...)` after quiesce returns).
        if self.quiescing.is_closed() {
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
            let outcome = match self.sync_wallet_dashpay(&wallet).await {
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

        summary
    }

    /// Run one wallet's comprehensive DashPay refresh. The orchestration lives
    /// here in the sync coordinator; each step is an `IdentityWallet` domain
    /// operation (each also has its own standalone on-demand FFI caller).
    ///
    /// The six steps run **independently** (log-and-continue) so a failure in
    /// one does not skip the others. The two network *fetch* steps
    /// (`sync_contact_requests`, `sync_profiles`) surface their first error so
    /// the sweep can record this wallet as failed; the remaining steps
    /// (contact profiles, contactInfo, the two payment reconciles) are
    /// display- or local-only and never fail the pass. Contact requests run
    /// first so freshly established contacts' accounts are registered before
    /// the incoming-payment reconcile.
    async fn sync_wallet_dashpay(
        &self,
        wallet: &Arc<PlatformWallet>,
    ) -> Result<(), PlatformWalletError> {
        let identity = wallet.identity();
        let wallet_id = wallet.wallet_id();

        // Contact requests first — may establish new contacts.
        let contact_result = identity.dashpay().sync_contact_requests().await;
        if let Err(e) = &contact_result {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay contact-request sync failed; continuing to profile sync"
            );
        }

        // Own-identity profiles — attempted even if the contact step failed.
        let profile_result = identity.dashpay().sync_profiles().await;
        if let Err(e) = &profile_result {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay profile sync failed"
            );
        }

        // Contact profiles (established contacts + pending senders) for the UI.
        // Distinct target set/cache from own profiles; display-only.
        if let Err(e) = identity.dashpay().sync_contact_profiles().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay contact-profile sync failed"
            );
        }

        // contactInfo (alias/note/hidden) — cross-device metadata.
        if let Err(e) = identity.dashpay().sync_contact_infos().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay contactInfo sync failed"
            );
        }

        // Local-only: derive missing `Received` entries from receival-account
        // UTXOs. After the contact step so newly established accounts exist.
        if let Err(e) = identity.dashpay().reconcile_incoming_payments().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay incoming-payment reconcile failed"
            );
        }

        // Local-only: rebuild missing `Sent` entries from persisted
        // wallet transaction history + the contact external-account
        // address pools. Runs after the incoming reconcile so an
        // existing received entry under the txid wins the dedup guard.
        if let Err(e) = identity
            .dashpay()
            .reconcile_sent_payments_from_tx_history()
            .await
        {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay sent-payment reconstruction failed"
            );
        }

        // Local-only: DIP-15 §12.6 coreHeight backfill — lower SPV synced_height
        // to re-scan for incoming payments that landed on a contact's receival
        // address before it was watched (restore-from-seed / 2nd device /
        // offline-accept→pay). After the reconcile above so newly established
        // receival accounts are visible; a per-contact guard prevents
        // re-triggering and thrashing the in-flight backfill.
        if let Err(e) = identity.dashpay().reconcile_dashpay_rescan().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay rescan reconcile failed"
            );
        }

        // Local-only: confirm `Pending` `Sent` payments the persisted core
        // record reports final (mined or InstantSend-locked).
        if let Err(e) = identity.dashpay().reconcile_sent_payments().await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "DashPay sent-payment reconcile failed"
            );
        }

        // Surface the first fetch error (if any); both fetch steps have run.
        contact_result?;
        profile_result?;
        Ok(())
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
        Arc::new(PlatformWalletManager::new(
            sdk,
            persister,
            vec![event_handler],
        ))
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
                &seed_bytes,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet registration should succeed");
        wallet.wallet_id()
    }

    /// **The load-bearing assertion.** A recurring DashPay sync pass
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

        assert!(!mgr.quiescing.is_closed());
        assert!(!mgr.is_syncing());
        pass.await.unwrap();
    }

    /// A pass that never drains must NOT hang `quiesce_within` forever:
    /// the drain returns `false` at its deadline and deliberately leaves
    /// the `quiescing` gate closed (so the wedged pass cannot be followed by
    /// a fresh one). A later successful quiesce reopens the gate. This is
    /// the bound that keeps FFI `destroy` from blocking indefinitely on a
    /// stalled network / persister await.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_within_times_out_and_leaves_gate_up_when_pass_never_drains() {
        let manager = make_manager();
        let mgr = manager.dashpay_sync_arc();

        // Wedge: take the slot exactly as a real pass would and never
        // release it (stands in for a pass stalled in an await).
        assert!(mgr
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok());

        let drained = tokio::time::timeout(
            Duration::from_secs(2),
            mgr.quiesce_within(Duration::from_millis(100)),
        )
        .await
        .expect("bounded quiesce must return at its deadline");
        assert!(!drained, "a wedged pass must be reported as non-drained");
        assert!(
            mgr.quiescing.is_closed(),
            "gate must stay up after a timed-out drain"
        );

        // Release the wedge; the next quiesce drains and reopens the gate.
        mgr.is_syncing.store(false, Ordering::Release);
        assert!(mgr.quiesce().await);
        assert!(!mgr.quiescing.is_closed());
    }

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — the gate that prevents a pass slipping in
    /// between `quiesce`'s `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let manager = make_manager();
        let _wallet_id = register_test_wallet(&manager).await;
        let mgr = manager.dashpay_sync_arc();

        // Raise the gate as an in-flight `quiesce()` would (a drain holds
        // the gate from its first instruction).
        let gate_hold = mgr.quiescing.hold();

        let summary = mgr.sync_now().await;

        // Empty summary (the registered wallet was NOT swept), slot
        // released so a later (post-quiesce) pass can still run.
        assert!(summary.is_empty());
        assert!(!mgr.is_syncing());
        drop(gate_hold);
    }

    /// Regression: a `stop()` + quick `start()` must leave the NEW loop
    /// running and cancellable — a stale prior generation's exit epilogue
    /// must not clobber the new generation's cancellation token.
    ///
    /// The failure this pins is a use-after-free across the FFI persister:
    /// if the old loop's exit nulled the new loop's token, `is_running()`
    /// would lie (`false` while the new loop runs) and a shutdown
    /// `stop()`/`quiesce()` would silently no-op while the new loop kept
    /// fanning out `persister.store(...)` through a freed context.
    ///
    /// `stop()` / `is_running()` now route through the shared
    /// `ThreadRegistry`, whose generation-guarded slot enforces this: a
    /// restart reaps the prior generation under the start slot lock and the
    /// prior's epilogue is gen-gated. The registry's own
    /// `generation_match_epilogue_preserves_new_token` test pins the
    /// primitive; this one pins the manager wiring through real
    /// `start()`/`stop()` on live OS-thread loops.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn stop_then_quick_start_keeps_new_loop_cancellable() {
        let manager = make_manager();
        let mgr = manager.dashpay_sync_arc();

        // Loop A starts (empty wallet set → each pass is a no-op, no I/O).
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        // stop() cancels loop A; the running flag clears immediately.
        mgr.stop();
        assert!(!mgr.is_running(), "stop() clears the running flag at once");

        // Loop B starts before loop A has necessarily drained. The registry
        // reaps the prior generation under the start slot lock and installs a
        // fresh generation, so A's later epilogue cannot clear B's token.
        Arc::clone(&mgr).start();
        assert!(mgr.is_running(), "loop B must be running after the restart");

        // A real shutdown still cancels loop B and joins it cleanly — proof
        // B stayed cancellable after A's stale exit.
        mgr.stop();
        assert!(!mgr.is_running());
        let report = manager.shutdown().await;
        assert!(
            report.all_clean(),
            "clean shutdown after restart: {report:?}"
        );
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
