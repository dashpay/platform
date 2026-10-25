//! Policy of the host TXO-store reconcile (dashpay/platform#4638).
//!
//! A host keeps a write-behind mirror of the wallet's coins and restores
//! the engine from it at every launch, so a coin the mirror lost, or a row
//! it holds unspent for a coin the engine proves spent, outlives a full
//! scan. The repair walks the host's own store, but every decision in it
//! lives here, so each host asks the same questions and gets the same
//! answers:
//!
//! - **when to run** — [`TxoReconciler::due_wallets`] on every SPV progress
//!   poll: each wallet on the rising edge into steady state, then again on
//!   [`TXO_RECONCILE_CADENCE`] while it lasts;
//! - **whether a run may start** — [`TxoReconciler::begin`]: one run per
//!   wallet at a time, SPV in steady state with a scan tip, no latched sync
//!   fault, and the wallet's own scan watermark within
//!   [`TXO_RECONCILE_TIP_MARGIN`] blocks of the tip;
//! - **which engine coins may be healed into the store** — [`txo_heal_page`]
//!   tags each inventory row with a [`TxoHealVerdict`];
//! - **how long to retry a page a persistence round staled** —
//!   [`txo_reconcile_retry_stale_page`].
//!
//! Which store rows to mark spent is
//! [`classify_outpoints`](crate::manager::accessors::classify_outpoints)'s
//! verdict. The host keeps only what needs its store: paging its rows,
//! resolving the account that owns each, its transactions, and the fences
//! that keep its persistence callbacks and teardown out of a run's writes.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::changeset::PlatformWalletPersistence;
use crate::manager::accessors::{wallet_utxos_page, WalletUtxoCursor, WalletUtxoRow};
use crate::manager::PlatformWalletManager;
use crate::spv::{ProgressPercentage, SpvRuntime, SyncProgress, SyncState};
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Confirmations an engine coin needs before a host may heal it into its
/// store: coinbase maturity, and well past any plausible reorg.
pub const TXO_RECONCILE_MIN_CONFIRMATIONS: u32 = 100;

/// How far a wallet's scan watermark may trail the scan tip for the scan to
/// count as complete for that wallet.
pub const TXO_RECONCILE_TIP_MARGIN: u32 = 6;

/// How often each wallet is reconciled again while SPV stays in steady
/// state.
pub const TXO_RECONCILE_CADENCE: Duration = Duration::from_secs(30 * 60);

/// Times in a row one page may be classified again because a persistence
/// round committed between its read and its apply, or was still open at
/// it, before the run gives up.
pub const TXO_RECONCILE_MAX_STALE_RETRIES: u32 = 5;

/// One SPV phase's heights, as dash-spv reports them for progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseHeights {
    pub current: u32,
    pub target: u32,
}

/// The part of an SPV progress read the reconcile schedule depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanProgress {
    pub state: SyncState,
    /// Absent until the filter manager has started.
    pub filters: Option<PhaseHeights>,
    /// Absent until the header manager has started.
    pub headers: Option<PhaseHeights>,
}

impl ScanProgress {
    pub fn from_sync_progress(progress: &SyncProgress) -> Self {
        let heights = |current: u32, target: u32| PhaseHeights { current, target };
        Self {
            state: progress.state(),
            filters: progress
                .filters()
                .ok()
                .map(|f| heights(f.current_height(), f.target_height())),
            headers: progress
                .headers()
                .ok()
                .map(|h| heights(h.current_height(), h.target_height())),
        }
    }

    /// Whether the scan is finished. dash-spv's steady state for a fully
    /// synced client is `WaitForEvents` with the filter phase at its target
    /// height; `Synced` is the transient window before it. Either counts —
    /// anything else is a scan still running, or none at all.
    pub fn is_steady(&self) -> bool {
        match self.state {
            SyncState::Synced => true,
            SyncState::WaitForEvents => self
                .filters
                .is_some_and(|f| f.target > 0 && f.current >= f.target),
            SyncState::WaitingForConnections | SyncState::Syncing | SyncState::Error => false,
        }
    }

    /// The height the scan reached: the filter phase's (the wallet-relevant
    /// one), falling back to the header tip when the filter phase has no
    /// height; `None` when neither has one.
    pub fn scan_tip(&self) -> Option<u32> {
        let positive = |phase: Option<PhaseHeights>| phase.map(|p| p.current).filter(|h| *h > 0);
        positive(self.filters).or_else(|| positive(self.headers))
    }
}

/// Why [`TxoReconciler::begin`] refused a run. Nothing was read or written.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxoReconcileSkip {
    /// The manager does not hold the wallet.
    WalletUnknown = 1,
    /// A run for this wallet is already in flight.
    AlreadyRunning = 2,
    /// SPV is not running, or not in its steady state
    /// ([`ScanProgress::is_steady`]).
    NotSteadyState = 3,
    /// The progress carries no scan tip.
    TipUnavailable = 4,
    /// A persistence round was rejected: rows are missing by design and a
    /// rescan is pending ([`PlatformWalletManager::sync_fault_detected`]).
    SyncFaultDetected = 5,
    /// The wallet's scan watermark trails the scan tip by more than
    /// [`TXO_RECONCILE_TIP_MARGIN`] blocks: it is still being scanned.
    WalletBehindTip = 6,
}

impl TxoReconcileSkip {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// The SPV half of the start gate: steady state, a scan tip, and no latched
/// sync fault, in that order. Returns the scan tip a run is measured
/// against.
pub fn txo_reconcile_scan_gate(
    progress: Option<ScanProgress>,
    sync_fault: bool,
) -> Result<u32, TxoReconcileSkip> {
    let progress = progress
        .filter(ScanProgress::is_steady)
        .ok_or(TxoReconcileSkip::NotSteadyState)?;
    let tip = progress
        .scan_tip()
        .ok_or(TxoReconcileSkip::TipUnavailable)?;
    if sync_fault {
        return Err(TxoReconcileSkip::SyncFaultDetected);
    }
    Ok(tip)
}

/// The wallet half of the start gate, against the wallet's scan watermark
/// as the engine holds it (`None` when the engine does not hold the
/// wallet). The tip says how far the CLIENT got; a wallet added behind it
/// is still being scanned. The engine's watermark, not a host's persisted
/// copy: that trails the engine by up to one persistence round, and the
/// transition into steady state lands inside that window.
pub fn txo_reconcile_wallet_gate(
    wallet_synced_height: Option<u32>,
    tip_height: u32,
) -> Result<(), TxoReconcileSkip> {
    let synced = wallet_synced_height.ok_or(TxoReconcileSkip::WalletUnknown)?;
    if synced.saturating_add(TXO_RECONCILE_TIP_MARGIN) >= tip_height {
        Ok(())
    } else {
        Err(TxoReconcileSkip::WalletBehindTip)
    }
}

/// Whether a host may heal one engine coin into its store — see
/// [`txo_heal_verdict`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxoHealVerdict {
    /// No script or no address: the store could not file the row.
    Invalid = 0,
    /// The engine does not call it confirmed, or it is fewer than
    /// [`TXO_RECONCILE_MIN_CONFIRMATIONS`] deep at the tip. A fresh coin can
    /// still reorg out (or, for coinbase, be immature); it ages into a later
    /// run.
    Immature = 1,
    /// May be inserted unspent, if the store lacks the outpoint and can
    /// resolve the account that owns it.
    Eligible = 2,
}

impl TxoHealVerdict {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// The verdict on one inventory row at `tip_height`. The watch-only contact
/// exclusion is not repeated here: [`wallet_utxos_page`] never offers a
/// contact's coin.
pub fn txo_heal_verdict(row: &WalletUtxoRow, tip_height: u32) -> TxoHealVerdict {
    if row.script_pubkey.is_empty() || row.address.is_empty() {
        return TxoHealVerdict::Invalid;
    }
    let deep_enough = row.height > 0
        && tip_height >= row.height
        && tip_height - row.height + 1 >= TXO_RECONCILE_MIN_CONFIRMATIONS;
    if row.is_confirmed && deep_enough {
        TxoHealVerdict::Eligible
    } else {
        TxoHealVerdict::Immature
    }
}

/// One page of `wallet_id`'s inventory ([`wallet_utxos_page`], same order,
/// cursor and bounds), each row with its [`TxoHealVerdict`] at
/// `tip_height` — the tip [`TxoReconciler::begin`] returned for the run.
pub fn txo_heal_page(
    wm: &WalletManager<PlatformWalletInfo>,
    wallet_id: &WalletId,
    after: Option<&WalletUtxoCursor>,
    limit: usize,
    tip_height: u32,
) -> (Vec<(WalletUtxoRow, TxoHealVerdict)>, bool) {
    let (rows, has_more) = wallet_utxos_page(wm, wallet_id, after, limit);
    let rows = rows
        .into_iter()
        .map(|row| {
            let verdict = txo_heal_verdict(&row, tip_height);
            (row, verdict)
        })
        .collect();
    (rows, has_more)
}

/// Whether a run classifies a page again after it came back stale for the
/// `stale_in_a_row`-th time in a row (counting this one), or stops
/// incomplete. A round committing between a page's read and its apply may
/// have re-credited one of its coins, so nothing is written and the page is
/// read again; a store under constant write load must not hold the run
/// forever.
pub fn txo_reconcile_retry_stale_page(stale_in_a_row: u32) -> bool {
    stale_in_a_row <= TXO_RECONCILE_MAX_STALE_RETRIES
}

/// The scheduling state behind [`TxoReconciler`]: the steady-state edge,
/// the runs in flight and each wallet's last run.
#[derive(Debug, Default)]
pub struct TxoReconcileSchedule {
    /// The SPV run session steady state was last seen in; `None` while the
    /// client is not steady. A new session is a new steady state even when
    /// no non-steady read was seen in between (a stop and restart quicker
    /// than the host's poll).
    steady_session: Option<u64>,
    in_flight: BTreeSet<WalletId>,
    last_run: BTreeMap<WalletId, Instant>,
}

impl TxoReconcileSchedule {
    /// Feed one progress observation (every poll, not only a changed one:
    /// this is the schedule's only clock, and a quiet steady-state client's
    /// progress does not change for the whole cadence). Returns the wallets
    /// to run now — all of `wallets` on the rising edge into steady state,
    /// and afterwards each wallet whose last run is at least
    /// [`TXO_RECONCILE_CADENCE`] old or that never ran — never one already
    /// in flight. Reporting a wallet consumes nothing: its cadence restarts
    /// only when a run claims it ([`Self::try_begin`]), so a wallet the host
    /// cannot run yet is reported again on the next poll.
    pub fn due_wallets(
        &mut self,
        steady: bool,
        spv_session: u64,
        wallets: impl IntoIterator<Item = WalletId>,
        now: Instant,
    ) -> Vec<WalletId> {
        if !steady {
            self.steady_session = None;
            return Vec::new();
        }
        let rising = self.steady_session != Some(spv_session);
        self.steady_session = Some(spv_session);
        wallets
            .into_iter()
            .filter(|wallet_id| !self.in_flight.contains(wallet_id))
            .filter(|wallet_id| {
                rising
                    || self.last_run.get(wallet_id).is_none_or(|last| {
                        now.saturating_duration_since(*last) >= TXO_RECONCILE_CADENCE
                    })
            })
            .collect()
    }

    /// Claim the wallet for one run at `now`; `false` when a run is already
    /// in flight. The claim restarts the wallet's cadence whether or not the
    /// run goes ahead, so a run its gate refuses is not retried on every
    /// poll.
    pub fn try_begin(&mut self, wallet_id: WalletId, now: Instant) -> bool {
        if !self.in_flight.insert(wallet_id) {
            return false;
        }
        self.last_run.insert(wallet_id, now);
        true
    }

    /// Release the claim [`Self::try_begin`] took. `ran_at` is when a run
    /// that went ahead finished, which restarts the wallet's cadence from
    /// then; `None` for one that did not.
    pub fn finish(&mut self, wallet_id: &WalletId, ran_at: Option<Instant>) {
        self.in_flight.remove(wallet_id);
        if let Some(at) = ran_at {
            self.last_run.insert(*wallet_id, at);
        }
    }

    /// Forget a removed wallet's cadence, so one re-added under the same id
    /// is due at once.
    pub fn forget(&mut self, wallet_id: &WalletId) {
        self.last_run.remove(wallet_id);
    }
}

/// Lock the schedule, recovering from poisoning: every method leaves it
/// consistent at each step, so a panicking holder cannot tear it.
fn lock_schedule(schedule: &Mutex<TxoReconcileSchedule>) -> MutexGuard<'_, TxoReconcileSchedule> {
    schedule.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A manager's reconcile schedule together with the state its gates read.
/// Cheap to clone; obtained from [`PlatformWalletManager::txo_reconciler`]
/// so a caller can wait on SPV without borrowing the manager.
#[derive(Clone)]
pub struct TxoReconciler {
    schedule: Arc<Mutex<TxoReconcileSchedule>>,
    spv: Arc<SpvRuntime>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    sync_fault: Arc<AtomicBool>,
}

impl TxoReconciler {
    /// The wallets to reconcile now — see [`TxoReconcileSchedule::due_wallets`].
    /// SPV not running counts as not steady, which arms the next rising
    /// edge. Call on every progress poll, running or not.
    pub async fn due_wallets(&self) -> Vec<WalletId> {
        let session = self.spv.run_session();
        let steady = self.scan_progress().await.is_some_and(|p| p.is_steady());
        let wallets: Vec<WalletId> = self.wallets.load().keys().copied().collect();
        lock_schedule(&self.schedule).due_wallets(steady, session, wallets, Instant::now())
    }

    /// Claim `wallet_id` for one run and check that it may start, in order:
    /// no run already in flight, SPV in steady state with a scan tip, no
    /// latched sync fault, and the wallet's scan watermark within
    /// [`TXO_RECONCILE_TIP_MARGIN`] blocks of the tip. Returns the scan tip
    /// the run's heal verdicts are measured against ([`txo_heal_page`]).
    /// On `Ok` the claim holds until [`Self::finish`]; a refusal releases
    /// it.
    pub async fn begin(&self, wallet_id: &WalletId) -> Result<u32, TxoReconcileSkip> {
        if !lock_schedule(&self.schedule).try_begin(*wallet_id, Instant::now()) {
            return Err(TxoReconcileSkip::AlreadyRunning);
        }
        let gate = self.gate(wallet_id).await;
        if gate.is_err() {
            lock_schedule(&self.schedule).finish(wallet_id, None);
        }
        gate
    }

    /// Release the claim [`Self::begin`] granted. `ran` is whether the run
    /// went ahead (even if it stopped early): that restarts the wallet's
    /// cadence from now.
    pub fn finish(&self, wallet_id: &WalletId, ran: bool) {
        lock_schedule(&self.schedule).finish(wallet_id, ran.then(Instant::now));
    }

    async fn gate(&self, wallet_id: &WalletId) -> Result<u32, TxoReconcileSkip> {
        let tip = txo_reconcile_scan_gate(
            self.scan_progress().await,
            self.sync_fault.load(Ordering::Relaxed),
        )?;
        let synced_height = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(wallet_id)
                .map(|info| info.core_wallet.metadata.synced_height)
        };
        txo_reconcile_wallet_gate(synced_height, tip)?;
        Ok(tip)
    }

    /// The client's progress, or `None` when it is not running.
    async fn scan_progress(&self) -> Option<ScanProgress> {
        if !self.spv.is_running() {
            return None;
        }
        self.spv
            .sync_progress()
            .await
            .map(|progress| ScanProgress::from_sync_progress(&progress))
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// This manager's TXO-store reconcile schedule and gates.
    pub fn txo_reconciler(&self) -> TxoReconciler {
        TxoReconciler {
            schedule: Arc::clone(&self.txo_reconcile_schedule),
            spv: Arc::clone(&self.spv_manager),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallets: Arc::clone(&self.wallets),
            sync_fault: Arc::clone(&self.sync_fault),
        }
    }

    /// Drop a removed wallet's reconcile cadence.
    pub(super) fn forget_txo_reconcile(&self, wallet_id: &WalletId) {
        lock_schedule(&self.txo_reconcile_schedule).forget(wallet_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Txid};
    use key_wallet::account::{AccountType, StandardAccountType};

    use crate::events::PlatformEventManager;

    const TIP: u32 = 2_535_898;

    fn progress(state: SyncState, filters: Option<(u32, u32)>, headers: u32) -> ScanProgress {
        ScanProgress {
            state,
            filters: filters.map(|(current, target)| PhaseHeights { current, target }),
            headers: Some(PhaseHeights {
                current: headers,
                target: headers,
            }),
        }
    }

    fn steady() -> ScanProgress {
        progress(SyncState::WaitForEvents, Some((TIP, TIP)), TIP)
    }

    fn syncing() -> ScanProgress {
        progress(SyncState::Syncing, Some((199_999, TIP)), TIP)
    }

    /// What a client that is not running reports.
    fn not_running() -> ScanProgress {
        ScanProgress {
            state: SyncState::WaitForEvents,
            filters: None,
            headers: None,
        }
    }

    #[test]
    fn the_steady_state_gate_refuses_an_unfinished_scan() {
        assert!(!syncing().is_steady());
        assert!(!progress(SyncState::WaitingForConnections, None, TIP).is_steady());
        assert!(!progress(SyncState::Error, Some((TIP, TIP)), TIP).is_steady());
        assert!(
            !progress(SyncState::WaitForEvents, Some((2_535_800, TIP)), TIP).is_steady(),
            "waiting for events with the filter phase behind its target is a scan still running"
        );
        assert!(
            !progress(SyncState::WaitForEvents, None, TIP).is_steady(),
            "no filter phase at all proves nothing"
        );
        assert!(
            !progress(SyncState::WaitForEvents, Some((0, 0)), TIP).is_steady(),
            "a filter phase with no target proves nothing"
        );
        assert!(!not_running().is_steady());
        assert!(progress(SyncState::Synced, Some((TIP, TIP)), TIP).is_steady());
        assert!(steady().is_steady(), "dash-spv's fully synced steady state");
    }

    #[test]
    fn the_tip_is_the_filter_height_falling_back_to_the_header_tip() {
        assert_eq!(steady().scan_tip(), Some(TIP));
        assert_eq!(
            progress(SyncState::Synced, None, TIP).scan_tip(),
            Some(TIP),
            "falls back to the header tip"
        );
        assert_eq!(
            progress(SyncState::Synced, Some((0, 0)), 2_535_000).scan_tip(),
            Some(2_535_000),
            "a filter phase at height 0 falls back too"
        );
        assert_eq!(not_running().scan_tip(), None);
    }

    #[test]
    fn a_default_progress_read_is_not_steady_and_has_no_tip() {
        let read = ScanProgress::from_sync_progress(&SyncProgress::default());
        assert_eq!(read, not_running());
    }

    #[test]
    fn the_scan_gate_checks_steady_state_then_the_tip_then_the_fault() {
        assert_eq!(txo_reconcile_scan_gate(Some(steady()), false), Ok(TIP));
        assert_eq!(
            txo_reconcile_scan_gate(None, false),
            Err(TxoReconcileSkip::NotSteadyState),
            "a client that is not running is not steady"
        );
        assert_eq!(
            txo_reconcile_scan_gate(Some(syncing()), true),
            Err(TxoReconcileSkip::NotSteadyState)
        );
        let synced_without_heights = ScanProgress {
            state: SyncState::Synced,
            filters: None,
            headers: None,
        };
        assert_eq!(
            txo_reconcile_scan_gate(Some(synced_without_heights), true),
            Err(TxoReconcileSkip::TipUnavailable)
        );
        assert_eq!(
            txo_reconcile_scan_gate(Some(steady()), true),
            Err(TxoReconcileSkip::SyncFaultDetected),
            "a rejected round means rows are missing by design"
        );
    }

    #[test]
    fn the_wallet_gate_uses_the_engines_watermark_within_the_tip_margin() {
        // `synced_height + margin >= tip`.
        assert_eq!(
            txo_reconcile_wallet_gate(Some(1_565_211), 1_565_211),
            Ok(())
        );
        assert_eq!(
            txo_reconcile_wallet_gate(Some(1_565_205), 1_565_211),
            Ok(())
        );
        assert_eq!(
            txo_reconcile_wallet_gate(Some(1_565_204), 1_565_211),
            Err(TxoReconcileSkip::WalletBehindTip)
        );
        assert_eq!(
            txo_reconcile_wallet_gate(Some(0), 1_565_211),
            Err(TxoReconcileSkip::WalletBehindTip),
            "a wallet added behind the tip is still being scanned"
        );
        assert_eq!(
            txo_reconcile_wallet_gate(None, 3),
            Err(TxoReconcileSkip::WalletUnknown)
        );
        assert_eq!(
            txo_reconcile_wallet_gate(Some(u32::MAX), u32::MAX),
            Ok(()),
            "the margin saturates rather than overflowing"
        );
    }

    fn row(height: u32, is_confirmed: bool) -> WalletUtxoRow {
        let mut script = vec![0x76, 0xa9, 0x14];
        script.extend([0x5b; 20]);
        script.extend([0x88, 0xac]);
        WalletUtxoRow {
            account_type: AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            outpoint: OutPoint {
                txid: Txid::from_byte_array([0x11; 32]),
                vout: 0,
            },
            value_duffs: 19_549,
            script_pubkey: script,
            address: "yStxXHHzhAx58JhaPBNhn3xsH93UwBM2nd".to_owned(),
            height,
            is_confirmed,
            is_instantlocked: false,
            is_coinbase: false,
            is_locked: false,
        }
    }

    #[test]
    fn a_coin_is_healed_only_at_one_hundred_engine_confirmations() {
        let tip = 1_000_000;
        assert_eq!(
            txo_heal_verdict(&row(tip - 99, true), tip),
            TxoHealVerdict::Eligible,
            "the tip block counts: 100 confirmations"
        );
        assert_eq!(
            txo_heal_verdict(&row(tip - 98, true), tip),
            TxoHealVerdict::Immature,
            "99 confirmations ages into a later run"
        );
        assert_eq!(
            txo_heal_verdict(&row(tip - 500, false), tip),
            TxoHealVerdict::Immature,
            "a coin the engine does not call confirmed is never healed, however deep"
        );
        assert_eq!(
            txo_heal_verdict(&row(0, true), tip),
            TxoHealVerdict::Immature,
            "no height"
        );
        assert_eq!(
            txo_heal_verdict(&row(tip + 1, true), tip),
            TxoHealVerdict::Immature,
            "above the tip"
        );
    }

    #[test]
    fn a_coin_the_store_could_not_file_is_invalid() {
        let tip = 1_000_000;
        let mut no_script = row(500_000, true);
        no_script.script_pubkey.clear();
        assert_eq!(txo_heal_verdict(&no_script, tip), TxoHealVerdict::Invalid);
        let mut no_address = row(500_000, true);
        no_address.address.clear();
        assert_eq!(txo_heal_verdict(&no_address, tip), TxoHealVerdict::Invalid);
        let mut immature_and_invalid = row(tip, false);
        immature_and_invalid.address.clear();
        assert_eq!(
            txo_heal_verdict(&immature_and_invalid, tip),
            TxoHealVerdict::Invalid,
            "validity is judged first"
        );
    }

    #[test]
    fn an_unknown_wallet_is_an_empty_terminal_heal_page() {
        let wm = WalletManager::<PlatformWalletInfo>::new(dashcore::Network::Testnet);
        let (rows, has_more) = txo_heal_page(&wm, &[0xEE; 32], None, 10, TIP);
        assert!(rows.is_empty());
        assert!(!has_more);
    }

    #[test]
    fn a_stale_page_is_retried_five_times_in_a_row_then_the_run_stops() {
        for stale_in_a_row in 1..=5 {
            assert!(txo_reconcile_retry_stale_page(stale_in_a_row));
        }
        assert!(!txo_reconcile_retry_stale_page(6));
        assert_eq!(TXO_RECONCILE_MAX_STALE_RETRIES, 5);
    }

    const A: WalletId = [0xA1; 32];
    const B: WalletId = [0xB2; 32];

    /// What a host does with the wallets it is told are due: claims each at
    /// `at` and finishes it there.
    fn run(schedule: &mut TxoReconcileSchedule, due: &[WalletId], at: Instant) {
        for wallet_id in due {
            assert!(schedule.try_begin(*wallet_id, at));
            schedule.finish(wallet_id, Some(at));
        }
    }

    #[test]
    fn every_wallet_runs_on_the_rising_edge_into_steady_state() {
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert!(schedule.due_wallets(false, 1, [A, B], t0).is_empty());
        assert_eq!(schedule.due_wallets(true, 1, [A, B], t0), vec![A, B]);
        run(&mut schedule, &[A, B], t0);
        assert!(
            schedule
                .due_wallets(true, 1, [A, B], t0 + Duration::from_secs(1))
                .is_empty(),
            "the next poll inside the cadence schedules nothing"
        );
    }

    #[test]
    fn a_steady_client_runs_each_wallet_again_on_its_own_cadence() {
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        let half = TXO_RECONCILE_CADENCE / 2;
        assert_eq!(schedule.due_wallets(true, 1, [A], t0), vec![A]);
        run(&mut schedule, &[A], t0);
        // A wallet loaded while steady never ran: it is due at once, on its
        // own clock.
        assert_eq!(schedule.due_wallets(true, 1, [A, B], t0 + half), vec![B]);
        run(&mut schedule, &[B], t0 + half);
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], t0 + TXO_RECONCILE_CADENCE),
            vec![A]
        );
        run(&mut schedule, &[A], t0 + TXO_RECONCILE_CADENCE);
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], t0 + half + TXO_RECONCILE_CADENCE),
            vec![B]
        );
        run(&mut schedule, &[B], t0 + half + TXO_RECONCILE_CADENCE);
        // A finished run restarts that wallet's cadence from its finish.
        let a_ran = t0 + TXO_RECONCILE_CADENCE + half + half / 2;
        assert!(schedule.try_begin(A, a_ran - Duration::from_secs(60)));
        schedule.finish(&A, Some(a_ran));
        assert!(schedule
            .due_wallets(true, 1, [A, B], t0 + 2 * TXO_RECONCILE_CADENCE)
            .is_empty());
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], a_ran + TXO_RECONCILE_CADENCE),
            vec![A, B]
        );
    }

    #[test]
    fn a_wallet_with_a_run_in_flight_is_never_scheduled_or_started_again() {
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert!(schedule.try_begin(A, t0));
        assert!(
            !schedule.try_begin(A, t0),
            "a second run for the same wallet is refused"
        );
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], t0),
            vec![B],
            "not even on the rising edge"
        );
        schedule.finish(&A, None);
        assert!(schedule.try_begin(A, t0));
    }

    #[test]
    fn a_wallet_the_host_skips_is_due_again_on_the_next_poll() {
        // The host learns of a wallet the engine already holds before it
        // can run it (its own wrapper is still being built), and skips it.
        // Being reported spent none of its cadence.
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        let second = Duration::from_secs(1);
        assert_eq!(schedule.due_wallets(true, 1, [A, B], t0), vec![A, B]);
        run(&mut schedule, &[A], t0);
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], t0 + second),
            vec![B],
            "the skipped wallet is reported again"
        );
        assert_eq!(
            schedule.due_wallets(true, 1, [A, B], t0 + 2 * second),
            vec![B],
            "and again, until a run claims it"
        );
        run(&mut schedule, &[B], t0 + 2 * second);
        assert!(schedule
            .due_wallets(true, 1, [A, B], t0 + 3 * second)
            .is_empty());
    }

    #[test]
    fn a_claim_the_gate_refuses_still_restarts_the_cadence() {
        // A refused run is not retried on every poll: the claim itself
        // restarts the cadence.
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert_eq!(schedule.due_wallets(true, 1, [A], t0), vec![A]);
        assert!(schedule.try_begin(A, t0));
        schedule.finish(&A, None);
        assert!(schedule
            .due_wallets(true, 1, [A], t0 + Duration::from_secs(1))
            .is_empty());
        assert_eq!(
            schedule.due_wallets(true, 1, [A], t0 + TXO_RECONCILE_CADENCE),
            vec![A]
        );
    }

    #[test]
    fn leaving_steady_state_arms_the_next_rising_edge() {
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert_eq!(schedule.due_wallets(true, 1, [A], t0), vec![A]);
        run(&mut schedule, &[A], t0);
        assert!(schedule
            .due_wallets(false, 1, [A], t0 + Duration::from_secs(1))
            .is_empty());
        assert_eq!(
            schedule.due_wallets(true, 1, [A], t0 + Duration::from_secs(2)),
            vec![A],
            "back in steady state: due again, cadence or not"
        );
    }

    #[test]
    fn a_restart_between_two_polls_is_a_new_steady_state() {
        // SPV stops and restarts straight into steady state between two
        // polls: no poll saw it stopped, but the run session moved.
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert_eq!(schedule.due_wallets(true, 1, [A, B], t0), vec![A, B]);
        run(&mut schedule, &[A, B], t0);
        assert!(schedule
            .due_wallets(true, 1, [A, B], t0 + Duration::from_secs(1))
            .is_empty());
        assert_eq!(
            schedule.due_wallets(true, 2, [A, B], t0 + Duration::from_secs(2)),
            vec![A, B],
            "the first steady read after the restart is a new steady state"
        );
        run(&mut schedule, &[A, B], t0 + Duration::from_secs(2));
        assert!(
            schedule
                .due_wallets(true, 2, [A, B], t0 + Duration::from_secs(3))
                .is_empty(),
            "and only the first"
        );
    }

    #[test]
    fn a_forgotten_wallet_is_due_at_once() {
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        assert_eq!(schedule.due_wallets(true, 1, [A], t0), vec![A]);
        run(&mut schedule, &[A], t0);
        schedule.forget(&A);
        assert_eq!(
            schedule.due_wallets(true, 1, [A], t0 + Duration::from_secs(1)),
            vec![A],
            "a re-added wallet is due at once"
        );
    }

    fn reconciler() -> TxoReconciler {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            dashcore::Network::Testnet,
        )));
        let spv = Arc::new(SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::new(PlatformEventManager::new(Vec::new())),
        ));
        TxoReconciler {
            schedule: Arc::default(),
            spv,
            wallet_manager,
            wallets: Arc::new(ArcSwap::from_pointee(BTreeMap::new())),
            sync_fault: Arc::new(AtomicBool::new(false)),
        }
    }

    #[tokio::test]
    async fn a_stopped_client_schedules_nothing_and_refuses_a_run() {
        let reconciler = reconciler();
        assert!(reconciler.due_wallets().await.is_empty());
        assert_eq!(
            reconciler.begin(&A).await,
            Err(TxoReconcileSkip::NotSteadyState)
        );
        assert_eq!(
            reconciler.begin(&A).await,
            Err(TxoReconcileSkip::NotSteadyState),
            "a refused run releases its claim"
        );
    }

    #[tokio::test]
    async fn a_granted_claim_holds_until_finish() {
        let reconciler = reconciler();
        assert!(lock_schedule(&reconciler.schedule).try_begin(A, Instant::now()));
        assert_eq!(
            reconciler.begin(&A).await,
            Err(TxoReconcileSkip::AlreadyRunning)
        );
        reconciler.finish(&A, true);
        assert_eq!(
            reconciler.begin(&A).await,
            Err(TxoReconcileSkip::NotSteadyState)
        );
    }
}
