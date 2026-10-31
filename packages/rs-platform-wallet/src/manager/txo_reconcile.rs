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
//! - **which engine coins may be healed into the store** —
//!   [`TxoReconciler::heal_page_held`]: [`txo_heal_page`] tags each
//!   inventory row with a [`TxoHealVerdict`], measured against the run's
//!   own tip and the engine's processed height, with the engine held still
//!   until the host has written the page;
//! - **which store rows to mark spent** — [`TxoReconciler::classify_held`]:
//!   [`classify_outpoints`](crate::manager::accessors::classify_outpoints)'s
//!   verdicts, held the same way; a spend counts only once a ChainLock
//!   covers its block;
//! - **how long to retry a page a persistence round staled** —
//!   [`txo_reconcile_retry_stale_page`].
//!
//! The host keeps only what needs its store: paging its rows, resolving the
//! account that owns each, its transactions, and the fences that keep its
//! persistence callbacks and teardown out of a run's writes.
//!
//! # Lock order
//!
//! A run touches, in this order and never the other way round:
//!
//! 1. a host's handle registry (the C API's manager registry, a fair
//!    `parking_lot` lock), read side only and only transiently, to resolve
//!    the manager for one call — never while a run holds an engine guard;
//! 2. the wallet manager's `RwLock`, read side only, taken by
//!    [`TxoReconciler::heal_page_held`] or [`TxoReconciler::classify_held`]
//!    and kept, in a [`TxoReconcileHeldVerdicts`], until the host's apply
//!    has committed;
//! 3. the host's callback exclusion (Kotlin's `callbackExclusion`), taken
//!    for the apply;
//! 4. the host's store transaction (one Room transaction for one page).
//!
//! The engine calls the persistence callbacks, which take the callback
//! exclusion, while it holds the wallet manager's write lock — the same
//! order, 2 before 3. So a host must take the guard before the exclusion
//! (never wait for the engine while holding the exclusion), must never
//! take a held read inside a persistence callback, and must release the guard
//! after the exclusion. Every other C API path also takes the registry
//! before the engine lock (a blocking read inside a registry read, or a
//! manager create or destroy that writes the registry), so **no path that
//! holds the engine guard ever waits for the registry**: dropping a
//! [`TxoReconcileHeldVerdicts`] touches only the guard's own slot, and the
//! host drops a page's guard before it resolves the manager again for the
//! next page or for [`TxoReconciler::finish`]. A held read resolves the
//! manager with no guard held, then waits for its new guard outside the
//! registry. The schedule mutex and a guard's slot mutex are leaves,
//! taken in that order when both are: each is held only for a few map or
//! slot operations, never across an await or another lock. SPV progress is
//! read only by [`TxoReconciler::due_wallets`] and [`TxoReconciler::begin`],
//! before a run holds anything.
//!
//! tokio's `RwLock` is fair: a writer waiting behind the held guard (a fork
//! in `truncate_above`, a block being applied) queues every later reader
//! behind it, and `parking_lot`'s registry lock likewise parks new readers
//! behind a queued writer. That cannot close a cycle here. The writer holds
//! nothing the guard holder waits for — the guard holder waits only for the
//! callback exclusion and the store, and whatever holds the exclusion is
//! either a callback that already holds its engine lock (a read lock taken
//! before the writer queued; a write lock cannot be held beside the guard)
//! or the host's own code, which makes no engine call under it. A registry
//! reader parked on the engine behind that writer, and a registry writer
//! queued behind that reader, wait on the guard, never the other way
//! round. The one reader that could queue behind such a writer while
//! holding a guard is the same run asking for its next page, which is why
//! a host releases each page's guard first and both held reads drop one it
//! failed to release before they wait for a new one.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use key_wallet_manager::WalletManager;
use tokio::sync::{OwnedRwLockReadGuard, RwLock};

use crate::changeset::PlatformWalletPersistence;
use crate::manager::accessors::{
    classify_outpoints, wallet_utxos_page, OutpointClass, OutpointOwnershipQuery, WalletUtxoCursor,
    WalletUtxoRow,
};
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
/// `tip_height`. A host reads it through [`TxoReconciler::heal_page_held`],
/// which picks the height ([`txo_heal_reference_height`]) and holds the
/// engine still until the page is written.
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

/// The height one heal page's confirmations are counted to: the scan tip
/// the run's start gate measured, or the wallet's last processed height in
/// `wm` when that is lower. The tip is the run's, never the host's, so a
/// host cannot pass a height that makes a fresh coin look deep. The
/// processed height is read under the same guard as the page: a fork that
/// truncated the engine since the gate lowered it with the coins it
/// dropped, so a coin is not called deep against blocks the engine no
/// longer has. A wallet the engine does not hold counts to 0, which calls
/// every row immature.
pub fn txo_heal_reference_height(
    wm: &WalletManager<PlatformWalletInfo>,
    wallet_id: &WalletId,
    run_tip: u32,
) -> u32 {
    wm.get_wallet_info(wallet_id)
        .map_or(0, |info| info.core_wallet.metadata.last_processed_height)
        .min(run_tip)
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
    in_flight: BTreeMap<WalletId, ReconcileClaim>,
    last_run: BTreeMap<WalletId, Instant>,
    /// The id the next claim gets.
    next_claim: u64,
}

/// The read side of the wallet manager's lock, owned so it can outlive the
/// call that took it — see [`TxoReconciler::classify_held`].
type EngineReadGuard = OwnedRwLockReadGuard<WalletManager<PlatformWalletInfo>>;

/// Where one page's engine guard lives: shared by the run's claim, which
/// empties it on [`TxoReconcileSchedule::finish`] and at shutdown, and the
/// host's [`TxoReconcileHeldVerdicts`], which empties it on drop. Whichever
/// comes first releases the guard; the other finds the slot empty.
type HeldSlot = Arc<Mutex<Option<EngineReadGuard>>>;

/// Empty `slot`, releasing the guard outside its mutex.
fn release_slot(slot: &HeldSlot) {
    let guard = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
    drop(guard);
}

/// The engine guard one page's heal or spend verdicts were answered under
/// ([`TxoReconciler::heal_page_held`], [`TxoReconciler::classify_held`]).
/// Dropping it releases the guard; that
/// touches nothing but the guard's own slot — no schedule lookup, no
/// manager, no host registry — so the host can release it whatever else is
/// queued. Drop it once the page is applied.
pub struct TxoReconcileHeldVerdicts {
    slot: HeldSlot,
}

impl fmt::Debug for TxoReconcileHeldVerdicts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TxoReconcileHeldVerdicts")
            .finish_non_exhaustive()
    }
}

impl Drop for TxoReconcileHeldVerdicts {
    fn drop(&mut self) {
        release_slot(&self.slot);
    }
}

/// One run in flight.
#[derive(Default)]
struct ReconcileClaim {
    /// Tells this claim from a later one for the same wallet, so a
    /// classification that waited across a finish and a new begin does not
    /// hand its guard to the new run.
    id: u64,
    /// The scan tip the run's start gate measured, set when
    /// [`TxoReconciler::begin`] hands the claim over (0 until then). Every
    /// heal page of the run is measured against it
    /// ([`txo_heal_reference_height`]).
    tip: u32,
    /// The wallet was removed while the run was in flight
    /// ([`TxoReconcileSchedule::forget`]). The run still holds the wallet
    /// until it finishes, but its finish records nothing: the cadence it
    /// would restart belongs to a wallet that no longer exists.
    forgotten: bool,
    /// The slot of the guard the run's last classification is held under
    /// until its host has applied the verdicts. Emptied by
    /// [`TxoReconcileSchedule::finish`], so a run never outlives its guard.
    held: Option<HeldSlot>,
}

impl fmt::Debug for ReconcileClaim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReconcileClaim")
            .field("id", &self.id)
            .field("tip", &self.tip)
            .field("forgotten", &self.forgotten)
            .field("holds_engine", &self.held.is_some())
            .finish()
    }
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
            .filter(|wallet_id| !self.in_flight.contains_key(wallet_id))
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
        if self.in_flight.contains_key(&wallet_id) {
            return false;
        }
        self.next_claim += 1;
        let claim = ReconcileClaim {
            id: self.next_claim,
            ..ReconcileClaim::default()
        };
        self.in_flight.insert(wallet_id, claim);
        self.last_run.insert(wallet_id, now);
        true
    }

    /// Release the claim [`Self::try_begin`] took. `ran_at` is when a run
    /// that went ahead finished, which restarts the wallet's cadence from
    /// then; `None` for one that did not. A claim whose wallet was
    /// forgotten meanwhile records nothing. An engine guard the run still
    /// holds is released.
    pub fn finish(&mut self, wallet_id: &WalletId, ran_at: Option<Instant>) {
        let Some(claim) = self.in_flight.remove(wallet_id) else {
            return;
        };
        if let Some(slot) = &claim.held {
            release_slot(slot);
        }
        if let Some(at) = ran_at.filter(|_| !claim.forgotten) {
            self.last_run.insert(*wallet_id, at);
        }
    }

    /// Forget a removed wallet's cadence, so one re-added under the same id
    /// is due at once. A run still in flight keeps the wallet until it
    /// finishes, but cannot bring the cadence back.
    pub fn forget(&mut self, wallet_id: &WalletId) {
        self.last_run.remove(wallet_id);
        if let Some(claim) = self.in_flight.get_mut(wallet_id) {
            claim.forgotten = true;
        }
    }
}

/// Lock the schedule, recovering from poisoning: every method leaves it
/// consistent at each step, so a panicking holder cannot tear it.
fn lock_schedule(schedule: &Mutex<TxoReconcileSchedule>) -> MutexGuard<'_, TxoReconcileSchedule> {
    schedule.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A claim [`TxoReconciler::begin`] took and has not handed to its caller
/// yet. Dropped while still armed — the gate refused, or the `begin`
/// future was dropped mid-await — it releases the claim as a run that did
/// not go ahead, so no caller is left owing a [`TxoReconciler::finish`]
/// it was never told about. Both the hand-over and the release act on this
/// claim only, by its id: a stray finish that released it while the gate
/// waited may have let another run claim the wallet.
struct ProvisionalClaim<'a> {
    schedule: &'a Mutex<TxoReconcileSchedule>,
    wallet_id: WalletId,
    id: u64,
    armed: bool,
}

impl<'a> ProvisionalClaim<'a> {
    /// Claim `wallet_id`; `None` when a run is already in flight.
    fn take(
        schedule: &'a Mutex<TxoReconcileSchedule>,
        wallet_id: WalletId,
        now: Instant,
    ) -> Option<Self> {
        // Built only once the claim is taken: an armed value dropped on the
        // refusal would release the claim of the run already in flight.
        let mut locked = lock_schedule(schedule);
        if !locked.try_begin(wallet_id, now) {
            return None;
        }
        let id = locked.in_flight.get(&wallet_id)?.id;
        Some(Self {
            schedule,
            wallet_id,
            id,
            armed: true,
        })
    }

    /// The caller now holds the claim, measured against `tip`, and
    /// releases it with [`TxoReconciler::finish`].
    fn hand_over(mut self, tip: u32) {
        if let Some(claim) = lock_schedule(self.schedule)
            .in_flight
            .get_mut(&self.wallet_id)
            .filter(|claim| claim.id == self.id)
        {
            claim.tip = tip;
        }
        self.armed = false;
    }
}

impl Drop for ProvisionalClaim<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let mut schedule = lock_schedule(self.schedule);
        if schedule
            .in_flight
            .get(&self.wallet_id)
            .is_some_and(|claim| claim.id == self.id)
        {
            schedule.finish(&self.wallet_id, None);
        }
    }
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
        let steady = self.scan_progress().await.is_some_and(|p| p.is_steady());
        // Read after the steady-state read, never before: a restart between
        // the two then pairs the new session with the steady read and costs
        // at most one extra (idempotent) run, where the other order would
        // pair the old session with the new client's steady state and miss
        // its rising edge until the cadence came round.
        let session = self.spv.run_session();
        let wallets: Vec<WalletId> = self.wallets.load().keys().copied().collect();
        lock_schedule(&self.schedule).due_wallets(steady, session, wallets, Instant::now())
    }

    /// Claim `wallet_id` for one run and check that it may start, in order:
    /// no run already in flight, SPV in steady state with a scan tip, no
    /// latched sync fault, and the wallet's scan watermark within
    /// [`TXO_RECONCILE_TIP_MARGIN`] blocks of the tip. Returns that scan
    /// tip, which the claim keeps: the run's heal verdicts are measured
    /// against it ([`Self::heal_page_held`]), so the host only reports it.
    /// On `Ok` the claim holds until [`Self::finish`]; a refusal releases
    /// it, and so does dropping this future before it completes.
    pub async fn begin(&self, wallet_id: &WalletId) -> Result<u32, TxoReconcileSkip> {
        self.claim_across(wallet_id, self.gate(wallet_id)).await
    }

    /// Claim `wallet_id`, then await `gate` holding the claim provisionally:
    /// it passes to the caller only with `gate`'s `Ok`. `gate` is not
    /// polled before the claim is taken.
    async fn claim_across(
        &self,
        wallet_id: &WalletId,
        gate: impl Future<Output = Result<u32, TxoReconcileSkip>>,
    ) -> Result<u32, TxoReconcileSkip> {
        let claim = ProvisionalClaim::take(&self.schedule, *wallet_id, Instant::now())
            .ok_or(TxoReconcileSkip::AlreadyRunning)?;
        let tip = gate.await?;
        claim.hand_over(tip);
        Ok(tip)
    }

    /// [`classify_outpoints`] for the run `wallet_id` holds a claim for
    /// ([`Self::begin`]), answered under a read guard on the wallet manager
    /// that is kept when this returns: it comes back as a
    /// [`TxoReconcileHeldVerdicts`], which the host drops once it has
    /// written the verdicts to its store. [`Self::finish`] and the manager's
    /// shutdown release it too. Only [`OutpointClass::KnownUncredited`]
    /// marks a row spent, and only a spend in a block the wallet's ChainLock
    /// covers earns it.
    ///
    /// Two things make a verdict safe to write. The host's own fences see
    /// its persistence rounds, not the engine, and the engine can move with
    /// no round at all: dash-spv follows a fork through
    /// `WalletManager::truncate_above` under the write lock, which drops the
    /// spenders above the fork and restores the coins they spent, and the
    /// truncation reaches no persistence round (the wider gap of
    /// truncations no host persists is dashpay/platform#5380). Marking the
    /// row spent for a spender such a fork drops would retire a live coin
    /// for good: no later run looks at a spent row, and the restore stops
    /// handing it back. The write outlives the guard, so the spender must
    /// be one no fork can drop: `truncate_above` refuses to go below the
    /// wallet's ChainLock, and a verdict names only a spender at or below
    /// it. The guard keeps the engine still between the read and the
    /// write, so the store never records a verdict the engine has already
    /// moved past.
    ///
    /// The guard is held for one page's apply, so it must not wait on
    /// anything slow; see the module's lock order for what a host may do
    /// while holding it. A run holds one guard at a time: the host drops a
    /// page's before it asks about the next, and one it failed to drop is
    /// released here before this waits.
    ///
    /// `None` — nothing classified, nothing held — when the wallet holds no
    /// run claim, or its claim was released while this waited (a new claim
    /// taken meanwhile does not get this guard).
    pub async fn classify_held(
        &self,
        wallet_id: &WalletId,
        queries: &[OutpointOwnershipQuery],
    ) -> Option<(Vec<OutpointClass>, TxoReconcileHeldVerdicts)> {
        self.hold_engine(wallet_id, |wm, _| {
            classify_outpoints(wm, wallet_id, queries)
        })
        .await
    }

    /// [`txo_heal_page`] for the run `wallet_id` holds a claim for, held as
    /// [`Self::classify_held`] holds its verdicts: the host drops the
    /// [`TxoReconcileHeldVerdicts`] once it has written the page's eligible
    /// rows. A fork that truncates the engine between the read and the
    /// write would otherwise leave the store holding, unspent, a coin the
    /// engine no longer owns — the classify pass answers `Unknown` for it
    /// and leaves it, and the next launch restores it as spendable. The
    /// same lock order and one-guard-per-run rule apply.
    ///
    /// Confirmations are counted to [`txo_heal_reference_height`] for the
    /// claim's tip, read under the page's own guard, so each page is
    /// measured against the engine it was read from. The host passes no
    /// height.
    ///
    /// `None` — nothing read, nothing held — when the wallet holds no run
    /// claim, or its claim was released while this waited.
    pub async fn heal_page_held(
        &self,
        wallet_id: &WalletId,
        after: Option<&WalletUtxoCursor>,
        limit: usize,
    ) -> Option<(
        (Vec<(WalletUtxoRow, TxoHealVerdict)>, bool),
        TxoReconcileHeldVerdicts,
    )> {
        self.hold_engine(wallet_id, |wm, tip| {
            let height = txo_heal_reference_height(wm, wallet_id, tip);
            txo_heal_page(wm, wallet_id, after, limit, height)
        })
        .await
    }

    /// Answer `read` under a read guard on the wallet manager that is kept
    /// when this returns, attached to `wallet_id`'s run claim — the body of
    /// [`Self::classify_held`] and [`Self::heal_page_held`]. `read` gets the
    /// claim's tip. A guard the run still holds is released before this
    /// waits for the new one.
    async fn hold_engine<T>(
        &self,
        wallet_id: &WalletId,
        read: impl FnOnce(&WalletManager<PlatformWalletInfo>, u32) -> T,
    ) -> Option<(T, TxoReconcileHeldVerdicts)> {
        let (claim_id, tip) = {
            let mut schedule = lock_schedule(&self.schedule);
            let claim = schedule.in_flight.get_mut(wallet_id)?;
            if let Some(previous) = claim.held.take() {
                release_slot(&previous);
            }
            (claim.id, claim.tip)
        };
        let engine = Arc::clone(&self.wallet_manager).read_owned().await;
        let answer = read(&engine, tip);
        let held = TxoReconcileHeldVerdicts {
            slot: Arc::new(Mutex::new(Some(engine))),
        };
        let mut schedule = lock_schedule(&self.schedule);
        let claim = schedule
            .in_flight
            .get_mut(wallet_id)
            .filter(|claim| claim.id == claim_id)?;
        claim.held = Some(Arc::clone(&held.slot));
        Some((answer, held))
    }

    /// Claim `wallet_id` as a [`Self::begin`] whose gate measured `tip`
    /// would, with no gate: for tests that need a run without a steady SPV
    /// client.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn claim_without_gate(&self, wallet_id: &WalletId, tip: u32) -> bool {
        let Some(claim) = ProvisionalClaim::take(&self.schedule, *wallet_id, Instant::now()) else {
            return false;
        };
        claim.hand_over(tip);
        true
    }

    /// Release the claim [`Self::begin`] granted, and the engine guard it
    /// may still hold. `ran` is whether the run went ahead (even if it
    /// stopped early): that restarts the wallet's cadence from now.
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

    /// Drop a removed wallet's reconcile cadence. A run still applying
    /// keeps its engine guard: the removal waits for that one page.
    pub(super) fn forget_txo_reconcile(&self, wallet_id: &WalletId) {
        lock_schedule(&self.txo_reconcile_schedule).forget(wallet_id);
    }

    /// Release every engine guard a reconcile run still holds, so a host
    /// that failed to release one cannot hold teardown's writers off.
    pub(super) fn release_txo_reconcile_holds(&self) {
        let schedule = lock_schedule(&self.txo_reconcile_schedule);
        for slot in schedule
            .in_flight
            .values()
            .filter_map(|claim| claim.held.as_ref())
        {
            release_slot(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::pending;
    use std::pin::pin;
    use std::task::{Context, Waker};

    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Txid};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet_manager::WalletInterface;
    use tokio::time::timeout;

    use crate::events::PlatformEventManager;
    use crate::manager::accessors::classify_outpoints;
    use crate::manager::accessors::txo_inventory_tests::{
        bip44_account_0, collateral_burn, funding, in_block, manager_with, query,
    };

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

    #[test]
    fn a_run_finishing_after_its_wallet_was_forgotten_restores_no_cadence() {
        // The wallet is removed mid-run; the host stops the run and still
        // reports it as having gone ahead. The same wallet id, re-added
        // while SPV stays steady, must not wait out the old run's cadence.
        let mut schedule = TxoReconcileSchedule::default();
        let t0 = Instant::now();
        let second = Duration::from_secs(1);
        assert_eq!(schedule.due_wallets(true, 1, [A], t0), vec![A]);
        assert!(schedule.try_begin(A, t0));
        schedule.forget(&A);
        assert!(
            !schedule.try_begin(A, t0 + second),
            "the old run holds the wallet until it exits"
        );
        assert!(schedule.due_wallets(true, 1, [A], t0 + second).is_empty());
        schedule.finish(&A, Some(t0 + 2 * second));
        assert_eq!(
            schedule.due_wallets(true, 1, [A], t0 + 3 * second),
            vec![A],
            "the forgotten lifecycle's finish brought no history back"
        );
        assert!(schedule.try_begin(A, t0 + 3 * second));
        schedule.finish(&A, Some(t0 + 4 * second));
        assert!(
            schedule
                .due_wallets(true, 1, [A], t0 + 5 * second)
                .is_empty(),
            "the new lifecycle's own run restarts its cadence"
        );
    }

    fn reconciler() -> TxoReconciler {
        reconciler_over(WalletManager::<PlatformWalletInfo>::new(
            dashcore::Network::Testnet,
        ))
    }

    fn reconciler_over(wallet_manager: WalletManager<PlatformWalletInfo>) -> TxoReconciler {
        let wallet_manager = Arc::new(RwLock::new(wallet_manager));
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

    #[tokio::test]
    async fn a_begin_dropped_mid_gate_releases_its_claim() {
        let reconciler = reconciler();
        {
            let mut blocked = pin!(reconciler.claim_across(&A, pending()));
            let mut cx = Context::from_waker(Waker::noop());
            assert!(blocked.as_mut().poll(&mut cx).is_pending());
            assert!(
                !lock_schedule(&reconciler.schedule).try_begin(A, Instant::now()),
                "the gate is awaited holding the claim"
            );
        }
        assert!(
            lock_schedule(&reconciler.schedule).try_begin(A, Instant::now()),
            "dropping the blocked begin released it"
        );
    }

    #[tokio::test]
    async fn only_a_gate_that_passes_hands_the_claim_over() {
        let reconciler = reconciler();
        assert_eq!(
            reconciler
                .claim_across(&A, async { Err(TxoReconcileSkip::WalletBehindTip) })
                .await,
            Err(TxoReconcileSkip::WalletBehindTip)
        );
        assert_eq!(
            reconciler.claim_across(&A, async { Ok(TIP) }).await,
            Ok(TIP),
            "the refusal released its claim"
        );
        assert_eq!(
            reconciler.claim_across(&A, async { Ok(TIP) }).await,
            Err(TxoReconcileSkip::AlreadyRunning),
            "the granted claim outlives the begin that took it"
        );
        reconciler.finish(&A, true);
        assert_eq!(
            reconciler.claim_across(&A, async { Ok(TIP) }).await,
            Ok(TIP)
        );
    }

    /// The ChainLock the fork tests park on the wallet.
    const CHAIN_LOCKED: u32 = 100_010;

    /// A wallet with three coins, each spent by a mined record: one block
    /// below the ChainLock, at it, and one block above it. The ChainLock is
    /// parked on the wallet without the promotion pass, which in a default
    /// build (`keep-finalized-transactions` off) would drop the records it
    /// covers.
    async fn wallet_with_three_spent_coins() -> (
        WalletManager<PlatformWalletInfo>,
        WalletId,
        Vec<OutpointOwnershipQuery>,
    ) {
        let mut ctx = TestWalletContext::new_random();
        let script = ctx.receive_address.script_pubkey();
        let mut queries = Vec::new();
        for (seed, spender_height) in [
            (31u8, CHAIN_LOCKED - 1),
            (32, CHAIN_LOCKED),
            (33, CHAIN_LOCKED + 1),
        ] {
            let funded = funding(script.clone(), seed, 7_000);
            assert!(
                ctx.check_transaction(&funded, in_block(100_000))
                    .await
                    .is_relevant
            );
            let coin = OutPoint {
                txid: funded.txid(),
                vout: 0,
            };
            assert!(
                ctx.check_transaction(&collateral_burn(coin), in_block(spender_height))
                    .await
                    .is_relevant
            );
            queries.push(query(bip44_account_0(), coin, &script));
        }
        let (mut wm, wallet_id) = manager_with(ctx);
        wm.update_wallet_last_processed_height(&wallet_id, CHAIN_LOCKED + 10);
        wm.get_wallet_info_mut(&wallet_id)
            .expect("wallet")
            .core_wallet
            .metadata
            .last_applied_chain_lock = Some(ChainLock::dummy(CHAIN_LOCKED));
        (wm, wallet_id, queries)
    }

    /// A fork at the ChainLock, as dash-spv's `handle_fork` runs it: the
    /// write lock, then `truncate_above`.
    fn spawn_fork(reconciler: &TxoReconciler) -> tokio::task::JoinHandle<()> {
        let engine = Arc::clone(&reconciler.wallet_manager);
        tokio::spawn(async move {
            engine
                .write()
                .await
                .truncate_above(CHAIN_LOCKED)
                .expect("a fork at the ChainLock is followed");
        })
    }

    /// How long a test waits to see that a fork has NOT gone ahead. A fork
    /// that could go ahead would do so at once; one that cannot never
    /// does, however long this is.
    const BLOCKED: Duration = Duration::from_millis(50);
    /// How long a test waits for a fork it expects to finish.
    const PROCEEDS: Duration = Duration::from_secs(10);

    /// Classify under the run's claim, keeping the held verdicts.
    async fn classify(
        reconciler: &TxoReconciler,
        wallet_id: &WalletId,
        queries: &[OutpointOwnershipQuery],
    ) -> (Vec<OutpointClass>, TxoReconcileHeldVerdicts) {
        reconciler
            .classify_held(wallet_id, queries)
            .await
            .expect("the wallet holds a run claim")
    }

    fn holds_engine(reconciler: &TxoReconciler, wallet_id: &WalletId) -> bool {
        lock_schedule(&reconciler.schedule).in_flight[wallet_id]
            .held
            .as_ref()
            .is_some_and(|slot| slot.lock().expect("slot").is_some())
    }

    #[tokio::test]
    async fn a_classification_needs_the_runs_claim() {
        let reconciler = reconciler();
        assert!(reconciler.classify_held(&A, &[]).await.is_none());
        assert!(reconciler.claim_without_gate(&A, TIP));
        let (classes, held) = classify(&reconciler, &A, &[]).await;
        assert!(classes.is_empty());
        assert!(holds_engine(&reconciler, &A));
        drop(held);
        assert!(
            !holds_engine(&reconciler, &A),
            "dropping the verdicts released the guard"
        );
        reconciler.finish(&A, true);
    }

    /// A heal page is held like a classification: it needs the run's claim,
    /// and a fork waits until the host has written the page.
    #[tokio::test]
    async fn a_fork_waits_until_a_held_heal_page_is_released() {
        let (wm, wallet_id, _) = wallet_with_three_spent_coins().await;
        let reconciler = reconciler_over(wm);
        assert!(reconciler
            .heal_page_held(&wallet_id, None, 10)
            .await
            .is_none());
        assert!(reconciler.claim_without_gate(&wallet_id, TIP));

        let (_, held) = reconciler
            .heal_page_held(&wallet_id, None, 10)
            .await
            .expect("the wallet holds a run claim");
        assert!(holds_engine(&reconciler, &wallet_id));
        let mut fork = spawn_fork(&reconciler);
        assert!(
            timeout(BLOCKED, &mut fork).await.is_err(),
            "the fork waits while the heal page is held"
        );

        drop(held);
        timeout(PROCEEDS, fork)
            .await
            .expect("the fork proceeds once the page is released")
            .expect("fork task");
        reconciler.finish(&wallet_id, true);
    }

    /// A provisional claim that a stray finish released while its gate
    /// waited leaves the next run's claim alone, whether it is handed over
    /// or dropped.
    #[test]
    fn a_released_provisional_claim_leaves_the_next_claim_alone() {
        let schedule = Mutex::new(TxoReconcileSchedule::default());
        let claim_tip = || {
            lock_schedule(&schedule)
                .in_flight
                .get(&A)
                .map(|claim| claim.tip)
        };

        let stale = ProvisionalClaim::take(&schedule, A, Instant::now()).expect("claimed");
        lock_schedule(&schedule).finish(&A, None);
        assert!(lock_schedule(&schedule).try_begin(A, Instant::now()));
        stale.hand_over(TIP);
        assert_eq!(claim_tip(), Some(0), "the new claim keeps its own tip");

        lock_schedule(&schedule).finish(&A, None);
        let stale = ProvisionalClaim::take(&schedule, A, Instant::now()).expect("claimed");
        lock_schedule(&schedule).finish(&A, None);
        assert!(lock_schedule(&schedule).try_begin(A, Instant::now()));
        drop(stale);
        assert_eq!(claim_tip(), Some(0), "the new claim is still in flight");
    }

    /// A heal page counts confirmations to the run's own tip, capped by the
    /// engine's processed height under the page's guard: the host passes
    /// no height to inflate, and a fork between pages lowers the next
    /// page's.
    #[tokio::test]
    async fn a_heal_page_is_measured_against_the_runs_tip_and_the_engine() {
        const FUNDED: u32 = 100_000;
        let deep = FUNDED + TXO_RECONCILE_MIN_CONFIRMATIONS - 1;
        let mut ctx = TestWalletContext::new_random();
        let funded = funding(ctx.receive_address.script_pubkey(), 41, 9_000);
        assert!(
            ctx.check_transaction(&funded, in_block(FUNDED))
                .await
                .is_relevant
        );
        let (mut wm, wallet_id) = manager_with(ctx);
        wm.update_wallet_last_processed_height(&wallet_id, deep);
        let reconciler = reconciler_over(wm);
        type HeldHealPage = (
            (Vec<(WalletUtxoRow, TxoHealVerdict)>, bool),
            TxoReconcileHeldVerdicts,
        );
        let verdict = |page: Option<HeldHealPage>| {
            let ((rows, _), held) = page.expect("the wallet holds a run claim");
            drop(held);
            assert_eq!(rows.len(), 1);
            rows[0].1
        };

        assert!(reconciler.claim_without_gate(&wallet_id, deep - 1));
        assert_eq!(
            verdict(reconciler.heal_page_held(&wallet_id, None, 10).await),
            TxoHealVerdict::Immature,
            "the run's tip caps the depth, whatever the engine processed"
        );
        reconciler.finish(&wallet_id, true);

        assert!(reconciler.claim_without_gate(&wallet_id, TIP));
        assert_eq!(
            verdict(reconciler.heal_page_held(&wallet_id, None, 10).await),
            TxoHealVerdict::Eligible,
            "deep enough at the engine's processed height"
        );
        reconciler
            .wallet_manager
            .write()
            .await
            .truncate_above(FUNDED + 50)
            .expect("no ChainLock in the way");
        assert_eq!(
            verdict(reconciler.heal_page_held(&wallet_id, None, 10).await),
            TxoHealVerdict::Immature,
            "the fork lowered the height the next page is measured against"
        );
        reconciler.finish(&wallet_id, true);
    }

    /// The race the guard closes: verdicts read, then a fork drops a
    /// spender and restores its coin before the host writes them, with no
    /// persistence round for the host's fences to see. The fork now waits
    /// for the write.
    #[tokio::test]
    async fn a_fork_waits_until_the_held_verdicts_are_released() {
        use OutpointClass::{KnownUncredited, Unknown, Unspent};
        let (wm, wallet_id, queries) = wallet_with_three_spent_coins().await;
        let reconciler = reconciler_over(wm);
        assert!(reconciler.claim_without_gate(&wallet_id, TIP));

        let (classes, held) = classify(&reconciler, &wallet_id, &queries).await;
        assert_eq!(
            classes,
            vec![KnownUncredited, KnownUncredited, Unknown],
            "a spender above the ChainLock earns no verdict"
        );
        let mut fork = spawn_fork(&reconciler);
        assert!(
            timeout(BLOCKED, &mut fork).await.is_err(),
            "the fork waits while the verdicts are held"
        );

        drop(held);
        timeout(PROCEEDS, fork)
            .await
            .expect("the fork proceeds once they are released")
            .expect("fork task");
        let wm = reconciler.wallet_manager.read().await;
        assert_eq!(
            classify_outpoints(&wm, &wallet_id, &queries),
            vec![KnownUncredited, KnownUncredited, Unspent],
            "the fork restored the coin whose spender sat above the ChainLock"
        );
    }

    #[tokio::test]
    async fn finishing_the_run_releases_its_held_verdicts() {
        let (wm, wallet_id, queries) = wallet_with_three_spent_coins().await;
        let reconciler = reconciler_over(wm);
        assert!(reconciler.claim_without_gate(&wallet_id, TIP));
        let (_, held) = classify(&reconciler, &wallet_id, &queries).await;
        let mut fork = spawn_fork(&reconciler);
        assert!(timeout(BLOCKED, &mut fork).await.is_err());

        reconciler.finish(&wallet_id, true);
        timeout(PROCEEDS, fork)
            .await
            .expect("the fork proceeds once the run finished")
            .expect("fork task");
        drop(held);
    }

    /// The store keeps a verdict after the guard is gone, so a spend counts
    /// only once a ChainLock covers its block: no fork can drop it then. A
    /// wallet with no ChainLock yet has no final spends at all, and a spend
    /// above the ChainLock earns its verdict once the next one reaches it.
    #[tokio::test]
    async fn a_spend_counts_once_a_chain_lock_covers_its_block() {
        use OutpointClass::{KnownUncredited, Unknown};
        let (mut wm, wallet_id, queries) = wallet_with_three_spent_coins().await;
        let lock_at = |wm: &mut WalletManager<PlatformWalletInfo>, height: Option<u32>| {
            wm.get_wallet_info_mut(&wallet_id)
                .expect("wallet")
                .core_wallet
                .metadata
                .last_applied_chain_lock = height.map(ChainLock::dummy);
        };
        lock_at(&mut wm, None);
        assert_eq!(
            classify_outpoints(&wm, &wallet_id, &queries),
            vec![Unknown; 3]
        );
        lock_at(&mut wm, Some(CHAIN_LOCKED));
        assert_eq!(
            classify_outpoints(&wm, &wallet_id, &queries),
            vec![KnownUncredited, KnownUncredited, Unknown]
        );
        lock_at(&mut wm, Some(CHAIN_LOCKED + 1));
        assert_eq!(
            classify_outpoints(&wm, &wallet_id, &queries),
            vec![KnownUncredited; 3]
        );
    }

    /// tokio's `RwLock` queues a reader behind a waiting writer. A run that
    /// asked for its next page while still holding the last one would wait
    /// behind a fork that waits for it. A guard the host failed to release
    /// is released first, so the fork goes ahead and the next page sees its
    /// outcome; the stale verdicts' later drop is harmless.
    #[tokio::test]
    async fn the_next_page_does_not_deadlock_behind_a_waiting_fork() {
        use OutpointClass::{KnownUncredited, Unspent};
        let (wm, wallet_id, queries) = wallet_with_three_spent_coins().await;
        let reconciler = reconciler_over(wm);
        assert!(reconciler.claim_without_gate(&wallet_id, TIP));
        let (_, stale) = classify(&reconciler, &wallet_id, &queries).await;
        let mut fork = spawn_fork(&reconciler);
        assert!(timeout(BLOCKED, &mut fork).await.is_err());

        let (next, held) = timeout(PROCEEDS, classify(&reconciler, &wallet_id, &queries))
            .await
            .expect("the next page is classified, not deadlocked");
        assert_eq!(
            next,
            vec![KnownUncredited, KnownUncredited, Unspent],
            "the queued fork went first"
        );
        timeout(PROCEEDS, fork)
            .await
            .expect("fork finished")
            .expect("fork task");
        drop(stale);
        assert!(
            holds_engine(&reconciler, &wallet_id),
            "dropping the stale verdicts leaves the new page's guard alone"
        );
        drop(held);
        assert!(!holds_engine(&reconciler, &wallet_id));
    }

    /// A classification that waits for the engine across its run's finish
    /// and a new begin does not hand its guard to the new run.
    #[tokio::test]
    async fn a_classification_outliving_its_claim_holds_nothing() {
        let reconciler = reconciler();
        assert!(reconciler.claim_without_gate(&A, TIP));
        let writer = Arc::clone(&reconciler.wallet_manager).write_owned().await;
        let mut waiting = pin!(reconciler.classify_held(&A, &[]));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(waiting.as_mut().poll(&mut cx).is_pending());

        reconciler.finish(&A, true);
        assert!(
            reconciler.claim_without_gate(&A, TIP),
            "a new run for the wallet"
        );
        drop(writer);
        assert!(
            timeout(PROCEEDS, waiting)
                .await
                .expect("classified")
                .is_none(),
            "the old run's claim is gone"
        );
        assert!(!holds_engine(&reconciler, &A), "the new run holds no guard");
    }
}
