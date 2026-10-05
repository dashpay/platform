//! Automatically asking the network about sends whose broadcast outcome is
//! unknown.
//!
//! A send the SPV broadcaster could not confirm ends as `MaybeSent`
//! (`BroadcastResult::Uncertain`): the wallet keeps its record as an
//! unconfirmed spend and nothing ever tells the user whether the payment
//! landed. This module re-asks the network through
//! [`AcceptanceProbe`](crate::broadcast_probe::AcceptanceProbe) and publishes
//! the verdict; it **changes nothing** in the wallet. Verdicts are positive
//! only — accepted or mined — or unresolved: no answer the probe gets proves
//! that a transaction can never land (see
//! [`broadcast_probe`](crate::broadcast_probe)).
//!
//! What is probed: the *roots* of the chains the wallet signed into — its
//! unsettled sends (not in a block, not InstantSend-locked) plus the unsettled
//! ancestors they depend on, including someone else's incoming payment —
//! taking only transactions that spend no output of an unsettled transaction.
//! Probing a child is pointless and misleading: a node that has not seen the
//! parent answers `missingorspent` for a perfectly valid child. A healthy send
//! is IS-locked within seconds, so in practice only sends that went quiet are
//! ever probed.
//!
//! What is published: verdicts per **own send**, which is what the host
//! shows. A root's verdict goes to the root when it is the wallet's send.
//! Every verdict is advisory, Mined included: only the wallet itself (a
//! block, an InstantSend lock) settles a send, so a root found mined stays a
//! root — probed on its schedule like any other — until the wallet settles
//! it, and a later probe may turn it back into Accepted. Only changes are
//! published: Unresolved never replaces a decided verdict (Accepted, Mined),
//! so a Mined a reorg dropped stays published until a probe disagrees or the
//! wallet settles the send. The host is told to drop a send's verdict
//! (cleared) when the send settles or leaves the wallet, when its wallet is
//! removed and, for every send, when probing is turned off; cleared says
//! only "forget the verdict", not that the send settled. A
//! wallet event that reports a send settled (in a block or InstantSend-
//! locked) or swept clears its verdict at once, even mid-scan; a pass
//! clears whatever else left the wallet when it next reads it.
//!
//! When: once as soon as dash-spv reports an `Uncertain` broadcast — the
//! current root of that transaction's chain; a send built on it becomes due
//! by schedule once the root settles, not at once — then on each advance of
//! the wallet's synced height that follows the tip. Steps of more than
//! [`CATCH_UP_STEP`] blocks, steps back (a rescan) and the first step after
//! launch are catch-up and skipped (passes queued before them probe only
//! what an `Uncertain` result forced). Each dash-spv sync completion (after
//! every block, and when a launch's or a reconnect's catch-up ends) queues
//! a followed pass for every wallet within [`CATCH_UP_STEP`] of its tip —
//! unless one already read the wallet at that height — so the pass a
//! catch-up skipped runs at the tip without waiting for the next block (a
//! completion while probing is off is ignored: the next one covers it). A
//! root is probed every block for [`EVERY_BLOCK_WINDOW`] blocks, counted
//! from its first probe by a height-driven pass while the wallet follows the
//! tip (a forced pass does not start it), and every [`SLOW_INTERVAL`] blocks
//! after. A probe no node answered (DAPI unreachable — its addresses
//! still banned just after an outage) does not count: its root is due again
//! at the next block, whatever its phase, and at the same height a retry
//! probes it again (as does any `Uncertain` report, which forces a probe) — after [`NO_ANSWER_RETRY`] if no block comes
//! first, then after twice and four times that again (at most
//! [`MAX_NO_ANSWER_RETRIES`] per height). Probes never ban a DAPI address
//! themselves. Each Uncertain report forces a probe of its chain's current
//! root in the next pass (reports queued together share it); a pass under
//! way gives way to it after its probe in flight. A pass probes at most
//! [`MAX_ROOTS_PER_PASS`] scheduled roots, those probed longest ago first —
//! a wallet's records can be fed by a peer, so one pass's network work is
//! bounded and the rest come round at the next heights. The roots of sends
//! reported `Uncertain` and not settled yet go first, with at most half of
//! the slots while other roots wait; a no-answer retry also makes a pass
//! under way give way.
//! Between passes a wallet waits for the height at which its next root is due;
//! one whose last pass had nothing left to probe is idle and not scanned
//! again until a wallet event that touched its records (a new or
//! InstantSend-locked transaction, a block that changed records, a sweep) or
//! an `Uncertain` result for one of its transactions wakes it. No verdict
//! pauses the probing of a root: neither a mempool nor the nodes' word on a
//! block is settlement.
//! Probes run only while the SPV client delivers blocks, i.e. while the app is
//! in the foreground.
//!
//! How: one task (the actor) owns all of this state and handles commands —
//! from wallet and SPV events, the switch and wallet removal — and the
//! results of its reads and probes one at a time, in order, so none of them
//! race. At most one pass per wallet runs; requests that arrive meanwhile
//! are merged into the next. Every host callback comes from that task, never
//! from a caller that may hold its own locks.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::mem;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

use async_trait::async_trait;
use dash_async::WorkerStatus;
use dash_spv::sync::SyncEvent;
use dash_spv::{BroadcastResult, EventHandler};
use dashcore::{Transaction, Txid};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::Wallet;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex, RwLock};
use tokio::task::{self, AbortHandle, JoinError, JoinHandle, JoinSet};
use tokio::time::{timeout_at, Instant};

use crate::broadcast_probe::{AcceptanceProbe, ProbeOutcome, ProbeVerdict};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::wallet::core::record_spends_own_coins;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Blocks from the start of a root's window (see [`ProbeSchedule`]) during
/// which it is probed on every block.
pub(crate) const EVERY_BLOCK_WINDOW: u32 = 24;

/// Blocks between probes once [`EVERY_BLOCK_WINDOW`] has passed.
pub(crate) const SLOW_INTERVAL: u32 = 10;

/// Scheduled roots a pass probes at most; forced ones (an `Uncertain`
/// report's, a no-answer retry's) come on top. A wallet's records are not
/// all its own doing — a peer can feed it transactions that make many roots —
/// so the work of one pass is bounded, and within it the roots probed
/// longest ago (or never) go first: the rest stay due and come round at the
/// next heights. The roots of sends reported `Uncertain` and not settled yet
/// go first, but take at most half of the slots while other roots wait.
pub(crate) const MAX_ROOTS_PER_PASS: usize = 8;

/// How long after a probe that no node answered (all of DAPI unreachable —
/// the network just coming back, its addresses still banned for at least a
/// minute) the wallet's roots are probed again instead of waiting for the
/// next block; doubled for each further retry at the same height, up to
/// [`MAX_NO_ANSWER_RETRIES`].
pub(crate) const NO_ANSWER_RETRY: Duration = Duration::from_secs(60);

/// Retries per wallet and height after probes no node answered: 1, 2 and 4
/// times [`NO_ANSWER_RETRY`] — then the next block decides.
pub(crate) const MAX_NO_ANSWER_RETRIES: u32 = 3;

/// Followed blocks an `Uncertain` send waits for its peer echo: as long as
/// the every-block probe window. Past that the probes decide alone.
pub(crate) const UNCERTAIN_EXPIRY: u32 = EVERY_BLOCK_WINDOW;

/// How far a wallet's synced height may advance in one step and still count
/// as following the tip. Larger steps, and steps back (a rescan after a
/// rewind), are catch-up: the scan itself settles most roots, so a
/// height-driven pass waits for it.
pub(crate) const CATCH_UP_STEP: u32 = 3;

/// One unsettled transaction of the wallet, merged across the accounts that
/// record it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutgoingView {
    pub txid: Txid,
    /// The transaction spends at least one of this wallet's coins, i.e. the
    /// wallet signed it.
    pub spends_own_coins: bool,
    /// Shared with the passes and probe jobs that send it; never changed.
    pub transaction: Arc<Transaction>,
}

impl OutgoingView {
    fn spends_output_of(&self, txids: &impl Fn(&Txid) -> bool) -> bool {
        self.transaction
            .input
            .iter()
            .any(|input| txids(&input.previous_output.txid))
    }
}

/// One account's record of a transaction, as far as the resolver cares.
pub(crate) struct RecordFacts<'a> {
    pub txid: Txid,
    /// In a block, InstantSend-locked, or finalized in this account.
    pub settled: bool,
    /// This account records the transaction spending the wallet's coins.
    pub own: bool,
    pub transaction: &'a Transaction,
}

/// The one place the multi-account rule lives: a transaction is one
/// transaction however many accounts record it; it is settled if any account
/// holds it settled, and the wallet's own send if any account records it
/// spending the wallet's coins (a transfer between accounts is "incoming" in
/// the receiving one). Returns the unsettled ones.
pub(crate) fn merge_records<'a>(
    records: impl IntoIterator<Item = RecordFacts<'a>>,
) -> Vec<OutgoingView> {
    let mut settled: HashSet<Txid> = HashSet::new();
    let mut unsettled: HashMap<Txid, (bool, &Transaction)> = HashMap::new();
    for record in records {
        if record.settled {
            settled.insert(record.txid);
            continue;
        }
        unsettled
            .entry(record.txid)
            .and_modify(|(own, _)| *own |= record.own)
            .or_insert((record.own, record.transaction));
    }
    unsettled
        .into_iter()
        .filter(|(txid, _)| !settled.contains(txid))
        .map(|(txid, (own, transaction))| OutgoingView {
            txid,
            spends_own_coins: own,
            transaction: Arc::new(transaction.clone()),
        })
        .collect()
}

/// This wallet's unsettled transactions. One walk over the records; only
/// unsettled ones are cloned — on a healthy wallet there are none or a
/// handful.
///
/// `signs`: the wallet can sign ([`Wallet::can_sign`]); a watch-only wallet sends
/// nothing, so none of its transactions is an own send.
pub(crate) fn collect_views(info: &PlatformWalletInfo, signs: bool) -> Vec<OutgoingView> {
    let accounts = info.core_wallet.accounts.all_accounts();
    merge_records(accounts.iter().flat_map(|account| {
        account.transactions().values().map(move |record| {
            record_facts(
                record,
                || account.transaction_is_finalized(&record.txid),
                signs,
            )
        })
    }))
}

/// One account's record as the resolver sees it; `finalized`: whether the
/// account holds the transaction finalized, asked only of a record not
/// already settled; `signs`: the wallet can sign ([`Wallet::can_sign`]).
fn record_facts(
    record: &TransactionRecord,
    finalized: impl FnOnce() -> bool,
    signs: bool,
) -> RecordFacts<'_> {
    RecordFacts {
        txid: record.txid,
        settled: is_settled(record) || finalized(),
        // The wallet signed it: it spends coins the wallet owns
        // ([`record_spends_own_coins`]; a contact's watch-only chain records
        // the contact's spends). Other records still take part in the chain
        // walk as ancestors of a send that is ours.
        own: signs && record_spends_own_coins(record),
        transaction: &record.transaction,
    }
}

/// Whether the accounts' records of one transaction make it an unsettled own
/// send — the same rule as [`merge_records`]: settled in any account means
/// settled, and own in any account means own.
fn is_unsettled_own<'a>(records: impl IntoIterator<Item = RecordFacts<'a>>) -> bool {
    // No allocation: this runs per holding wallet on every holder lookup.
    records
        .into_iter()
        .try_fold(false, |own, record| {
            (!record.settled).then_some(own || record.own)
        })
        .unwrap_or(false)
}

/// This wallet's unsettled transactions as chains. `views` hold one entry per
/// transaction (see [`merge_records`]).
pub(crate) struct ChainGraph<'a> {
    by_txid: HashMap<Txid, &'a OutgoingView>,
    /// Transactions an ancestry traversal expanded, for the tests that pin
    /// its cost.
    #[cfg(test)]
    expanded: std::cell::Cell<usize>,
}

impl<'a> ChainGraph<'a> {
    /// Only the wallet settles a transaction: a probe's verdict, Mined
    /// included, changes nothing here.
    pub(crate) fn new(views: &'a [OutgoingView]) -> Self {
        Self {
            by_txid: views.iter().map(|view| (view.txid, view)).collect(),
            #[cfg(test)]
            expanded: std::cell::Cell::new(0),
        }
    }

    fn is_own(&self, txid: &Txid) -> bool {
        self.by_txid
            .get(txid)
            .is_some_and(|view| view.spends_own_coins)
    }

    fn is_unsettled(&self, txid: &Txid) -> bool {
        self.by_txid.contains_key(txid)
    }

    /// The wallet's unsettled sends.
    pub(crate) fn own_unsettled(&self) -> BTreeSet<Txid> {
        self.by_txid
            .keys()
            .filter(|txid| self.is_own(txid))
            .copied()
            .collect()
    }

    /// The unsettled transactions any of `sources` depends on, transitively,
    /// sources included — one traversal with one visited set, so each
    /// transaction is expanded once however many sources share its ancestry
    /// (a long chain of sends costs its length, not its square).
    fn with_ancestors(&self, sources: impl IntoIterator<Item = Txid>) -> BTreeSet<Txid> {
        let mut found = BTreeSet::new();
        let mut frontier: Vec<Txid> = sources.into_iter().collect();
        while let Some(txid) = frontier.pop() {
            if !self.is_unsettled(&txid) || !found.insert(txid) {
                continue;
            }
            #[cfg(test)]
            self.expanded.set(self.expanded.get() + 1);
            frontier.extend(
                self.by_txid[&txid]
                    .transaction
                    .input
                    .iter()
                    .map(|input| input.previous_output.txid),
            );
        }
        found
    }

    fn is_root(&self, txid: &Txid) -> bool {
        self.is_unsettled(txid)
            && !self.by_txid[txid].spends_output_of(&|parent| self.is_unsettled(parent))
    }

    /// The roots to probe: members of the chains the wallet signed into — its
    /// unsettled sends plus every unsettled ancestor they depend on, including
    /// someone else's incoming payment — that spend no output of an unsettled
    /// transaction. A node that has not seen a parent refuses a valid child.
    pub(crate) fn roots(&self) -> Vec<&'a OutgoingView> {
        self.with_ancestors(self.own_unsettled())
            .iter()
            .filter(|txid| self.is_root(txid))
            .map(|txid| self.by_txid[txid])
            .collect()
    }

    /// The roots of the chains of `txids` — what to probe when dash-spv
    /// reports them uncertain but they spend an unsettled parent. One
    /// traversal for all of them.
    pub(crate) fn roots_of(&self, txids: impl IntoIterator<Item = Txid>) -> BTreeSet<Txid> {
        self.with_ancestors(txids)
            .into_iter()
            .filter(|txid| self.is_root(txid))
            .collect()
    }
}

/// The roots of `views` — see [`ChainGraph::roots`].
#[cfg(test)]
fn ambiguous_roots(views: &[OutgoingView]) -> Vec<&OutgoingView> {
    ChainGraph::new(views).roots()
}

/// When a root is next due for a probe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProbeSchedule {
    /// Where the every-block window starts: the root's first probe by a
    /// height-driven pass while the wallet followed the tip. `None` until
    /// then — waiting in a backlog, or probed only by forced passes, whose
    /// height may be a stale stored tip; `None` counting as in-window matters
    /// for `next_due`.
    window_start: Option<u32>,
    /// The first height a pass that followed the tip saw the root at: its
    /// place in the queue until its first probe (see `MAX_ROOTS_PER_PASS`).
    seen_at: Option<u32>,
    last_probe: Option<u32>,
    /// The last probe reached no node: due again at the next height, in any
    /// phase — but not at the same height, where only a retry
    /// ([`NO_ANSWER_RETRY`]) probes it again.
    unanswered: bool,
}

impl ProbeSchedule {
    /// Start the window here if it has not started — once.
    fn anchor(&mut self, height: u32) {
        self.window_start.get_or_insert(height);
    }

    fn in_window(&self, height: u32) -> bool {
        self.window_start
            .is_none_or(|first| height.saturating_sub(first) < EVERY_BLOCK_WINDOW)
    }

    pub(crate) fn is_due(&self, height: u32) -> bool {
        match self.last_probe {
            None => true,
            Some(last) if height <= last => false,
            Some(_) if self.unanswered || self.in_window(height) => true,
            Some(last) => height - last >= SLOW_INTERVAL,
        }
    }

    /// Never moves back: a probe marked at a stale height (a pass that ran
    /// across a catch-up) must not make the root due again below its last.
    fn probed_at(&mut self, height: u32) {
        self.last_probe = Some(self.last_probe.map_or(height, |last| last.max(height)));
        // A new probe is out: its own answer decides the mark.
        self.unanswered = false;
    }

    /// The last probe produced nothing: due again at once.
    fn withdraw(&mut self) {
        self.last_probe = None;
    }

    /// The lowest height at which [`is_due`](Self::is_due) holds.
    pub(crate) fn next_due(&self) -> u32 {
        match self.last_probe {
            None => 0,
            Some(last) if self.unanswered || self.in_window(last.saturating_add(1)) => {
                last.saturating_add(1)
            }
            Some(last) => last.saturating_add(SLOW_INTERVAL),
        }
    }
}

/// Whether two verdicts say the same thing. Every Unresolved counts as one,
/// since its reason varies from probe to probe. Accepted and Mined differ —
/// though the FFI host receives both as the same code (see
/// `OUTGOING_PROBE_VERDICT_ACCEPTED`).
fn same_verdict(a: &ProbeVerdict, b: &ProbeVerdict) -> bool {
    match (a, b) {
        (ProbeVerdict::Unresolved { .. }, ProbeVerdict::Unresolved { .. }) => true,
        _ => a == b,
    }
}

fn is_decided(verdict: &ProbeVerdict) -> bool {
    !matches!(verdict, ProbeVerdict::Unresolved { .. })
}

/// What the host must hear, per own send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolverEvent {
    /// A send's verdict changed.
    Verdict(Txid, ProbeVerdict),
    /// The host must drop the verdict it holds for a send: the send settled
    /// or left the wallet, its wallet was removed, or probing was turned off.
    /// Not a statement that the send settled.
    Cleared(Txid),
}

/// One event for the host, with what the log line needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outgoing {
    wallet_id: WalletId,
    event: ResolverEvent,
    height: Option<u32>,
    trigger: &'static str,
}

/// What started a pass: dash-spv's `Uncertain` result for one of the
/// wallet's transactions, or an advance of its synced height.
fn trigger_of(forced: bool) -> &'static str {
    if !forced {
        "height"
    } else {
        "uncertain"
    }
}

/// Bookkeeping for every wallet. Schedules are per probed root; published
/// verdicts are per own send, which is what the host shows.
/// Everything for a transaction is kept while it stays unsettled in the
/// wallet, and dropped once it settles or leaves. Owned by the resolver's
/// actor, so every change happens in one sequence.
#[derive(Debug, Default)]
pub(crate) struct ResolverState {
    schedules: HashMap<(WalletId, Txid), ProbeSchedule>,
    /// What the host holds per send.
    published: HashMap<(WalletId, Txid), ProbeVerdict>,
}

impl ResolverState {
    /// Drop everything kept for `wallet_id`; a clear for every send the host
    /// had a verdict for.
    fn forget_wallet(&mut self, wallet_id: &WalletId, trigger: &'static str) -> Vec<Outgoing> {
        self.schedules.retain(|(wallet, _), _| wallet != wallet_id);
        self.clear_where(|wallet, _| wallet == wallet_id, None, trigger)
    }

    /// Drop everything; a clear for every published send.
    fn forget_all(&mut self) -> Vec<Outgoing> {
        self.schedules.clear();
        self.clear_where(|_, _| true, None, "disabled")
    }

    /// A wallet event reported `txids` settled or gone: forget them and clear
    /// their verdicts.
    fn settle(&mut self, wallet_id: WalletId, txids: &HashSet<Txid>) -> Vec<Outgoing> {
        for txid in txids {
            self.schedules.remove(&(wallet_id, *txid));
        }
        self.clear_where(
            |wallet, txid| *wallet == wallet_id && txids.contains(txid),
            None,
            "settled-or-left",
        )
    }

    /// Unpublish every send `drop` matches; a clear for each, in a stable order.
    fn clear_where(
        &mut self,
        drop: impl Fn(&WalletId, &Txid) -> bool,
        height: Option<u32>,
        trigger: &'static str,
    ) -> Vec<Outgoing> {
        let mut cleared: Vec<(WalletId, Txid)> = self
            .published
            .keys()
            .filter(|(wallet, txid)| drop(wallet, txid))
            .copied()
            .collect();
        cleared.sort();
        self.published
            .retain(|(wallet, txid), _| !drop(wallet, txid));
        cleared
            .into_iter()
            .map(|(wallet_id, txid)| Outgoing {
                wallet_id,
                event: ResolverEvent::Cleared(txid),
                height,
                trigger,
            })
            .collect()
    }

    /// Record `verdict` for `send`; returns whether the host must hear it.
    /// Unresolved never replaces a decided verdict (Accepted, Mined) — one
    /// probe that only met transport errors says nothing new.
    fn publish(&mut self, key: (WalletId, Txid), verdict: &ProbeVerdict) -> bool {
        let changed = match self.published.get(&key) {
            None => true,
            Some(previous) if is_decided(previous) && !is_decided(verdict) => false,
            Some(previous) => !same_verdict(previous, verdict),
        };
        if changed {
            self.published.insert(key, verdict.clone());
        }
        changed
    }

    /// Publish `verdict` for each of `sends`, collecting what changed.
    fn publish_all(
        &mut self,
        wallet_id: WalletId,
        sends: impl IntoIterator<Item = Txid>,
        verdict: &ProbeVerdict,
        height: u32,
        trigger: &'static str,
        out: &mut Vec<Outgoing>,
    ) {
        for send in sends {
            if self.publish((wallet_id, send), verdict) {
                out.push(Outgoing {
                    wallet_id,
                    event: ResolverEvent::Verdict(send, verdict.clone()),
                    height: Some(height),
                    trigger,
                });
            }
        }
    }
}

/// The first advance after launch has no previous height to compare with, and
/// it is usually the stored tip from before the scan: treat it as catch-up
/// too. So is a step back — a rescan after a rewind (a new account) replays
/// old heights. Probing starts from the next step that follows the tip.
pub(crate) fn is_catch_up_step(previous: Option<u32>, height: u32) -> bool {
    previous.is_none_or(|previous| height < previous || height - previous > CATCH_UP_STEP)
}

/// A wallet at `height` is at a sync's completed `tip`, as far as following
/// goes.
fn near_tip(height: u32, tip: u32) -> bool {
    height.abs_diff(tip) <= CATCH_UP_STEP
}

/// A forced request (a report's, or a no-answer retry's) waits for the
/// wallet's run: the run gives way, keeping
/// only its own forced roots — the scheduled ones it has not sent stay due
/// and come round at the next pass — so the forced pass starts after the
/// probe in flight, not after a queue of scheduled roots.
fn yield_to_forced(entry: &mut WalletEntry) {
    let Some(run) = entry.run.as_mut() else {
        return;
    };
    // Only for a request the run does not already carry (a repeated report
    // of a send it forces itself changes nothing).
    let new = entry.pending.as_ref().is_some_and(|pending| {
        pending
            .forced
            .iter()
            .chain(&pending.retried)
            .any(|txid| !run.forced.contains(txid) && !run.retried.contains(txid))
    });
    if !new {
        return;
    }
    let anchor = run.anchor_at;
    if let Some(probing) = run.probing.as_mut() {
        probing.due.retain(|root| root.forced);
    }
    // The forced pass that follows takes the dropped roots after its own:
    // it follows the tip at the same height.
    if let (Some(anchor), Some(pending)) = (anchor, entry.pending.as_mut()) {
        pending.anchor_at = pending.anchor_at.max(Some(anchor));
        pending.height = pending.height.max(anchor);
    }
}

/// A root chosen for a probe.
#[derive(Debug, Clone)]
pub(crate) struct DueRoot {
    pub txid: Txid,
    /// Shared with the probe job: sent as is, never changed.
    pub transaction: Arc<Transaction>,
    /// The root is the wallet's own send.
    pub own: bool,
    /// An `Uncertain` result forced it (it may also be due by its schedule).
    /// Labels its verdict in the log, and is the only kind of root a pass
    /// without an anchor probes.
    pub forced: bool,
    /// A no-answer retry repeats it (see [`NO_ANSWER_RETRY`]): labels its
    /// verdict.
    pub retry: bool,
}

impl DueRoot {
    /// What made this probe happen, for the log.
    fn trigger(&self) -> &'static str {
        if self.retry {
            "no-answer-retry"
        } else {
            trigger_of(self.forced)
        }
    }
}

/// The start of a pass: what the host must hear now, and the roots to probe.
#[derive(Debug)]
pub(crate) struct PassStart {
    pub events: Vec<Outgoing>,
    pub due: VecDeque<DueRoot>,
}

/// Start a pass over one wallet at `height`: forget what settled or left
/// (clearing the sends the host had a verdict for) and pick the roots due for
/// a probe. The
/// roots of every `forced` transaction's chain (dash-spv has just reported
/// it `Uncertain`) are due regardless of their schedule — even if one went
/// out at this height already: a repeat probe is cheaper than bookkeeping
/// that tells the two apart.
/// `anchor`: the tip height
/// a height-driven pass knows the wallet follows — a root it takes starts
/// its every-block window when its probe goes out (see [`mark_sent`]).
/// `priority`: the wallet's sends reported `Uncertain` and not settled yet —
/// their roots go first among the scheduled ones (see
/// [`MAX_ROOTS_PER_PASS`]).
pub(crate) fn begin_pass(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
    priority: &HashSet<Txid>,
    anchor: Option<u32>,
) -> PassStart {
    let mut events = Vec::new();
    // What the wallet itself still holds unsettled — what state is kept for.
    let present: HashSet<Txid> = views.iter().map(|view| view.txid).collect();
    let own_present: HashSet<Txid> = views
        .iter()
        .filter(|view| view.spends_own_coins)
        .map(|view| view.txid)
        .collect();
    let is_gone = |wallet: &WalletId, txid: &Txid| *wallet == wallet_id && !present.contains(txid);
    state
        .schedules
        .retain(|(wallet, txid), _| !is_gone(wallet, txid));
    // Cleared because the send is no longer an unsettled own send: it
    // settled, or left the wallet — not a statement that it settled.
    events.extend(state.clear_where(
        |wallet, txid| *wallet == wallet_id && !own_present.contains(txid),
        Some(height),
        "settled-or-left",
    ));

    let graph = ChainGraph::new(views);
    let roots = graph.roots();
    let forced_roots = graph.roots_of(forced.iter().copied());

    // Forced roots first, then at most `MAX_ROOTS_PER_PASS` scheduled ones
    // in turn: those probed longest ago first, a never-probed one by the
    // height it was first seen at — a queue, so fresh roots (a peer can feed
    // them at will) wait behind older ones and cannot starve them. Reported
    // sends' roots go first, in the same order among themselves, but leave
    // at least half of the slots to the rest: a peer whose outputs a reported
    // send spends makes its parents such roots too.
    let priority = graph.roots_of(priority.iter().copied());
    let due_root = |view: &OutgoingView, forced| DueRoot {
        txid: view.txid,
        transaction: Arc::clone(&view.transaction),
        own: graph.is_own(&view.txid),
        forced,
        retry: false,
    };
    let mut due = VecDeque::new();
    type Turn = (u32, bool, Txid);
    let mut first: Vec<(Turn, &OutgoingView)> = Vec::new();
    let mut rest: Vec<(Turn, &OutgoingView)> = Vec::new();
    for view in &roots {
        let schedule = state.schedules.entry((wallet_id, view.txid)).or_default();
        if let Some(anchor) = anchor {
            schedule.seen_at.get_or_insert(anchor);
        }
        if forced_roots.contains(&view.txid) {
            due.push_back(due_root(view, true));
        } else if anchor.is_some() && schedule.is_due(height) {
            // Without an anchor the pass is confined to forced roots.
            // At the same height, one still waiting for its first probe goes
            // before one probed there.
            let turn = (
                schedule.last_probe.or(schedule.seen_at).unwrap_or(0),
                schedule.last_probe.is_some(),
                view.txid,
            );
            if priority.contains(&view.txid) {
                first.push((turn, view));
            } else {
                rest.push((turn, view));
            }
        }
    }
    let first_slots = first
        .len()
        .min((MAX_ROOTS_PER_PASS / 2).max(MAX_ROOTS_PER_PASS.saturating_sub(rest.len())));
    let rest_slots = rest.len().min(MAX_ROOTS_PER_PASS - first_slots);
    let mut scheduled = take_in_turn(first, first_slots);
    scheduled.extend(take_in_turn(rest, rest_slots));
    due.extend(scheduled.into_iter().map(|(_, view)| due_root(view, false)));
    PassStart { events, due }
}

/// The first `count` of `queue` by turn, in turn.
fn take_in_turn<K: Ord + Copy, V>(mut queue: Vec<(K, V)>, count: usize) -> Vec<(K, V)> {
    if queue.len() > count {
        if count == 0 {
            return Vec::new();
        }
        queue.select_nth_unstable_by_key(count, |(turn, _)| *turn);
        queue.truncate(count);
    }
    queue.sort_unstable_by_key(|(turn, _)| *turn);
    queue
}

/// A root is being sent to the network at `height`: its schedule counts from
/// here. Marked when the probe goes out, not when the pass is planned, so a
/// root the pass never reached stays due.
pub(crate) fn mark_sent(
    state: &mut ResolverState,
    wallet_id: WalletId,
    root: Txid,
    height: u32,
    anchor: Option<u32>,
) {
    if let Some(schedule) = state.schedules.get_mut(&(wallet_id, root)) {
        // A root's every-block window starts at its first probe by a pass
        // that follows the tip — not while it waits in a backlog, nor when a
        // pass that planned it gave way before sending it.
        if let Some(anchor) = anchor {
            schedule.anchor(anchor);
        }
        schedule.probed_at(height);
    }
}

/// Whether a root's last probe reached a node (see
/// [`ProbeSchedule::unanswered`]).
fn mark_answered(state: &mut ResolverState, wallet_id: WalletId, root: Txid, answered: bool) {
    if let Some(schedule) = state.schedules.get_mut(&(wallet_id, root)) {
        schedule.unanswered = !answered;
    }
}

/// The roots of `wallet_id` no node answered at `height`, made due again —
/// a retry's, the only probe again at that height. A root whose probe is out
/// was marked sent, which cleared its mark.
fn withdraw_unanswered(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
) -> HashSet<Txid> {
    let mut roots = HashSet::new();
    for ((wallet, txid), schedule) in state.schedules.iter_mut() {
        if *wallet == wallet_id && schedule.unanswered && schedule.last_probe == Some(height) {
            schedule.withdraw();
            roots.insert(*txid);
        }
    }
    roots
}

/// A root's probe never produced a verdict (its job panicked): nothing was
/// learned, so it is due again at once.
pub(crate) fn withdraw_send(state: &mut ResolverState, wallet_id: WalletId, root: Txid) {
    if let Some(schedule) = state.schedules.get_mut(&(wallet_id, root)) {
        schedule.withdraw();
    }
}

/// Record one root's probe result: the verdict goes to the root when it is
/// the wallet's own send. `height` is the pass's.
pub(crate) fn record_probe(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    root: &DueRoot,
    verdict: &ProbeVerdict,
) -> Vec<Outgoing> {
    let mut events = Vec::new();
    if root.own {
        state.publish_all(
            wallet_id,
            [root.txid],
            verdict,
            height,
            root.trigger(),
            &mut events,
        );
    }
    events
}

/// The lowest height at which a root of the wallet can be due, if nothing new
/// happens to it: a root without a schedule yet is due at once; `u32::MAX`
/// when there is nothing to probe.
pub(crate) fn next_pass_height(
    state: &ResolverState,
    wallet_id: WalletId,
    views: &[OutgoingView],
) -> u32 {
    ChainGraph::new(views)
        .roots()
        .iter()
        .map(|view| {
            state
                .schedules
                .get(&(wallet_id, view.txid))
                .map_or(0, ProbeSchedule::next_due)
        })
        .min()
        .unwrap_or(u32::MAX)
}

fn log_outgoing(item: &Outgoing) {
    match &item.event {
        ResolverEvent::Verdict(txid, verdict) => tracing::info!(
            wallet_id = %hex::encode(item.wallet_id),
            txid = %txid,
            height = ?item.height,
            trigger = item.trigger,
            ?verdict,
            "broadcast probe: verdict for an unconfirmed send"
        ),
        ResolverEvent::Cleared(txid) => tracing::info!(
            wallet_id = %hex::encode(item.wallet_id),
            txid = %txid,
            height = ?item.height,
            trigger = item.trigger,
            "broadcast probe: verdict cleared"
        ),
    }
}

// ---- The actor -------------------------------------------------------------

/// What a pass reads from a wallet.
pub(crate) struct WalletRead {
    pub views: Vec<OutgoingView>,
    pub synced_height: u32,
    /// The requested `Uncertain` txids the wallet actually holds.
    pub forced: HashSet<Txid>,
}

/// An `Uncertain` result is queued for every wallet; only one holding the
/// transaction acts on it. `None`: nothing to do for this wallet — the pass
/// need not `walk` the records (it is confined to forced roots) and the
/// wallet holds none of `forced`.
fn forced_held(
    walk: bool,
    forced: HashSet<Txid>,
    holds: impl Fn(&Txid) -> bool,
) -> Option<HashSet<Txid>> {
    let forced: HashSet<Txid> = forced.into_iter().filter(|txid| holds(txid)).collect();
    (walk || !forced.is_empty()).then_some(forced)
}

/// Which wallets hold a transaction (see [`WalletSource::holders`]).
#[derive(Debug, Default)]
pub(crate) struct Holders {
    /// Every wallet whose records hold it: where it is probed.
    pub wallets: Vec<WalletId>,
    /// Those where it is an unsettled own send: the ones its echo could
    /// still be news to.
    pub own: Vec<WalletId>,
}

/// Where passes read wallets from.
#[async_trait]
pub(crate) trait WalletSource: Send + Sync {
    /// The wallets whose records hold `txid`, and among them those where it
    /// is an unsettled own send — one look under one lock.
    async fn holders(&self, txid: Txid) -> Holders;

    /// `None`: no such wallet. `Some(None)`: nothing to do (see
    /// [`forced_held`]) — decided without walking the records. `walk` is read
    /// once the wallet is at hand: a catch-up may clear it meanwhile.
    async fn read(
        &self,
        wallet_id: WalletId,
        walk: &AtomicBool,
        forced: HashSet<Txid>,
    ) -> Option<Option<WalletRead>>;
}

/// Whether any account of the wallet records `txid` — a map lookup per
/// account, not a walk over the records.
fn holds(info: &PlatformWalletInfo, txid: &Txid) -> bool {
    info.core_wallet
        .accounts
        .all_accounts()
        .iter()
        .any(|account| account.transactions().contains_key(txid))
}

/// Whether the wallet records `txid` (`None` if not) and, if it does, whether
/// it is an own send no account has settled (in a block, InstantSend-locked,
/// or finalized) — the same rule as [`merge_records`]: settled in any account
/// means settled. One pass over the accounts.
fn holding(wallet: &Wallet, info: &PlatformWalletInfo, txid: &Txid) -> Option<bool> {
    let signs = wallet.can_sign();
    let accounts = info.core_wallet.accounts.all_accounts();
    // No allocation: `is_unsettled_own` stops at the first settled record,
    // which has been seen by then.
    let mut held = false;
    let own = is_unsettled_own(
        accounts
            .iter()
            .filter_map(|account| {
                account.transactions().get(txid).map(|record| {
                    record_facts(record, || account.transaction_is_finalized(txid), signs)
                })
            })
            .inspect(|_| held = true),
    );
    held.then_some(own)
}

struct ManagerSource(Arc<RwLock<WalletManager<PlatformWalletInfo>>>);

#[async_trait]
impl WalletSource for ManagerSource {
    async fn holders(&self, txid: Txid) -> Holders {
        let manager = self.0.read().await;
        let mut holders = Holders::default();
        for wallet_id in manager.list_wallets() {
            let Some((wallet, info)) = manager.get_wallet_and_info(wallet_id) else {
                continue;
            };
            if let Some(own) = holding(wallet, info, &txid) {
                holders.wallets.push(*wallet_id);
                if own {
                    holders.own.push(*wallet_id);
                }
            }
        }
        holders
    }

    async fn read(
        &self,
        wallet_id: WalletId,
        walk: &AtomicBool,
        forced: HashSet<Txid>,
    ) -> Option<Option<WalletRead>> {
        let manager = self.0.read().await;
        let (wallet, info) = manager.get_wallet_and_info(&wallet_id)?;
        // Checked again under this lock: the record may have left since the
        // holders lookup (a sweep, a removal).
        let forced = forced_held(walk.load(Ordering::SeqCst), forced, |txid| {
            holds(info, txid)
        });
        Some(forced.map(|forced| WalletRead {
            views: collect_views(info, wallet.can_sign()),
            synced_height: info.core_wallet.synced_height(),
            forced,
        }))
    }
}

/// Where events for the host go.
pub(crate) trait VerdictSink: Send + Sync {
    fn deliver(&self, item: &Outgoing);
}

/// The host's event callbacks, reached through the event manager this
/// resolver is registered with (weakly: the manager holds the resolver).
#[derive(Default)]
struct EventSink(Mutex<Weak<PlatformEventManager>>);

impl VerdictSink for EventSink {
    fn deliver(&self, item: &Outgoing) {
        log_outgoing(item);
        let events = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .upgrade();
        let Some(events) = events else {
            return;
        };
        match &item.event {
            ResolverEvent::Verdict(txid, verdict) => {
                events.on_outgoing_transaction_probed(&item.wallet_id, txid, verdict)
            }
            ResolverEvent::Cleared(txid) => {
                events.on_outgoing_transaction_cleared(&item.wallet_id, txid)
            }
        }
    }
}

/// Hand `events` to the host. The one production handler — the FFI bridge —
/// cannot panic, and the release profiles abort on a panic anyway; for a Rust
/// embedder whose handler panics under unwinding, the event it panicked on is
/// logged and dropped rather than taking the resolver task down.
fn deliver(sink: &Arc<dyn VerdictSink>, events: Vec<Outgoing>) {
    for item in &events {
        if catch_unwind(AssertUnwindSafe(|| sink.deliver(item))).is_err() {
            tracing::error!(
                wallet_id = %hex::encode(item.wallet_id),
                event = ?item.event,
                "broadcast probe: a host handler panicked; its event is dropped"
            );
        }
    }
}

/// What the resolver is told. Sent from event handlers and the manager; the
/// actor handles them one at a time, in order.
#[derive(Debug)]
enum Command {
    Height {
        wallet_id: WalletId,
        height: u32,
    },
    /// Any other wallet event; `touched` when it may have left something new
    /// to probe (see [`may_leave_work`]).
    Seen {
        wallet_id: WalletId,
        touched: bool,
    },
    /// Transactions a wallet event reports settled (in a block or
    /// InstantSend-locked) or gone (swept): their verdicts are cleared at once,
    /// without a pass — during a catch-up too.
    Settled {
        wallet_id: WalletId,
        txids: HashSet<Txid>,
    },
    Uncertain(Txid),
    /// dash-spv saw a peer echo a send it had reported `Uncertain`.
    Echoed(Txid),
    SetEnabled(bool),
    /// The wallet was registered or loaded into the manager: only this
    /// creates a wallet's entry, so events queued behind a removal cannot
    /// bring one back.
    WalletAdded(WalletId),
    WalletRemoved(WalletId),
    /// dash-spv finished a sync cycle with its headers at `tip` — after
    /// every new block, and when a launch's or a reconnect's catch-up ends.
    /// A wallet within [`CATCH_UP_STEP`] of that tip follows it now: it gets
    /// a followed pass, which the per-height schedule bounds (one probe per
    /// root per height; a root no node answered is due again).
    SyncComplete {
        tip: u32,
    },
}

/// What a finished job reports back to the actor.
enum JobDone {
    /// The wallets holding a transaction reported `Uncertain` (`wallets`,
    /// where it is probed), and those where it is an unsettled own send
    /// (`own`, the ones a later echo could still be news to).
    /// `report` names the `Uncertain` report this lookup answers: a newer
    /// report of the same txid, or a switch-off, makes it obsolete.
    Listed {
        txid: Txid,
        report: u64,
        wallets: Vec<WalletId>,
        own: Vec<WalletId>,
    },
    /// The wallets holding a send that was echoed after its `Uncertain`.
    /// `token` names the echo this lookup answers: a send that settled, or a
    /// wallet that left or re-registered, since then has withdrawn it.
    EchoListed {
        txid: Txid,
        wallets: Vec<WalletId>,
        token: u64,
    },
    Read {
        wallet_id: WalletId,
        run: u64,
        height: u32,
        read: Option<Option<WalletRead>>,
    },
    Probed {
        wallet_id: WalletId,
        run: u64,
        /// The root it probed: a result for a root no longer in flight (its
        /// probe was stopped, but had already finished) is not this one's.
        root: Txid,
        outcome: ProbeOutcome,
    },
    /// [`NO_ANSWER_RETRY`] passed since a probe at `height` that no node
    /// answered.
    Retry {
        wallet_id: WalletId,
        series: u64,
        height: u32,
    },
}

#[derive(Default)]
struct PendingPass {
    height: u32,
    forced: HashSet<Txid>,
    /// The latest height that joined this pass while the wallet followed the
    /// tip; a catch-up step clears it. A pass without one — only `Uncertain`
    /// results asked for it, or a catch-up overtook it — probes only the
    /// roots those results forced and starts no windows.
    anchor_at: Option<u32>,
    /// Roots a retry forces (see [`NO_ANSWER_RETRY`]): probed only if they
    /// are still roots — one whose parent has since shown up unsettled is
    /// not resolved to that parent.
    retried: HashSet<Txid>,
}

/// A pass in flight: reading the wallet, then probing its due roots one at
/// a time. Results carry the run id; a result for a run that was dropped
/// (probing off, wallet removed) is ignored.
struct Run {
    id: u64,
    /// Txids a wallet event settled or removed while the run was reading: the
    /// read may still list them, and they are left out of it. Once the run
    /// probes, a settle prunes its views and due roots instead (a root whose
    /// probe is out is moot once gone from the views). Gone with the run — a
    /// later read is current, a send a reorg brings back included.
    settled: HashSet<Txid>,
    /// Something touched the wallet during the run: its read may be stale, so
    /// the next height must not be skipped.
    woken: bool,
    /// Where this pass starts windows; `None` confines it to forced roots
    /// (see [`PendingPass::anchor_at`]).
    anchor_at: Option<u32>,
    /// What `Uncertain` results forced it to probe: carried over to the next
    /// pass when a rewind stops it.
    forced: HashSet<Txid>,
    /// What a retry forced it to probe (see [`PendingPass::retried`]).
    retried: HashSet<Txid>,
    /// Whether the read walks the records even when none of the forced txids
    /// is held (`anchor_at.is_some()`): shared with the read job, cleared by a
    /// catch-up.
    walk: Arc<AtomicBool>,
    abort: AbortHandle,
    probing: Option<Probing>,
}

struct Probing {
    views: Vec<OutgoingView>,
    height: u32,
    due: VecDeque<DueRoot>,
    /// The root whose probe is out now.
    in_flight: Option<DueRoot>,
}

#[derive(Default)]
struct WalletEntry {
    /// Last synced height reported, for telling catch-up from following.
    height: Option<u32>,
    /// Height advances below this are skipped — no walk over the wallet's
    /// records while no root can be due. `u32::MAX` when nothing is left to
    /// probe (idle). Reset by anything that may have left new work — which
    /// assumes every change to the wallet's records arrives as a touching
    /// wallet event (see [`may_leave_work`]); a path that changes records
    /// without emitting one must send the resolver such an event too.
    wait_until: Option<u32>,
    pending: Option<PendingPass>,
    run: Option<Run>,
    /// The retries at one height after probes no node answered (see
    /// [`NO_ANSWER_RETRY`]): how many were armed, and whether one waits. The
    /// roots carry the unanswered mark in their schedules; the retry
    /// withdraws them when it fires. A retry that merges into a followed
    /// pass already queued at that height rides along with it.
    retry_series: Option<NoAnswerRetries>,
    /// The height of the last anchored pass that read the wallet: a sync
    /// completion at that height (after every block) adds no second walk
    /// over the records.
    followed_pass_at: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct NoAnswerRetries {
    /// Names the series: a timer from before a reset (a rewind, a switch)
    /// matches no later series at the same height.
    series: u64,
    height: u32,
    armed: u32,
    waiting: bool,
}

/// Owns all of the resolver's state. One task; commands and job results are
/// handled one at a time, so no two of them race, and every host callback
/// comes from this task — never from a caller holding its own locks.
struct Actor {
    enabled: bool,
    state: ResolverState,
    wallets: HashMap<WalletId, WalletEntry>,
    next_run: u64,
    probe: Arc<dyn AcceptanceProbe>,
    source: Arc<dyn WalletSource>,
    sink: Arc<dyn VerdictSink>,
    jobs: JoinSet<JobDone>,
    /// Which run each read or probe job belongs to, so a job that panicked
    /// can end its run instead of leaving the wallet stuck behind it.
    /// A probe job also names its root.
    job_runs: HashMap<task::Id, (WalletId, u64, Option<Txid>)>,
    /// Sends dash-spv reported `Uncertain`, with the followed height their
    /// echo window starts at (`None` until the next followed block after the
    /// latest report — a report while the wallets catch up starts nothing):
    /// a later peer echo of one of them is its acceptance. Removed by its
    /// echo, its settling, the latest report's holder lookup if it finds no
    /// unsettled own send,
    /// [`UNCERTAIN_EXPIRY`] followed blocks into its window, and all at
    /// switch-off. One no wallet resolves (its wallet removed, say) lingers
    /// until it expires.
    uncertain: HashMap<Txid, Option<u32>>,
    /// The token of the latest `Uncertain` report per txid, until its holder
    /// lookup completes: only that completion counts (see `JobDone::Listed`).
    /// A newer report, a switch-off, or a wallet leaving or re-registering
    /// withdraws it — like `echo_lookups`. A lookup that panicked leaves its
    /// token until one of those.
    report_lookups: HashMap<Txid, u64>,
    next_report_token: u64,
    /// Every send dash-spv reported `Uncertain` this session, with the
    /// wallets that hold it as an unsettled own send: this process's own
    /// broadcasts. Their roots go first in a pass's queue for as long as they
    /// stay unsettled — past their echo window too — so a backlog fed by a
    /// peer never holds them back. A wallet leaves an entry when the send
    /// settles or leaves it (seen by its next read), or when it is removed.
    reported: HashMap<Txid, HashSet<WalletId>>,
    /// The highest height any wallet has followed the tip at: the one clock
    /// the echo windows run on, so a wallet behind the others (rescanning,
    /// rewound) neither stamps nor expires them.
    followed_tip: Option<u32>,
    /// Names each retry series (see [`NoAnswerRetries`]), so a stale timer
    /// matches no later series.
    next_retry_series: u64,
    /// Echoes whose holder lookup is out, by the token their result carries.
    /// A send that settles, or a wallet that leaves or re-registers, withdraws
    /// them: an acceptance landing after that must not reach the host.
    echo_lookups: HashMap<Txid, u64>,
    next_echo_token: u64,
}

impl Actor {
    fn new(
        probe: Arc<dyn AcceptanceProbe>,
        source: Arc<dyn WalletSource>,
        sink: Arc<dyn VerdictSink>,
    ) -> Self {
        Self {
            enabled: false,
            state: ResolverState::default(),
            wallets: HashMap::new(),
            next_run: 0,
            probe,
            source,
            sink,
            jobs: JoinSet::new(),
            job_runs: HashMap::new(),
            uncertain: HashMap::new(),
            report_lookups: HashMap::new(),
            next_report_token: 0,
            reported: HashMap::new(),
            followed_tip: None,
            next_retry_series: 0,
            echo_lookups: HashMap::new(),
            next_echo_token: 0,
        }
    }

    async fn run(
        mut self,
        mut commands: mpsc::UnboundedReceiver<Command>,
        mut stop: oneshot::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                biased;
                _ = &mut stop => break,
                command = commands.recv() => match command {
                    Some(command) => self.handle(command),
                    None => break,
                },
                Some(joined) = self.jobs.join_next_with_id(), if !self.jobs.is_empty() => {
                    self.joined(joined);
                }
            }
        }
        // Abort every read and probe and wait for them, so nothing the
        // resolver started outlives it.
        self.jobs.shutdown().await;
    }

    fn joined(&mut self, joined: Result<(task::Id, JobDone), JoinError>) {
        match joined {
            Ok((id, done)) => {
                self.job_runs.remove(&id);
                self.job_done(done);
            }
            Err(error) => {
                let owner = self.job_runs.remove(&error.id());
                if !error.is_panic() {
                    return; // aborted: its run was dropped on purpose
                }
                tracing::error!(?owner, %error, "broadcast probe: a job panicked");
                // End the run it belonged to and let the next height probe the
                // wallet again — what the run carried (a forced txid) is lost,
                // but its roots are due: the panicked one is withdrawn, the
                // ones it had not sent yet were never marked. A panicked lookup
                // for an `Uncertain` result belongs to no run: wake every
                // wallet instead.
                match owner {
                    Some((wallet_id, run, probed)) => {
                        // Only the wallet's current run: a stale panic (its run
                        // was dropped) must not undo a newer run's send. The
                        // root whose probe panicked is the one in flight.
                        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                            return;
                        };
                        let Some(current) = entry.run.as_mut().filter(|current| current.id == run)
                        else {
                            return;
                        };
                        // A probe job's panic belongs to its own root only: the
                        // stopped probe of a settled root is not the one out now.
                        let in_flight_txid = current
                            .probing
                            .as_ref()
                            .and_then(|probing| probing.in_flight.as_ref())
                            .map(|root| root.txid);
                        if probed.is_some() && probed != in_flight_txid {
                            return;
                        }
                        let in_flight = current
                            .probing
                            .as_mut()
                            .and_then(|probing| probing.in_flight.take());
                        entry.wait_until = None;
                        if let Some(root) = in_flight {
                            withdraw_send(&mut self.state, wallet_id, root.txid);
                        }
                        self.end_run(wallet_id);
                    }
                    None => {
                        for entry in self.wallets.values_mut() {
                            entry.wait_until = None;
                        }
                    }
                }
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Height { wallet_id, height } => {
                // Only a wallet the actor was told about (see `WalletAdded`).
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                let previous = entry.height.replace(height);
                let rewound = previous.is_some_and(|previous| height < previous);
                if rewound {
                    // A rewind replays the chain: whatever the resolver kept
                    // for the wallet may rest on blocks the replay drops, so
                    // it starts over: every verdict cleared, every root due
                    // at once, its window from the next following pass. A run
                    // under way rests on that state too: it is stopped, and what
                    // `Uncertain` results forced it to probe goes to the next pass.
                    entry.wait_until = None;
                    entry.retry_series = None;
                    entry.followed_pass_at = None;
                    if let Some(run) = entry.run.take() {
                        run.abort.abort();
                        if !run.forced.is_empty() {
                            entry
                                .pending
                                .get_or_insert_with(PendingPass::default)
                                .forced
                                .extend(run.forced);
                        }
                    }
                    let events = self.state.forget_wallet(&wallet_id, "rewound");
                    deliver(&self.sink, events);
                }
                let catch_up = is_catch_up_step(previous, height);
                if catch_up {
                    // The tip moved on, unfollowed: no lower wallet's steps
                    // may run the echo windows now.
                    self.followed_tip = self.followed_tip.max(Some(height));
                } else {
                    self.expire_uncertain(height);
                }
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                if catch_up {
                    // Behind the tip: passes queued or running from before are
                    // confined to what an `Uncertain` result forced — the scan
                    // settles most roots, and its events clear their verdicts
                    // (see `Command::Settled`) — and start no windows. The
                    // sync's completion runs the pass at the tip.
                    if let Some(pending) = entry.pending.as_mut() {
                        pending.anchor_at = None;
                        // A rewind replays lower heights: never run above them.
                        pending.height = height;
                        if pending.forced.is_empty() {
                            entry.pending = None;
                        }
                    }
                    let Some(run) = entry.run.as_mut() else {
                        // A rewind stopped the run: start what it carried.
                        self.start(wallet_id);
                        return;
                    };
                    run.anchor_at = None;
                    run.walk.store(false, Ordering::SeqCst);
                    if let Some(probing) = run.probing.as_mut() {
                        probing.due.retain(|root| root.forced);
                    }
                    // Still reading, with nothing forced: nothing it could
                    // probe — stop its walk over the records now.
                    if run.probing.is_none() && run.forced.is_empty() {
                        run.abort.abort();
                        self.end_run(wallet_id);
                    }
                    return;
                }
                self.queue_followed_pass(wallet_id, height);
            }
            Command::Settled { wallet_id, txids } => {
                let mut next_probe = false;
                // Only a wallet already known: an event queued behind its
                // removal must not bring back an entry nothing would drop.
                if let Some(entry) = self.wallets.get_mut(&wallet_id) {
                    // A pass under way read the wallet before this: it neither
                    // probes these any more nor publishes anything for them.
                    if let Some(run) = entry.run.as_mut() {
                        match run.probing.as_mut() {
                            None => run.settled.extend(txids.iter().copied()),
                            Some(probing) => {
                                probing.due.retain(|root| !txids.contains(&root.txid));
                                probing.views.retain(|view| !txids.contains(&view.txid));
                                // The root whose probe is out settled: stop
                                // that probe and go on with the next root.
                                if probing
                                    .in_flight
                                    .as_ref()
                                    .is_some_and(|root| txids.contains(&root.txid))
                                {
                                    probing.in_flight = None;
                                    run.abort.abort();
                                    next_probe = true;
                                }
                            }
                        }
                    }
                }
                // Settled sends have nothing left to accept: an echo, or a
                // lookup for one already out, must not publish after this.
                for txid in &txids {
                    self.uncertain.remove(txid);
                    self.echo_lookups.remove(txid);
                    if let Some(holders) = self.reported.get_mut(txid) {
                        holders.remove(&wallet_id);
                        if holders.is_empty() {
                            self.reported.remove(txid);
                        }
                    }
                }
                let events = self.state.settle(wallet_id, &txids);
                deliver(&self.sink, events);
                if next_probe {
                    self.probe_next(wallet_id);
                }
            }
            Command::Seen { wallet_id, touched } => {
                // Only a wallet already known (see `Settled`).
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                if touched {
                    entry.wait_until = None;
                    if let Some(run) = entry.run.as_mut() {
                        run.woken = true;
                    }
                }
            }
            Command::Uncertain(txid) => {
                // The switch is decided here, in order with SetEnabled — not
                // by the sender, which may see the flag before the actor does.
                if !self.enabled {
                    return;
                }
                // A new report restarts the window, from the next followed
                // block.
                self.uncertain.insert(txid, None);
                let report = self.next_report_token;
                self.next_report_token += 1;
                self.report_lookups.insert(txid, report);
                // Routed to the registered wallets holding it, including ones
                // no height step has reached yet.
                let source = Arc::clone(&self.source);
                self.jobs.spawn(async move {
                    let Holders { wallets, own } = source.holders(txid).await;
                    JobDone::Listed {
                        txid,
                        report,
                        wallets,
                        own,
                    }
                });
            }
            Command::Echoed(txid) => {
                // Every healthy send is echoed: only one that was `Uncertain`
                // has a verdict to change.
                if !self.enabled || self.uncertain.remove(&txid).is_none() {
                    return;
                }
                let token = self.next_echo_token;
                self.next_echo_token += 1;
                self.echo_lookups.insert(txid, token);
                let source = Arc::clone(&self.source);
                self.jobs.spawn(async move {
                    JobDone::EchoListed {
                        txid,
                        wallets: source.holders(txid).await.own,
                        token,
                    }
                });
            }
            Command::SetEnabled(enabled) => {
                if enabled == self.enabled {
                    return;
                }
                self.enabled = enabled;
                // Either way every wallet waits for nothing, so turning
                // probing back on re-publishes what turning it off cleared.
                for entry in self.wallets.values_mut() {
                    entry.wait_until = None;
                    entry.retry_series = None;
                    entry.followed_pass_at = None;
                    if !enabled {
                        entry.pending = None;
                        if let Some(run) = entry.run.take() {
                            run.abort.abort();
                        }
                    }
                }
                if !enabled {
                    self.uncertain.clear();
                    self.report_lookups.clear();
                    self.reported.clear();
                    self.echo_lookups.clear();
                    let events = self.state.forget_all();
                    deliver(&self.sink, events);
                }
            }
            Command::SyncComplete { tip } => {
                // Every wallet at the tip follows it now: a launch's or a
                // reconnect's catch-up skipped its pass, and the next block
                // can be minutes away. A wallet still far below (rescanning)
                // waits: its windows must not start at a stale height. One
                // without a step since launch reads its own height. While
                // probing is off this does nothing (queue_followed_pass): the
                // next completion, after the next block, covers it.
                let at_tip: Vec<(WalletId, u32)> = self
                    .wallets
                    .iter()
                    .map(|(wallet_id, entry)| (wallet_id, entry, entry.height.unwrap_or(tip)))
                    .filter(|(_, entry, height)| {
                        let anchored_at = [
                            entry.followed_pass_at,
                            entry.run.as_ref().and_then(|run| run.anchor_at),
                            entry.pending.as_ref().and_then(|pending| pending.anchor_at),
                        ];
                        near_tip(*height, tip)
                            && !anchored_at
                                .iter()
                                .any(|at| at.is_some_and(|at| at >= *height))
                    })
                    .map(|(wallet_id, _, height)| (*wallet_id, height))
                    .collect();
                for (wallet_id, height) in at_tip {
                    self.queue_followed_pass(wallet_id, height);
                }
            }
            Command::WalletAdded(wallet_id) => {
                // A fresh start: whatever was kept under this id (a same-id
                // wallet whose removal never reached the resolver) is stale.
                self.drop_wallet(&wallet_id, "re-registered");
                self.wallets.insert(wallet_id, WalletEntry::default());
            }
            Command::WalletRemoved(wallet_id) => self.drop_wallet(&wallet_id, "removed"),
        }
    }

    /// Drop the wallet's entry, stop its pass and forget its state, the host
    /// told to clear what it showed (logged with `trigger`).
    fn drop_wallet(&mut self, wallet_id: &WalletId, trigger: &'static str) {
        if let Some(run) = self.wallets.remove(wallet_id).and_then(|entry| entry.run) {
            run.abort.abort();
        }
        // Which wallet an echo's lookup will name is not known yet: withdraw
        // them all — at worst an acceptance waits for the next probe.
        self.echo_lookups.clear();
        self.report_lookups.clear();
        self.reported.retain(|_, holders| {
            holders.remove(wallet_id);
            !holders.is_empty()
        });
        let events = self.state.forget_wallet(wallet_id, trigger);
        deliver(&self.sink, events);
    }

    /// A wallet followed the tip to `height`: advance the one clock, start
    /// the echo windows of the sends reported since the last followed block,
    /// and forget those [`UNCERTAIN_EXPIRY`] blocks into their window.
    fn expire_uncertain(&mut self, height: u32) {
        if self.followed_tip.is_some_and(|tip| tip >= height) {
            return; // a wallet behind another: not the clock
        }
        self.followed_tip = Some(height);
        self.uncertain.retain(|_, start| {
            let at = *start.get_or_insert(height);
            height < at.saturating_add(UNCERTAIN_EXPIRY)
        });
    }

    /// Start the wallet's pending pass unless one is already running.
    fn start(&mut self, wallet_id: WalletId) {
        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
            return;
        };
        if entry.run.is_some() {
            return; // the running pass takes it up when it ends
        }
        let Some(pending) = entry.pending.take() else {
            return;
        };
        let anchor_at = pending.anchor_at;
        let forced = pending.forced.clone();
        let retried = pending.retried.clone();
        let to_read: HashSet<Txid> = pending.forced.union(&pending.retried).copied().collect();
        // Walk the records if the pass may probe by schedule; one confined to
        // forced roots with none of them held has nothing to read for.
        let walk = Arc::new(AtomicBool::new(anchor_at.is_some()));
        self.next_run += 1;
        let run = self.next_run;
        let source = Arc::clone(&self.source);
        let job_walk = Arc::clone(&walk);
        let abort = self.jobs.spawn(async move {
            let read = source.read(wallet_id, &job_walk, to_read).await;
            JobDone::Read {
                wallet_id,
                run,
                height: pending.height,
                read,
            }
        });
        self.job_runs.insert(abort.id(), (wallet_id, run, None));
        entry.run = Some(Run {
            id: run,
            settled: HashSet::new(),
            woken: false,
            anchor_at,
            forced,
            retried,
            walk,
            abort,
            probing: None,
        });
    }

    fn job_done(&mut self, done: JobDone) {
        match done {
            JobDone::Listed {
                txid,
                report,
                wallets,
                own,
            } => {
                // Only the latest report's lookup: a newer report's own
                // lookup reads the wallets no earlier than that report, so it
                // decides; one from before a switch-off or a wallet's
                // departure belongs to nothing.
                if !self.enabled || self.report_lookups.get(&txid) != Some(&report) {
                    return;
                }
                self.report_lookups.remove(&txid);
                // No wallet holds it as an unsettled own send: its echo
                // would be news to no one.
                if own.is_empty() {
                    self.uncertain.remove(&txid);
                    self.reported.remove(&txid);
                } else {
                    self.reported.entry(txid).or_default().extend(own);
                }
                for wallet_id in wallets {
                    // Only a wallet the actor was told about (see
                    // `WalletAdded`): one removed since the lookup is gone.
                    let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                        continue;
                    };
                    let pending = entry.pending.get_or_insert_with(PendingPass::default);
                    pending.height = pending.height.max(entry.height.unwrap_or(0));
                    pending.forced.insert(txid);
                    yield_to_forced(entry);
                    self.start(wallet_id);
                }
            }
            JobDone::EchoListed {
                txid,
                wallets,
                token,
            } => {
                if !self.enabled || self.echo_lookups.get(&txid) != Some(&token) {
                    return;
                }
                self.echo_lookups.remove(&txid);
                let mut events = Vec::new();
                for wallet_id in wallets {
                    let Some(entry) = self.wallets.get(&wallet_id) else {
                        continue;
                    };
                    // An echo says only that the send is out there: it adds
                    // nothing to a decided verdict (and must not turn a Mined
                    // back into Accepted).
                    if self
                        .state
                        .published
                        .get(&(wallet_id, txid))
                        .is_some_and(is_decided)
                    {
                        continue;
                    }
                    let height = entry.height.unwrap_or(0);
                    self.state.publish_all(
                        wallet_id,
                        [txid],
                        &ProbeVerdict::Accepted,
                        height,
                        "echo",
                        &mut events,
                    );
                }
                deliver(&self.sink, events);
            }
            JobDone::Read {
                wallet_id,
                run,
                height,
                read,
            } => {
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                if !entry.run.as_ref().is_some_and(|current| current.id == run) {
                    return;
                }
                match read {
                    None => {
                        // Gone from the wallet manager. This manager's own
                        // removals queue WalletRemoved under the same guard,
                        // and commands are handled before job results, so only
                        // a removal through `wallet_manager_arc` gets here.
                        // Fail safe: forget its state and end the run.
                        tracing::error!(
                            wallet_id = %hex::encode(wallet_id),
                            "broadcast probe: a registered wallet read as gone"
                        );
                        // Idle, not re-read every block: a same-id wallet's
                        // events or an Uncertain wake it.
                        entry.wait_until = Some(u32::MAX);
                        let events = self.state.forget_wallet(&wallet_id, "gone");
                        deliver(&self.sink, events);
                        self.end_run(wallet_id);
                    }
                    Some(None) => self.end_run(wallet_id),
                    Some(Some(read)) => {
                        // A request queued before any height was reported
                        // carries 0; the wallet's own synced height is better.
                        let height = height.max(read.synced_height);
                        let Some(current) = entry.run.as_mut() else {
                            return; // matched by id above
                        };
                        // The wallet is not where this pass was queued: far
                        // ahead (a catch-up whose height step has not arrived
                        // yet) or behind (a rewind for a rescan). Either way a
                        // catch-up — confine the pass to forced roots.
                        if current.anchor_at.is_some_and(|at| {
                            read.synced_height < at
                                || read.synced_height > at.saturating_add(CATCH_UP_STEP)
                        }) {
                            current.anchor_at = None;
                        }
                        // What an event settled since the read began is not
                        // the pass's any more.
                        let views: Vec<OutgoingView> = read
                            .views
                            .into_iter()
                            .filter(|view| !current.settled.contains(&view.txid))
                            .collect();
                        // A retried txid that is no longer a root is not the
                        // retry's business: forcing it would resolve to a parent
                        // already probed at this height.
                        let graph = ChainGraph::new(&views);
                        let roots_now: HashSet<Txid> =
                            graph.roots().iter().map(|view| view.txid).collect();
                        // A root a report forced (directly or through a child)
                        // is labelled by the report, not by the retry.
                        let reported_roots = graph.roots_of(current.forced.iter().copied());
                        let forced: HashSet<Txid> = read
                            .forced
                            .iter()
                            .filter(|txid| {
                                current.forced.contains(*txid)
                                    || !current.retried.contains(*txid)
                                    || roots_now.contains(*txid)
                            })
                            .copied()
                            .collect();
                        let anchor = current.anchor_at;
                        // Only the sends this wallet still holds unsettled: one
                        // that settled or left it is no longer its priority.
                        let present: HashSet<Txid> = views.iter().map(|view| view.txid).collect();
                        let mut priority = HashSet::new();
                        self.reported.retain(|txid, holders| {
                            if holders.contains(&wallet_id) {
                                if present.contains(txid) {
                                    priority.insert(*txid);
                                } else {
                                    holders.remove(&wallet_id);
                                }
                            }
                            !holders.is_empty()
                        });
                        let mut start = begin_pass(
                            &mut self.state,
                            wallet_id,
                            height,
                            &views,
                            &forced,
                            &priority,
                            anchor,
                        );
                        for root in start.due.iter_mut() {
                            root.retry = current.retried.contains(&root.txid)
                                && !reported_roots.contains(&root.txid);
                        }
                        if anchor.is_some() {
                            entry.followed_pass_at = Some(height);
                        }
                        let Some(current) = entry.run.as_mut() else {
                            return;
                        };
                        current.probing = Some(Probing {
                            views,
                            height,
                            due: start.due,
                            in_flight: None,
                        });
                        // A forced request that came while this pass read.
                        yield_to_forced(entry);
                        // Clears reach the host before the pass goes out to
                        // the network.
                        deliver(&self.sink, start.events);
                        self.probe_next(wallet_id);
                    }
                }
            }
            JobDone::Retry {
                wallet_id,
                series,
                height,
            } => {
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                match entry.retry_series.as_mut() {
                    Some(retries) if retries.series == series => retries.waiting = false,
                    // Reset since (a rewind, a switch-off, a new height).
                    _ => return,
                }
                // Still at that height: a new block's pass probes the
                // unanswered roots anyway (due at the next height).
                if entry.height.is_some_and(|now| now != height) {
                    return;
                }
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    height,
                    "broadcast probe: retrying probes no node answered"
                );
                let roots = withdraw_unanswered(&mut self.state, wallet_id, height);
                // Nothing left to retry (settled since): no pass, and the
                // wait stands.
                if roots.is_empty() {
                    return;
                }
                let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                    return;
                };
                // Due now, though the wait was computed before.
                entry.wait_until = None;
                if !self.enabled {
                    return;
                }
                // A pass of exactly those roots, ahead of the per-pass cap and
                // of nothing else: it takes no scheduled roots (they would
                // fail too while no node answers) and starts no window.
                let pending = entry.pending.get_or_insert_with(PendingPass::default);
                pending.height = pending.height.max(height);
                pending.retried.extend(roots);
                yield_to_forced(entry);
                self.start(wallet_id);
            }
            JobDone::Probed {
                wallet_id,
                run,
                root: probed,
                outcome: ProbeOutcome { verdict, answered },
            } => {
                let Some(current) = self
                    .wallets
                    .get_mut(&wallet_id)
                    .and_then(|entry| entry.run.as_mut())
                    .filter(|current| current.id == run)
                else {
                    return; // a dropped run's result
                };
                // Cannot happen: a probe job is spawned only by a probing run
                // with its root in flight. Fail safe — end the run rather than
                // leave it blocking every later pass for the wallet.
                let Some(probing) = current.probing.as_mut() else {
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        "broadcast probe: a probe result for a run that is not probing"
                    );
                    current.abort.abort();
                    self.end_run(wallet_id);
                    return;
                };
                if probing
                    .in_flight
                    .as_ref()
                    .is_some_and(|root| root.txid != probed)
                {
                    // The stopped probe of a settled root, finished before it
                    // could be stopped: not the root in flight now.
                    return;
                }
                let Some(root) = probing.in_flight.take() else {
                    // Cannot happen: a settle that clears the root in flight
                    // re-arms the run or ends it at once. Log it and carry on
                    // with the roots still due, forced ones included.
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        "broadcast probe: a probe result with no root in flight"
                    );
                    self.probe_next(wallet_id);
                    return;
                };
                if !probing.views.iter().any(|view| view.txid == root.txid) {
                    // Settled or gone while its probe was out: moot.
                    self.probe_next(wallet_id);
                    return;
                }
                let height = probing.height;
                let events = record_probe(&mut self.state, wallet_id, height, &root, &verdict);
                deliver(&self.sink, events);
                mark_answered(&mut self.state, wallet_id, root.txid, answered);
                if !answered {
                    self.arm_no_answer_retry(wallet_id, root.txid, height);
                }
                self.probe_next(wallet_id);
            }
        }
    }

    /// A height-driven pass of a wallet that follows the tip at `height`:
    /// queued unless a pass already runs (its end decides) or no root can be
    /// due yet.
    fn queue_followed_pass(&mut self, wallet_id: WalletId, height: u32) {
        let enabled = self.enabled;
        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
            return;
        };
        // While a pass runs, its end decides the wait: queue the height and
        // let `probe_next` drop it if no root turns out due.
        if !enabled || (entry.run.is_none() && entry.wait_until.is_some_and(|until| height < until))
        {
            return;
        }
        let pending = entry.pending.get_or_insert_with(PendingPass::default);
        pending.height = pending.height.max(height);
        pending.anchor_at = Some(height);
        self.start(wallet_id);
    }

    /// A probe of `root` at `height` reached no node: nothing was learned.
    /// The root is due again at the next height, in any phase of its
    /// schedule (see [`ProbeSchedule::unanswered`]) — the next block probes
    /// it — and at this height only a retry does: after [`NO_ANSWER_RETRY`],
    /// then twice and four times that again (about 1, 3 and 7 minutes after
    /// the first probe), at most [`MAX_NO_ANSWER_RETRIES`] per height, while
    /// no block has come. A retry is a pass of exactly those roots (still
    /// roots), ahead of the per-pass cap: it takes no scheduled roots and
    /// starts no window. While DAPI
    /// stays down, an unanswered root is due once at every block (in its turn
    /// under [`MAX_ROOTS_PER_PASS`] when a backlog is larger) plus the
    /// retries, which go ahead of the cap, until the next block comes. One info line per such root and
    /// probe — an unchanged Unresolved publishes nothing otherwise.
    fn arm_no_answer_retry(&mut self, wallet_id: WalletId, root: Txid, height: u32) {
        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
            return;
        };
        let retries = match entry.retry_series {
            Some(retries) if retries.height == height => retries,
            _ => {
                self.next_retry_series += 1;
                NoAnswerRetries {
                    series: self.next_retry_series,
                    height,
                    armed: 0,
                    waiting: false,
                }
            }
        };
        entry.retry_series = Some(retries);
        if retries.waiting {
            tracing::info!(
                wallet_id = %hex::encode(wallet_id),
                txid = %root,
                height,
                "broadcast probe: no node answered; the waiting retry covers it"
            );
            return;
        }
        if retries.armed >= MAX_NO_ANSWER_RETRIES {
            tracing::info!(
                wallet_id = %hex::encode(wallet_id),
                txid = %root,
                height,
                "broadcast probe: no node answered; retries at this height used up, \
                 waiting for the next block"
            );
            return;
        }
        let delay = NO_ANSWER_RETRY * 2u32.pow(retries.armed);
        entry.retry_series = Some(NoAnswerRetries {
            armed: retries.armed + 1,
            waiting: true,
            ..retries
        });
        let series = retries.series;
        tracing::info!(
            wallet_id = %hex::encode(wallet_id),
            txid = %root,
            height,
            "broadcast probe: no node answered; probing again in {}s",
            delay.as_secs()
        );
        self.jobs.spawn(async move {
            tokio::time::sleep(delay).await;
            JobDone::Retry {
                wallet_id,
                series,
                height,
            }
        });
    }

    /// Probe the run's next due root, or end the run.
    fn probe_next(&mut self, wallet_id: WalletId) {
        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
            return;
        };
        let Some(run) = entry.run.as_mut() else {
            return;
        };
        let Some(probing) = run.probing.as_mut() else {
            return;
        };
        if let Some(root) = probing.due.pop_front() {
            let probe = Arc::clone(&self.probe);
            let id = run.id;
            let transaction = Arc::clone(&root.transaction);
            let root_txid = root.txid;
            run.abort = self.jobs.spawn(async move {
                let outcome = probe.probe(&transaction).await;
                JobDone::Probed {
                    wallet_id,
                    run: id,
                    root: root_txid,
                    outcome,
                }
            });
            self.job_runs
                .insert(run.abort.id(), (wallet_id, id, Some(root_txid)));
            mark_sent(
                &mut self.state,
                wallet_id,
                root.txid,
                probing.height,
                run.anchor_at,
            );
            probing.in_flight = Some(root);
            return;
        }
        entry.wait_until =
            (!run.woken).then(|| next_pass_height(&self.state, wallet_id, &probing.views));
        self.end_run(wallet_id);
    }

    /// End the wallet's run and start what is queued — except a height
    /// queued during the run that no root is due at; a forced request always
    /// runs.
    fn end_run(&mut self, wallet_id: WalletId) {
        let Some(entry) = self.wallets.get_mut(&wallet_id) else {
            return;
        };
        entry.run = None;
        if let (Some(pending), Some(until)) = (&entry.pending, entry.wait_until) {
            if pending.forced.is_empty() && pending.height < until {
                entry.pending = None;
            }
        }
        self.start(wallet_id);
    }
}

enum ActorSlot {
    /// Waiting for the first command sent from inside a Tokio runtime.
    NotStarted(Box<Actor>, mpsc::UnboundedReceiver<Command>),
    Running(JoinHandle<()>, oneshot::Sender<()>),
    /// Told to stop but not yet ended within a budget: a retried stop joins
    /// it again rather than reporting it gone.
    Stopping(JoinHandle<()>),
    Closed,
}

/// Probes unconfirmed sends and delivers each change of a send's verdict as
/// [`PlatformEventHandler::on_outgoing_transaction_probed`], and each verdict
/// the host must drop as
/// [`PlatformEventHandler::on_outgoing_transaction_cleared`] — always from
/// the resolver's own task, asynchronously to whoever caused it.
///
/// Off until the host turns it on with [`set_enabled`](Self::set_enabled): a
/// probe sends the signed transaction to evonodes over DAPI, which is a
/// product decision, not something a wallet should start doing on upgrade.
pub(crate) struct BroadcastResolver {
    enabled: AtomicBool,
    /// Held while the flag is stored and the switch is queued, so the flag
    /// and the actor end in the same state under concurrent calls — event
    /// handlers gate on the flag.
    switching: Mutex<()>,
    commands: mpsc::UnboundedSender<Command>,
    /// The actor has been started (or the resolver closed): `send` no longer
    /// needs the slot.
    started: AtomicBool,
    actor: Mutex<ActorSlot>,
    /// Serialises [`stop_within`](Self::stop_within) calls.
    stopping: AsyncMutex<()>,
    sink: Arc<EventSink>,
}

impl BroadcastResolver {
    pub(crate) fn new(
        probe: Arc<dyn AcceptanceProbe>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    ) -> Self {
        let sink = Arc::new(EventSink::default());
        let actor = Actor::new(
            probe,
            Arc::new(ManagerSource(wallet_manager)),
            Arc::clone(&sink) as Arc<dyn VerdictSink>,
        );
        let (commands, receiver) = mpsc::unbounded_channel();
        Self {
            enabled: AtomicBool::new(false),
            commands,
            started: AtomicBool::new(false),
            actor: Mutex::new(ActorSlot::NotStarted(Box::new(actor), receiver)),
            stopping: AsyncMutex::new(()),
            switching: Mutex::new(()),
            sink,
        }
    }

    /// Where verdicts are delivered. Weak: the event manager holds this
    /// resolver as one of its handlers.
    pub(crate) fn set_event_manager(&self, events: Weak<PlatformEventManager>) {
        *self.sink.0.lock().unwrap_or_else(PoisonError::into_inner) = events;
    }

    fn send(&self, command: Command) {
        // Fails only once the actor is gone (shutdown); nothing to do then.
        let _ = self.commands.send(command);
        if self.started.load(Ordering::Acquire) {
            return;
        }
        let Ok(runtime) = Handle::try_current() else {
            return; // queued; started by the first command sent from a runtime
        };
        let mut slot = self.actor.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*slot, ActorSlot::NotStarted(..)) {
            if let ActorSlot::NotStarted(actor, receiver) =
                mem::replace(&mut *slot, ActorSlot::Closed)
            {
                let (stop, stopped) = oneshot::channel();
                *slot = ActorSlot::Running(runtime.spawn(actor.run(receiver, stopped)), stop);
            }
        }
        self.started.store(true, Ordering::Release);
    }

    /// Turn probing on or off. Turning it off forgets every send and has the
    /// host told to drop every verdict it holds — with no passes running,
    /// nothing would ever clear them — and drops the passes in flight. Either
    /// way every wallet is woken, so turning it back on re-publishes what was
    /// cleared. Returns at once; the clears follow from the resolver's task.
    pub(crate) fn set_enabled(&self, enabled: bool) {
        let _switching = self
            .switching
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.enabled.store(enabled, Ordering::SeqCst);
        self.send(Command::SetEnabled(enabled));
    }

    /// The last value passed to [`set_enabled`](Self::set_enabled).
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Forget a removed wallet — call after it has left the wallet manager:
    /// its pass in flight, its entry and every verdict, each of which the host
    /// is told to drop (from the resolver's task, not the caller's).
    pub(crate) fn wallet_removed(&self, wallet_id: &WalletId) {
        self.send(Command::WalletRemoved(*wallet_id));
    }

    /// A wallet was registered or loaded — call after it is in the manager.
    pub(crate) fn wallet_added(&self, wallet_id: &WalletId) {
        self.send(Command::WalletAdded(*wallet_id));
    }

    /// Stop the resolver's task: it aborts every read and probe it started and
    /// waits for them, all within `budget`; after this no host callback comes
    /// from the resolver. `NotRunning` if it never started or was already
    /// stopped; `Timeout` if it did not end in time (a later call waits for
    /// it again); `Panicked` if it had died. Concurrent calls take turns.
    pub(crate) async fn stop_within(&self, budget: Duration) -> WorkerStatus {
        self.started.store(true, Ordering::Release);
        stop_slot(&self.actor, &self.stopping, budget).await
    }
}

/// See [`BroadcastResolver::stop_within`].
async fn stop_slot(
    actor: &Mutex<ActorSlot>,
    stopping: &AsyncMutex<()>,
    budget: Duration,
) -> WorkerStatus {
    let deadline = Instant::now() + budget;
    // One stop at a time: a second caller waits for the first instead of
    // finding the slot emptied and reporting a live task as not running.
    let Ok(_one_at_a_time) = timeout_at(deadline, stopping.lock()).await else {
        return WorkerStatus::Timeout;
    };
    let slot = mem::replace(
        &mut *actor.lock().unwrap_or_else(PoisonError::into_inner),
        ActorSlot::Closed,
    );
    let handle = match slot {
        ActorSlot::Running(handle, stop) => {
            let _ = stop.send(());
            handle
        }
        ActorSlot::Stopping(handle) => handle,
        ActorSlot::NotStarted(..) | ActorSlot::Closed => return WorkerStatus::NotRunning,
    };
    // Put back unless the task is joined here: on a timeout, and also when the
    // caller drops this future mid-wait — otherwise the only handle goes with
    // it and a retried stop would report a live task as not running.
    let mut held = HeldActor {
        actor,
        handle: Some(handle),
    };
    let Some(handle) = held.handle.as_mut() else {
        return WorkerStatus::NotRunning;
    };
    let joined = timeout_at(deadline, handle).await;
    match joined {
        Ok(result) => {
            held.handle = None;
            match result {
                Ok(()) => WorkerStatus::Ok,
                Err(error) if error.is_panic() => WorkerStatus::Panicked(error.to_string()),
                Err(error) => WorkerStatus::Stopped(Some(error.to_string())),
            }
        }
        // Told to stop, still going — likely inside a host callback. Not
        // aborted: `held` puts it back, and a retried stop waits for it to end
        // on its own and can then report it clean.
        Err(_) => WorkerStatus::Timeout,
    }
}

/// The resolver task's handle while a stop waits for it. Dropped with the
/// handle still in it — the wait timed out, or the stopping future was
/// cancelled — it goes back into the slot as [`ActorSlot::Stopping`].
struct HeldActor<'a> {
    actor: &'a Mutex<ActorSlot>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for HeldActor<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            *self.actor.lock().unwrap_or_else(PoisonError::into_inner) =
                ActorSlot::Stopping(handle);
        }
    }
}

/// Whether a wallet event may have left something new to probe. A chain lock
/// (it only promotes records their block already reported), or a block that
/// only matured coins or moved balances, does not.
fn may_leave_work(event: &WalletEvent) -> bool {
    match event {
        WalletEvent::TransactionDetected { .. }
        | WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::TransactionsSwept { .. } => true,
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => !inserted.is_empty() || !updated.is_empty(),
        // A chain lock only promotes records their block already reported.
        WalletEvent::ChainLockProcessed { .. } | WalletEvent::SyncHeightAdvanced { .. } => false,
    }
}

/// The transactions a wallet event reports settled — in a block or
/// InstantSend-locked — or gone from the wallet (swept). A chain lock adds
/// nothing: it only promotes records their block already reported.
fn settled_or_gone(event: &WalletEvent) -> HashSet<Txid> {
    match event {
        WalletEvent::TransactionDetected { record, .. } => is_settled(record)
            .then_some(record.txid)
            .into_iter()
            .collect(),
        WalletEvent::TransactionInstantLocked { txid, .. } => HashSet::from([*txid]),
        WalletEvent::TransactionsSwept { txids, .. } => txids.iter().copied().collect(),
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => inserted
            .iter()
            .chain(updated)
            .filter(|record| is_settled(record))
            .map(|record| record.txid)
            .collect(),
        // A chain lock only promotes records already in a block — reported
        // settled when their block was processed.
        WalletEvent::ChainLockProcessed { .. } | WalletEvent::SyncHeightAdvanced { .. } => {
            HashSet::new()
        }
    }
}

fn is_settled(record: &TransactionRecord) -> bool {
    record.is_confirmed() || matches!(record.context, TransactionContext::InstantSend(_))
}

impl EventHandler for BroadcastResolver {
    fn on_wallet_event(&self, event: &WalletEvent) {
        let wallet_id = event.wallet_id();
        // While probing is off nothing is published, so there is nothing to
        // clear (turning it off cleared everything).
        let settled = if self.is_enabled() {
            settled_or_gone(event)
        } else {
            HashSet::new()
        };
        if !settled.is_empty() {
            self.send(Command::Settled {
                wallet_id,
                txids: settled,
            });
        }
        let command = match event {
            WalletEvent::SyncHeightAdvanced { height, .. } => Command::Height {
                wallet_id,
                height: *height,
            },
            other => Command::Seen {
                wallet_id,
                touched: may_leave_work(other),
            },
        };
        self.send(command);
    }

    fn on_sync_event(&self, event: &SyncEvent) {
        if let Some(command) = sync_command(event) {
            self.send(command);
        }
    }
}

/// The command an SPV sync event means for the resolver, if any.
fn sync_command(event: &SyncEvent) -> Option<Command> {
    match event {
        SyncEvent::TransactionBroadcastResult {
            txid,
            result: BroadcastResult::Uncertain,
        } => Some(Command::Uncertain(*txid)),
        // dash-spv lets a send it reported Uncertain still turn Accepted when
        // a peer echoes it later.
        SyncEvent::TransactionBroadcastResult {
            txid,
            result: BroadcastResult::Accepted { .. },
        } => Some(Command::Echoed(*txid)),
        // A tip of 0 is dash-spv's "unknown": no height to follow.
        SyncEvent::SyncComplete { header_tip, .. } if *header_tip > 0 => {
            Some(Command::SyncComplete { tip: *header_tip })
        }
        _ => None,
    }
}

impl PlatformEventHandler for BroadcastResolver {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::hashes::Hash;
    use dashcore::Address;
    use dashcore::{BlockHash, OutPoint, TxIn};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        InputDetail, TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    use key_wallet::transaction_checking::BlockInfo;
    use key_wallet::Network;
    use key_wallet::WalletCoreBalance;
    use tokio::sync::Notify;
    use tokio::time::timeout;

    use super::*;
    use crate::test_support::test_platform_wallet_manager;

    fn txid(n: u8) -> Txid {
        Txid::from_byte_array([n; 32])
    }

    fn outpoint(n: u8, vout: u32) -> OutPoint {
        OutPoint {
            txid: txid(n),
            vout,
        }
    }

    /// Fixtures key transactions by `lock_time` == txid byte, so a probe can
    /// tell which one it was handed.
    fn tx(n: u8, inputs: &[OutPoint]) -> Transaction {
        Transaction {
            version: 2,
            lock_time: n as u32,
            input: inputs
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn send(n: u8, inputs: &[OutPoint]) -> OutgoingView {
        OutgoingView {
            txid: txid(n),
            spends_own_coins: true,
            transaction: Arc::new(tx(n, inputs)),
        }
    }

    fn incoming(n: u8, inputs: &[OutPoint]) -> OutgoingView {
        OutgoingView {
            spends_own_coins: false,
            ..send(n, inputs)
        }
    }

    fn roots(views: &[OutgoingView]) -> BTreeSet<Txid> {
        ambiguous_roots(views).into_iter().map(|v| v.txid).collect()
    }

    fn none() -> HashSet<Txid> {
        HashSet::new()
    }

    // ---- merge_records ------------------------------------------------------

    fn facts(n: u8, settled: bool, own: bool, transaction: &Transaction) -> RecordFacts<'_> {
        RecordFacts {
            txid: txid(n),
            settled,
            own,
            transaction,
        }
    }

    /// A transfer between the wallet's own accounts is recorded twice: as
    /// incoming in the receiving account (listed first) and as spending in the
    /// sending one. It is the wallet's own send either way.
    #[test]
    fn should_treat_a_transfer_between_own_accounts_as_the_wallets_send() {
        let t = tx(1, &[outpoint(90, 0)]);
        let views = merge_records([facts(1, false, false, &t), facts(1, false, true, &t)]);
        assert_eq!(views.len(), 1);
        assert!(views[0].spends_own_coins);
    }

    /// A transaction settled in any account is settled.
    #[test]
    fn should_drop_a_transaction_settled_in_any_account() {
        let t = tx(1, &[outpoint(90, 0)]);
        assert!(merge_records([facts(1, false, true, &t), facts(1, true, false, &t)]).is_empty());
        assert!(merge_records([facts(1, true, false, &t), facts(1, false, true, &t)]).is_empty());
    }

    // ---- ownership of a record ---------------------------------------------

    fn standard_account() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    /// A contact's watch-only chain: we see its coins but hold no key.
    fn contact_account() -> AccountType {
        AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id: [1u8; 32],
            friend_identity_id: [2u8; 32],
        }
    }

    /// An unconfirmed record of `transaction` in `account` that spends one
    /// coin the account tracked (upstream fills `input_details` from the
    /// account's own UTXOs, watch-only ones included).
    fn spending_record(account: AccountType, transaction: Transaction) -> TransactionRecord {
        TransactionRecord::new(
            transaction,
            account,
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 100_000,
                address: Address::dummy(Network::Testnet, 3),
            }],
            Vec::new(),
            -100_000,
        )
    }

    /// The production holder lookup, on a real wallet: every record holding
    /// the txid makes the wallet a holder, and only an unsettled own send
    /// (spends the wallet's coins, signed by it, not settled) makes it one an
    /// echo could still be news to.
    #[tokio::test]
    async fn should_tell_holders_from_unsettled_own_holders_in_a_real_wallet() {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let platform_wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let wallet_manager = Arc::clone(platform_wallet.wallet_manager());
        {
            let mut guard = wallet_manager.write().await;
            let (_, info) = guard.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            let records = info
                .core_wallet
                .accounts
                .funds_account_mut(&standard_account())
                .expect("bip44 account")
                .transactions_mut();
            let own = spending_record(standard_account(), tx(1, &[outpoint(90, 0)]));
            let incoming = record(2, TransactionContext::Mempool);
            let mut settled = spending_record(standard_account(), tx(3, &[outpoint(91, 0)]));
            settled.context = in_block();
            for record in [own, incoming, settled] {
                records.insert(record.txid, record);
            }
        }
        // Records are keyed by the real txid.
        let id = |n: u8| tx(n, &[]).txid();
        let (own_txid, incoming_txid, settled_txid) = (
            tx(1, &[outpoint(90, 0)]).txid(),
            record(2, TransactionContext::Mempool).txid,
            tx(3, &[outpoint(91, 0)]).txid(),
        );
        let source = ManagerSource(wallet_manager);

        let own = source.holders(own_txid).await;
        assert_eq!((own.wallets, own.own), (vec![wallet_id], vec![wallet_id]));
        for not_own in [incoming_txid, settled_txid] {
            let holders = source.holders(not_own).await;
            assert_eq!(holders.wallets, vec![wallet_id], "held");
            assert!(holders.own.is_empty(), "but not an unsettled own send");
        }
        assert!(source.holders(id(4)).await.wallets.is_empty());
    }

    /// The contact spending a payment it received from us: our watch-only
    /// copy of its chain records the spend, but it is the contact's send.
    /// It gets no own-send verdict and no probe of its own.
    #[test]
    fn should_not_treat_a_contact_spend_as_the_wallets_send() {
        let record = spending_record(contact_account(), tx(1, &[outpoint(90, 0)]));
        let facts = record_facts(&record, || false, true);
        assert!(!facts.own, "a contact's spend is not the wallet's send");

        let views = merge_records([record_facts(&record, || false, true)]);
        assert_eq!(views.len(), 1);
        assert!(!views[0].spends_own_coins);
        let graph = ChainGraph::new(&views);
        assert!(graph.own_unsettled().is_empty());
        assert!(roots(&views).is_empty(), "nothing of ours depends on it");
        // Neither a scheduled pass nor an `Uncertain` report of it probes it.
        let start = begin_pass(
            &mut ResolverState::default(),
            wallet(),
            100,
            &views,
            &HashSet::from([record.txid]),
            &HashSet::new(),
            Some(100),
        );
        assert!(start.due.is_empty(), "no probe for a contact's own spend");
        // A late acceptance is news to no wallet.
        assert!(!is_unsettled_own([record_facts(&record, || false, true)]));
    }

    /// The same spend recorded by a signing account is the wallet's send.
    #[test]
    fn should_treat_a_spend_from_a_signing_account_as_the_wallets_send() {
        let record = spending_record(standard_account(), tx(1, &[outpoint(90, 0)]));
        assert!(record_facts(&record, || false, true).own);
        assert!(is_unsettled_own([record_facts(&record, || false, true)]));
        assert!(
            !is_unsettled_own([record_facts(&record, || true, true)]),
            "finalized"
        );
        let views = merge_records([record_facts(&record, || false, true)]);
        assert_eq!(roots(&views), BTreeSet::from([record.txid]));
    }

    /// A watch-only wallet signs nothing: a spend of a coin it tracks is
    /// someone else's, even in a standard account.
    #[test]
    fn should_not_treat_a_spend_seen_by_a_watch_only_wallet_as_its_send() {
        let record = spending_record(standard_account(), tx(1, &[outpoint(90, 0)]));
        assert!(!record_facts(&record, || false, false).own);
        assert!(!is_unsettled_own([record_facts(&record, || false, false)]));
        assert!(roots(&merge_records([record_facts(&record, || false, false)])).is_empty());
    }

    /// A contact's unsettled spend that one of our sends builds on is still
    /// walked as its ancestor: the contact's transaction is the root probed,
    /// while only our send can receive a verdict.
    #[test]
    fn should_walk_a_contact_spend_as_the_ancestor_of_an_own_send() {
        let parent = spending_record(contact_account(), tx(5, &[outpoint(80, 0)]));
        let child = spending_record(
            standard_account(),
            tx(
                6,
                &[OutPoint {
                    txid: parent.txid,
                    vout: 0,
                }],
            ),
        );
        let views = merge_records([
            record_facts(&parent, || false, true),
            record_facts(&child, || false, true),
        ]);
        let graph = ChainGraph::new(&views);
        assert_eq!(graph.own_unsettled(), BTreeSet::from([child.txid]));
        assert_eq!(roots(&views), BTreeSet::from([parent.txid]));
        assert!(!graph.is_own(&parent.txid));
        let start = begin_pass(
            &mut ResolverState::default(),
            wallet(),
            100,
            &views,
            &HashSet::new(),
            &HashSet::new(),
            Some(100),
        );
        let due: Vec<(Txid, bool)> = start.due.iter().map(|root| (root.txid, root.own)).collect();
        assert_eq!(
            due,
            vec![(parent.txid, false)],
            "probed, but no verdict of its own"
        );
        assert!(!is_unsettled_own([record_facts(&parent, || false, true)]));
        assert!(is_unsettled_own([record_facts(&child, || false, true)]));
    }

    // ---- ChainGraph ---------------------------------------------------------

    #[test]
    fn should_pick_a_lone_unsettled_send() {
        assert_eq!(
            roots(&[send(1, &[outpoint(90, 0)])]),
            BTreeSet::from([txid(1)])
        );
    }

    /// The customer's shape: sends built on the change of an unsettled send.
    /// Only the first is probed — a node that has not seen it answers
    /// `missingorspent` for the others.
    #[test]
    fn should_pick_only_the_root_of_a_chain() {
        let views = [
            send(1, &[outpoint(90, 0)]),
            send(2, &[outpoint(1, 1)]),
            send(3, &[outpoint(2, 1)]),
        ];
        assert_eq!(roots(&views), BTreeSet::from([txid(1)]));
    }

    /// Once the parent settles it is gone from the views, and the child has no
    /// unsettled parent: it is a root in its own right.
    #[test]
    fn should_treat_the_child_of_a_settled_parent_as_a_root() {
        assert_eq!(
            roots(&[send(2, &[outpoint(1, 1)])]),
            BTreeSet::from([txid(2)])
        );
    }

    /// Nodes that have not seen an unconfirmed incoming payment refuse a send
    /// that spends it, so the send is not probed itself — the incoming parent
    /// is, since its fate decides the send's.
    #[test]
    fn should_probe_the_unconfirmed_incoming_parent_of_a_send_instead_of_the_send() {
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];
        assert_eq!(roots(&views), BTreeSet::from([txid(5)]));
    }

    /// An unconfirmed incoming payment nobody built on is not this wallet's
    /// business to probe.
    #[test]
    fn should_not_probe_an_incoming_payment_the_wallet_did_not_spend() {
        assert!(roots(&[incoming(5, &[outpoint(80, 0)])]).is_empty());
    }

    /// dash-spv reports the uncertain txid, which may be a child: the root of
    /// its chain is what must be probed at once.
    #[test]
    fn should_find_the_root_of_a_forced_child() {
        let views = [
            send(1, &[outpoint(90, 0)]),
            send(2, &[outpoint(1, 1)]),
            send(3, &[outpoint(2, 1)]),
        ];
        let graph = ChainGraph::new(&views);
        assert_eq!(graph.roots_of([txid(3)]), BTreeSet::from([txid(1)]));
        assert!(graph.roots_of([txid(9)]).is_empty(), "unknown txid");
    }

    /// A long chain of the wallet's own unsettled sends — each spending the
    /// last one's change — is walked once: every send is expanded exactly one
    /// time, not once per send that depends on it.
    #[test]
    fn should_expand_each_transaction_of_a_long_chain_once() {
        let views: Vec<OutgoingView> = (1..=200u8)
            .map(|n| {
                if n == 1 {
                    send(n, &[outpoint(250, 0)])
                } else {
                    send(n, &[outpoint(n - 1, 1)])
                }
            })
            .collect();
        let graph = ChainGraph::new(&views);
        let roots: Vec<Txid> = graph.roots().iter().map(|view| view.txid).collect();
        assert_eq!(roots, vec![txid(1)]);
        assert_eq!(graph.expanded.get(), 200);
    }

    /// Sends that share ancestry share the walk: 100 chained sends plus 50
    /// sends each built on the last of them expand 150 transactions, and the
    /// forced lookup for all of them is one more walk of the same size.
    #[test]
    fn should_walk_shared_ancestry_once_for_many_sends() {
        let mut views: Vec<OutgoingView> = (1..=100u8)
            .map(|n| {
                if n == 1 {
                    send(n, &[outpoint(250, 0)])
                } else {
                    send(n, &[outpoint(n - 1, 1)])
                }
            })
            .collect();
        views.extend((101..=150u8).map(|n| send(n, &[outpoint(100, u32::from(n))])));
        let graph = ChainGraph::new(&views);
        let found: BTreeSet<Txid> = graph.roots().iter().map(|view| view.txid).collect();
        assert_eq!(found, BTreeSet::from([txid(1)]));
        assert_eq!(graph.expanded.get(), 150, "roots(): one walk for all sends");
        let before = graph.expanded.get();
        assert_eq!(
            graph.roots_of((101..=150u8).map(txid)),
            BTreeSet::from([txid(1)])
        );
        assert_eq!(graph.expanded.get() - before, 150);
    }

    // ---- ProbeSchedule / catch-up ------------------------------------------

    fn anchored(height: u32) -> ProbeSchedule {
        let mut schedule = ProbeSchedule::default();
        schedule.anchor(height);
        schedule
    }

    #[test]
    fn should_be_due_immediately_when_never_probed() {
        assert!(anchored(100).is_due(100));
    }

    #[test]
    fn should_probe_at_most_once_per_block() {
        let mut s = anchored(100);
        s.probed_at(100);
        assert!(!s.is_due(100));
        assert!(s.is_due(101));
    }

    #[test]
    fn should_slow_down_after_the_every_block_window() {
        let first = 100;
        let mut s = anchored(first);
        let late = first + EVERY_BLOCK_WINDOW;
        s.probed_at(late);
        for height in late + 1..late + SLOW_INTERVAL {
            assert!(!s.is_due(height), "not due at {height}");
        }
        assert!(s.is_due(late + SLOW_INTERVAL));
    }

    /// `next_due` is exactly the first height `is_due` accepts.
    #[test]
    fn should_agree_on_the_next_due_height_with_is_due() {
        let first = 100;
        let mut schedule = anchored(first);
        for probed in [
            first,
            first + 5,
            first + EVERY_BLOCK_WINDOW - 1,
            first + EVERY_BLOCK_WINDOW + 3,
        ] {
            schedule.probed_at(probed);
            let next = schedule.next_due();
            assert!(schedule.is_due(next), "due at {next}");
            for height in probed + 1..next {
                assert!(!schedule.is_due(height), "not due at {height}");
            }
        }
    }

    #[test]
    fn should_treat_only_large_height_steps_as_catch_up() {
        assert!(
            is_catch_up_step(None, 1_000),
            "the first step after launch is catch-up"
        );
        assert!(!is_catch_up_step(Some(999), 1_000));
        assert!(!is_catch_up_step(Some(1_000 - CATCH_UP_STEP), 1_000));
        assert!(is_catch_up_step(Some(1_000 - CATCH_UP_STEP - 1), 1_000));
        assert!(
            is_catch_up_step(Some(1_000), 999),
            "a step back is a rescan, not the tip"
        );
        assert!(!is_catch_up_step(Some(1_000), 1_000));
    }

    // ---- begin_pass / record_probe ------------------------------------------

    /// Answers per txid and records every probe it received.
    struct ScriptedProbe {
        answers: BTreeMap<Txid, ProbeVerdict>,
        probed: StdMutex<Vec<Txid>>,
    }

    impl ScriptedProbe {
        fn new(answers: &[(Txid, ProbeVerdict)]) -> Self {
            Self {
                answers: answers.iter().cloned().collect(),
                probed: StdMutex::new(Vec::new()),
            }
        }

        fn probed(&self) -> Vec<Txid> {
            self.probed.lock().expect("probed").clone()
        }
    }

    #[async_trait]
    impl AcceptanceProbe for ScriptedProbe {
        async fn probe(&self, transaction: &Transaction) -> ProbeOutcome {
            let id = txid(transaction.lock_time as u8);
            self.probed.lock().expect("probed").push(id);
            outcome(
                self.answers
                    .get(&id)
                    .cloned()
                    .unwrap_or(ProbeVerdict::Unresolved {
                        reason: "scripted".to_string(),
                    }),
            )
        }
    }

    fn wallet() -> WalletId {
        [7u8; 32]
    }

    /// The nodes' advisory word that a send is in a block: the root is still
    /// probed until the wallet settles it.
    fn mined() -> ProbeVerdict {
        ProbeVerdict::Mined
    }

    fn unresolved() -> ProbeVerdict {
        ProbeVerdict::Unresolved {
            reason: "no quorum".to_string(),
        }
    }

    /// The reason the fakes give a probe no node answered.
    const NO_ANSWER: &str = "no available addresses";

    /// No node answered at all (all of DAPI unreachable).
    fn unreachable_verdict() -> ProbeVerdict {
        ProbeVerdict::Unresolved {
            reason: NO_ANSWER.to_string(),
        }
    }

    /// What a fake probe reports for `verdict`: answered, except for
    /// [`unreachable_verdict`].
    fn outcome(verdict: ProbeVerdict) -> ProbeOutcome {
        let answered =
            !matches!(&verdict, ProbeVerdict::Unresolved { reason } if reason == NO_ANSWER);
        ProbeOutcome { verdict, answered }
    }

    /// The resolver's bookkeeping plus what the host was sent, driven the way
    /// the actor drives it but without the task.
    #[derive(Default)]
    struct Harness {
        state: ResolverState,
        sent: Vec<ResolverEvent>,
    }

    impl Harness {
        fn push(&mut self, events: Vec<Outgoing>) {
            self.sent.extend(events.into_iter().map(|item| item.event));
        }

        fn forget_wallet(&mut self, wallet_id: &WalletId) {
            let events = self.state.forget_wallet(wallet_id, "removed");
            self.push(events);
        }

        fn forget_all(&mut self) {
            let events = self.state.forget_all();
            self.push(events);
        }
    }

    async fn pass(
        state: &Mutex<Harness>,
        probe: &dyn AcceptanceProbe,
        height: u32,
        views: &[OutgoingView],
    ) -> Vec<ResolverEvent> {
        run(state, probe, wallet(), height, views, &none()).await;
        outbox(state)
    }

    /// One pass: begin, probe each due root with no lock held, record.
    /// Returns the height the wallet may wait for.
    async fn run(
        state: &Mutex<Harness>,
        probe: &dyn AcceptanceProbe,
        wallet_id: WalletId,
        height: u32,
        views: &[OutgoingView],
        forced: &HashSet<Txid>,
    ) -> u32 {
        let due = {
            let mut guard = state.lock().expect("state");
            let start = begin_pass(
                &mut guard.state,
                wallet_id,
                height,
                views,
                forced,
                &HashSet::new(),
                Some(height),
            );
            guard.push(start.events);
            start.due
        };
        // The same per-root steps as the actor's: mark sent, probe, record.
        for root in due {
            mark_sent(
                &mut state.lock().expect("state").state,
                wallet_id,
                root.txid,
                height,
                Some(height),
            );
            let verdict = probe.probe(&root.transaction).await.verdict;
            let mut guard = state.lock().expect("state");
            let events = record_probe(&mut guard.state, wallet_id, height, &root, &verdict);
            guard.push(events);
        }
        let guard = state.lock().expect("state");
        next_pass_height(&guard.state, wallet_id, views)
    }

    fn outbox(state: &Mutex<Harness>) -> Vec<ResolverEvent> {
        std::mem::take(&mut state.lock().expect("state").sent)
    }

    #[tokio::test]
    async fn should_probe_only_roots_and_report_each_verdict() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = pass(&state, &probe, 100, &views).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(1), unresolved())]);
    }

    /// Acceptance by one node's mempool is not settlement: the root keeps
    /// being asked until it settles, but the host hears about it only once.
    #[tokio::test]
    async fn should_keep_probing_an_accepted_root_but_publish_it_once() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        let first = pass(&state, &probe, 100, &views).await;
        let second = pass(&state, &probe, 101, &views).await;

        assert_eq!(
            first,
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)]
        );
        assert!(
            second.is_empty(),
            "an unchanged verdict is not published again"
        );
        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    #[tokio::test]
    async fn should_keep_probing_an_unresolved_root_on_each_new_block() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        let first = pass(&state, &probe, 100, &views).await;
        let same_block = pass(&state, &probe, 100, &views).await;
        let next_block = pass(&state, &probe, 101, &views).await;

        assert_eq!(first, vec![ResolverEvent::Verdict(txid(1), unresolved())]);
        assert!(same_block.is_empty(), "one probe per block");
        assert!(
            next_block.is_empty(),
            "still unresolved: nothing new to publish"
        );
        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// dash-spv's `Uncertain` result makes the root due at once, even inside
    /// the same block as an earlier probe.
    #[tokio::test]
    async fn should_probe_a_forced_root_regardless_of_its_schedule() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        run(
            &state,
            &probe,
            wallet(),
            100,
            &views,
            &HashSet::from([txid(1)]),
        )
        .await;

        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// The customer's shape: the uncertain send spends the change of an
    /// earlier unsettled send. Forcing the child probes its root at once.
    #[tokio::test]
    async fn should_probe_the_root_when_a_child_is_forced() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        pass(&state, &probe, 100, &views).await;
        run(
            &state,
            &probe,
            wallet(),
            100,
            &views,
            &HashSet::from([txid(2)]),
        )
        .await;

        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// A send that settled or left the wallet is forgotten, and the host is
    /// told to drop the verdict it was shown.
    #[tokio::test]
    async fn should_clear_a_published_send_that_is_no_longer_unconfirmed() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        let gone = pass(&state, &probe, 101, &[]).await;

        assert_eq!(gone, vec![ResolverEvent::Cleared(txid(1))]);
        let guard = state.lock().expect("state");
        assert!(guard.state.schedules.is_empty());
        assert!(guard.state.published.is_empty());
    }

    #[tokio::test]
    async fn should_not_let_one_wallets_pass_forget_another_wallets_roots() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        run(&state, &probe, [9u8; 32], 100, &[], &none()).await;

        assert_eq!(state.lock().expect("state").state.schedules.len(), 1);
    }

    /// Answers in turn, then Unresolved; counts the probes.
    struct SequenceProbe {
        answers: StdMutex<VecDeque<ProbeVerdict>>,
        calls: StdMutex<usize>,
    }

    impl SequenceProbe {
        fn new(answers: &[ProbeVerdict]) -> Self {
            Self {
                answers: StdMutex::new(answers.iter().cloned().collect()),
                calls: StdMutex::new(0),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.lock().expect("calls")
        }
    }

    #[async_trait]
    impl AcceptanceProbe for SequenceProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeOutcome {
            *self.calls.lock().expect("calls") += 1;
            outcome(
                self.answers
                    .lock()
                    .expect("answers")
                    .pop_front()
                    .unwrap_or_else(unresolved),
            )
        }
    }

    /// Nodes' word that a send is mined is not settlement: the root keeps
    /// being probed on its schedule, a later probe that finds it only in a
    /// mempool turns the verdict back into Accepted, a probe that learns
    /// nothing keeps the last decided verdict, and only the wallet settling
    /// the send ends it.
    #[tokio::test]
    async fn should_keep_probing_a_mined_root_until_the_wallet_settles_it() {
        let state = Mutex::new(Harness::default());
        let probe = SequenceProbe::new(&[ProbeVerdict::Mined, ProbeVerdict::Accepted]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 0)])];

        assert_eq!(
            pass(&state, &probe, 100, &views).await,
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Mined)]
        );
        assert_eq!(
            pass(&state, &probe, 101, &views).await,
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)],
            "a later probe disagrees"
        );
        assert!(pass(&state, &probe, 102, &views).await.is_empty());
        assert_eq!(probe.calls(), 3, "only the root, on every block");

        // The wallet settles the root: it is cleared and its child is the
        // root now.
        let settled = [send(2, &[outpoint(1, 0)])];
        assert_eq!(
            pass(&state, &probe, 103, &settled).await,
            vec![
                ResolverEvent::Cleared(txid(1)),
                ResolverEvent::Verdict(txid(2), unresolved()),
            ]
        );
    }

    #[tokio::test]
    async fn should_publish_nothing_while_an_incoming_parent_is_only_accepted() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(5), ProbeVerdict::Accepted)]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        assert!(pass(&state, &probe, 100, &views).await.is_empty());
    }

    // ---- ResolverState ------------------------------------------------------

    #[tokio::test]
    async fn should_forget_one_wallet_and_queue_a_clear_for_its_published_sends() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved()), (txid(2), unresolved())]);
        pass(&state, &probe, 100, &[send(1, &[outpoint(90, 0)])]).await;
        run(
            &state,
            &probe,
            [9u8; 32],
            100,
            &[send(2, &[outpoint(91, 0)])],
            &none(),
        )
        .await;
        outbox(&state);

        state.lock().expect("state").forget_wallet(&wallet());

        assert_eq!(outbox(&state), vec![ResolverEvent::Cleared(txid(1))]);
        let guard = state.lock().expect("state");
        assert_eq!(
            guard.state.published.len(),
            1,
            "the other wallet is untouched"
        );
        assert_eq!(guard.state.schedules.len(), 1);
    }

    #[tokio::test]
    async fn should_forget_everything_and_queue_a_clear_for_every_published_send() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved()), (txid(2), unresolved())]);
        pass(&state, &probe, 100, &[send(1, &[outpoint(90, 0)])]).await;
        run(
            &state,
            &probe,
            [9u8; 32],
            100,
            &[send(2, &[outpoint(91, 0)])],
            &none(),
        )
        .await;
        outbox(&state);

        state.lock().expect("state").forget_all();

        assert_eq!(outbox(&state).len(), 2);
        let guard = state.lock().expect("state");
        assert!(guard.state.published.is_empty() && guard.state.schedules.is_empty());
    }

    /// One probe that met only transport errors says nothing new about a send
    /// already seen accepted or mined; the UI must not flicker back to
    /// "unknown". Accepted and Mined replace each other — a later probe may
    /// disagree with an earlier one either way — and a repeat is no news.
    #[test]
    fn should_publish_only_a_change_of_verdict() {
        let mut state = ResolverState::default();
        let key = (wallet(), txid(1));

        assert!(state.publish(key, &ProbeVerdict::Accepted));
        assert!(!state.publish(key, &unresolved()));
        assert!(!state.publish(key, &ProbeVerdict::Accepted));
        assert!(state.publish(key, &ProbeVerdict::Mined));
        assert!(!state.publish(key, &unresolved()));
        assert!(!state.publish(key, &ProbeVerdict::Mined));
        assert!(state.publish(key, &ProbeVerdict::Accepted));
    }

    // ---- the actor ----------------------------------------------------------

    /// Unsettled views per wallet; records every walk over a wallet's records.
    #[derive(Default)]
    struct FakeSource {
        views: StdMutex<HashMap<WalletId, Vec<OutgoingView>>>,
        walks: StdMutex<Vec<WalletId>>,
        /// The synced height each wallet's reads report.
        synced: StdMutex<HashMap<WalletId, u32>>,
        /// Reads wait while set, so a test can hold a pass in its read.
        paused: StdMutex<bool>,
        resume: Notify,
    }

    impl FakeSource {
        async fn wait_if_paused(&self) {
            loop {
                let resumed = self.resume.notified();
                if !*self.paused.lock().expect("paused") {
                    return;
                }
                resumed.await;
            }
        }

        fn pause(&self, paused: bool) {
            *self.paused.lock().expect("paused") = paused;
            if !paused {
                self.resume.notify_waiters();
            }
        }
    }

    #[async_trait]
    impl WalletSource for FakeSource {
        async fn holders(&self, txid: Txid) -> Holders {
            let views = self.views.lock().expect("views");
            let held = |own: bool| {
                views
                    .iter()
                    .filter(|(_, views)| {
                        views
                            .iter()
                            .any(|view| view.txid == txid && (!own || view.spends_own_coins))
                    })
                    .map(|(wallet_id, _)| *wallet_id)
                    .collect()
            };
            Holders {
                wallets: held(false),
                own: held(true),
            }
        }

        async fn read(
            &self,
            wallet_id: WalletId,
            walk: &AtomicBool,
            forced: HashSet<Txid>,
        ) -> Option<Option<WalletRead>> {
            self.wait_if_paused().await;
            let views = self.views.lock().expect("views").get(&wallet_id)?.clone();
            let forced = forced_held(walk.load(Ordering::SeqCst), forced, |txid| {
                views.iter().any(|view| view.txid == *txid)
            });
            Some(forced.map(|forced| {
                self.walks.lock().expect("walks").push(wallet_id);
                WalletRead {
                    views,
                    synced_height: self
                        .synced
                        .lock()
                        .expect("synced")
                        .get(&wallet_id)
                        .copied()
                        .unwrap_or(0),
                    forced,
                }
            }))
        }
    }

    #[derive(Default)]
    struct CollectSink(StdMutex<Vec<ResolverEvent>>);

    impl VerdictSink for CollectSink {
        fn deliver(&self, item: &Outgoing) {
            self.0.lock().expect("sink").push(item.event.clone());
        }
    }

    struct Rig {
        actor: Actor,
        source: Arc<FakeSource>,
        sink: Arc<CollectSink>,
    }

    impl Rig {
        fn new(probe: Arc<dyn AcceptanceProbe>) -> Self {
            Self::with_sink(probe, Arc::new(CollectSink::default()))
        }

        fn with_sink(probe: Arc<dyn AcceptanceProbe>, sink: Arc<CollectSink>) -> Self {
            let source = Arc::new(FakeSource::default());
            let actor = Actor::new(
                probe,
                Arc::clone(&source) as Arc<dyn WalletSource>,
                Arc::clone(&sink) as Arc<dyn VerdictSink>,
            );
            Self {
                actor,
                source,
                sink,
            }
        }

        /// The wallet's unsettled records; registers the wallet if the actor
        /// does not know it, as the manager does before any of its events.
        fn views(&mut self, wallet_id: WalletId, views: Vec<OutgoingView>) {
            self.source
                .views
                .lock()
                .expect("views")
                .insert(wallet_id, views);
            if !self.actor.wallets.contains_key(&wallet_id) {
                self.actor.handle(Command::WalletAdded(wallet_id));
            }
        }

        /// Handle `command`, then every job it caused, as the task would.
        async fn send(&mut self, command: Command) {
            self.handle(command);
            self.settle().await;
        }

        /// Handle `command` without running its jobs. A height step moves the
        /// wallet's synced height along, as the SPV client does before it
        /// reports the step.
        fn handle(&mut self, command: Command) {
            if let Command::Height { wallet_id, height } = command {
                let mut synced = self.source.synced.lock().expect("synced");
                let at = synced.entry(wallet_id).or_insert(0);
                *at = (*at).max(height);
            }
            self.actor.handle(command);
        }

        async fn settle(&mut self) {
            while let Some(joined) = self.actor.jobs.join_next_with_id().await {
                self.actor.joined(joined);
            }
        }

        /// Two height steps: the first after launch is catch-up.
        async fn follow(&mut self, wallet_id: WalletId, from: u32) {
            self.send(Command::Height {
                wallet_id,
                height: from,
            })
            .await;
            self.send(Command::Height {
                wallet_id,
                height: from + 1,
            })
            .await;
        }

        async fn height(&mut self, wallet_id: WalletId, height: u32) {
            self.send(Command::Height { wallet_id, height }).await;
        }

        fn sent(&self) -> Vec<ResolverEvent> {
            std::mem::take(&mut *self.sink.0.lock().expect("sink"))
        }

        fn walks(&self) -> usize {
            self.source.walks.lock().expect("walks").len()
        }
    }

    async fn enabled(probe: Arc<dyn AcceptanceProbe>) -> Rig {
        let mut rig = Rig::new(probe);
        rig.send(Command::SetEnabled(true)).await;
        rig
    }

    #[tokio::test]
    async fn should_ignore_heights_and_uncertain_results_while_off() {
        let mut rig = Rig::new(Arc::new(ScriptedProbe::new(&[])));
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(rig.walks(), 0);
        assert!(rig.sent().is_empty());
    }

    /// An `Uncertain` result carries no wallet id: every wallet hears it, but
    /// only the one holding the transaction walks its records.
    #[tokio::test]
    async fn should_act_on_an_uncertain_result_only_in_the_wallet_that_holds_it() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        let other = [9u8; 32];
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.views(other, vec![send(2, &[outpoint(91, 0)])]);
        for wallet_id in [wallet(), other] {
            rig.send(Command::Seen {
                wallet_id,
                touched: false,
            })
            .await;
        }

        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(*rig.source.walks.lock().expect("walks"), vec![wallet()]);
        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(
            rig.sent(),
            vec![ResolverEvent::Verdict(txid(1), unresolved())]
        );
    }

    /// An `Uncertain` result no registered wallet holds (an unrelated
    /// broadcast) is forgotten once its holder lookup comes back empty.
    #[tokio::test]
    async fn should_forget_an_uncertain_result_no_wallet_holds() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.send(Command::Uncertain(txid(7))).await;

        assert!(rig.actor.uncertain.is_empty());
    }

    /// An echo within [`UNCERTAIN_EXPIRY`] followed blocks of the window an
    /// `Uncertain` result starts (at the next followed block) still makes an
    /// own send Accepted; one after it is ignored — the probes decide by then.
    #[tokio::test]
    async fn should_accept_an_echo_only_within_the_expiry() {
        for (blocks, accepted) in [(UNCERTAIN_EXPIRY - 1, true), (UNCERTAIN_EXPIRY, false)] {
            let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
            let mut rig = enabled(probe).await;
            rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
            rig.follow(wallet(), 100).await; // at 101
            rig.send(Command::Uncertain(txid(1))).await;
            // The window starts at 102.
            for height in 102..=102 + blocks {
                rig.height(wallet(), height).await;
            }
            rig.sent();

            rig.send(Command::Echoed(txid(1))).await;

            assert_eq!(
                rig.sent()
                    .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)),
                accepted,
                "{blocks} blocks into the window"
            );
        }
    }

    /// The window starts only once a wallet follows the tip, and runs on the
    /// highest followed height: catch-up steps and a wallet behind the
    /// others neither start nor end it.
    #[tokio::test]
    async fn should_run_the_echo_window_on_the_followed_tip_only() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        let behind = [9u8; 32];
        rig.send(Command::WalletAdded(behind)).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        // Reported while nothing follows: no window yet.
        rig.send(Command::Uncertain(txid(1))).await;
        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&None));

        // Catch-up steps (the first step, then a jump) start nothing.
        rig.height(wallet(), 500).await;
        rig.height(wallet(), 1000).await;
        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&None));

        // The first followed block starts it.
        rig.height(wallet(), 1001).await;
        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&Some(1001)));

        // A wallet far behind, following its own (lower) steps, changes
        // nothing; nor does a catch-up jump far ahead.
        rig.follow(behind, 600).await;
        rig.height(wallet(), 1001 + UNCERTAIN_EXPIRY + 50).await;
        assert!(rig.actor.uncertain.contains_key(&txid(1)));

        // The tip followed past the window ends it.
        rig.height(wallet(), 1001 + UNCERTAIN_EXPIRY + 51).await;
        assert!(rig.actor.uncertain.is_empty());
    }

    /// A new `Uncertain` report of the same send restarts its echo window
    /// from the next followed block.
    #[tokio::test]
    async fn should_restart_the_echo_window_on_a_new_report() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(1))).await;
        rig.height(wallet(), 102).await;
        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&Some(102)));

        rig.send(Command::Uncertain(txid(1))).await;
        rig.height(wallet(), 103).await;

        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&Some(103)));
    }

    /// A wallet that jumped ahead (catch-up) holds the clock there: another
    /// wallet still scanning far behind, in small steps, neither starts nor
    /// ends an echo window.
    #[tokio::test]
    async fn should_not_let_a_wallet_behind_a_catch_up_run_the_echo_window() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        let behind = [9u8; 32];
        rig.send(Command::WalletAdded(behind)).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 1000).await; // follows at 1001
        rig.height(wallet(), 5000).await; // catch-up jump
        rig.send(Command::Uncertain(txid(1))).await;

        rig.follow(behind, 1500).await;
        for height in 1502..1502 + 2 * UNCERTAIN_EXPIRY {
            rig.height(behind, height).await;
        }

        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&None));
        rig.height(wallet(), 5001).await;
        assert_eq!(rig.actor.uncertain.get(&txid(1)), Some(&Some(5001)));
    }

    /// A send a wallet holds but not as an unsettled own send (an incoming
    /// payment): its echo would be news to no one, so the entry goes — and
    /// the holder still probes its chain.
    #[tokio::test]
    async fn should_drop_an_uncertain_result_held_only_as_incoming_but_still_probe_it() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![incoming(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 0)])],
        );

        rig.send(Command::Uncertain(txid(1))).await;

        assert!(rig.actor.uncertain.is_empty());
        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    fn sync_complete(tip: u32) -> Command {
        Command::SyncComplete { tip }
    }

    /// The log's first shape: the app launches on a stored tip, the network
    /// comes back with one short step (the first after launch, so catch-up)
    /// and then stays quiet for minutes. The sync's completion runs the pass
    /// the catch-up skipped — once: a later completion at the same height,
    /// or after a followed step, probes nothing more.
    #[tokio::test]
    async fn should_probe_at_the_end_of_a_catch_up_then_wait_for_the_next_block() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.height(wallet(), 1_566_577).await; // first step after launch
        assert!(probe.probed().is_empty(), "catch-up: no pass");
        rig.send(sync_complete(1_566_577)).await;
        assert_eq!(
            probe.probed(),
            vec![txid(1)],
            "the skipped pass, at the tip"
        );

        rig.send(sync_complete(1_566_577)).await;
        rig.height(wallet(), 1_566_578).await; // followed
        rig.send(sync_complete(1_566_578)).await;
        assert_eq!(probe.probed(), vec![txid(1), txid(1)], "one per block");
    }

    /// A long catch-up (a reconnect after an outage) ends the same way.
    #[tokio::test]
    async fn should_probe_at_the_end_of_a_long_catch_up() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        let before = probe.probed().len();

        rig.height(wallet(), 150).await; // catch-up
        assert_eq!(probe.probed().len(), before);
        rig.send(sync_complete(150)).await;

        assert_eq!(probe.probed().len(), before + 1);
    }

    /// A launch with no block since the app last ran gives the wallet no
    /// height step at all; the first completion probes it at the tip.
    #[tokio::test]
    async fn should_probe_a_wallet_with_no_step_since_launch_at_the_first_completion() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        // The wallet reads its stored tip, already the network's.
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 1_566_577);

        rig.send(sync_complete(1_566_577)).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// A wallet still rescanning far below the tip gets no pass from a
    /// completion: its windows must not start at a stale height.
    #[tokio::test]
    async fn should_not_probe_a_wallet_far_below_a_completed_tip() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.height(wallet(), 500).await; // catch-up, far below

        rig.send(sync_complete(5_000)).await;

        assert!(probe.probed().is_empty());
    }

    /// The log's second shape: one new block arrives as the network comes
    /// back, but DAPI's addresses are still banned, so its probe reaches no
    /// node. It is repeated after [`NO_ANSWER_RETRY`] rather than at the next
    /// block, minutes away.
    #[tokio::test(start_paused = true)]
    async fn should_retry_a_probe_no_node_answered_without_waiting_for_a_block() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.follow(wallet(), 100).await; // probes at 101; the retry follows

        assert_eq!(probe.calls(), 2);
        assert!(rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)));
    }

    /// Every root no node answered in a pass is retried, by one timer.
    #[tokio::test(start_paused = true)]
    async fn should_retry_every_root_no_node_answered() {
        let probe = Arc::new(ScriptedProbe::new(&[
            (txid(1), unreachable_verdict()),
            (txid(2), unreachable_verdict()),
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );

        rig.follow(wallet(), 100).await;

        // Each retry probes both: the probe and its retries, for each root.
        let probed = probe.probed();
        let each = 1 + MAX_NO_ANSWER_RETRIES as usize;
        assert_eq!(
            probed.iter().filter(|probed| **probed == txid(1)).count(),
            each
        );
        assert_eq!(
            probed.iter().filter(|probed| **probed == txid(2)).count(),
            each
        );
    }

    /// The retries are bounded: [`MAX_NO_ANSWER_RETRIES`] per height, with
    /// growing delays, however long no node answers.
    #[tokio::test(start_paused = true)]
    async fn should_retry_probes_no_node_answered_a_bounded_number_of_times_per_height() {
        let probe = Arc::new(SequenceProbe::new(&vec![unreachable_verdict(); 10]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        let per_height = 1 + MAX_NO_ANSWER_RETRIES as usize;

        let started = Instant::now();
        rig.follow(wallet(), 100).await;
        assert_eq!(
            probe.calls(),
            per_height,
            "the probe and its retries at 101"
        );
        assert!(
            started.elapsed() >= NO_ANSWER_RETRY * 7,
            "1, 2 and 4 times the delay"
        );

        rig.height(wallet(), 102).await;
        assert_eq!(probe.calls(), 2 * per_height, "and again at the next block");
    }

    /// A block arriving while a retry waits takes over: the earlier height's
    /// retry does not cancel the new height's.
    #[tokio::test(start_paused = true)]
    async fn should_keep_the_retry_of_a_new_block_when_an_old_retry_fires() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        // Run the pass at 101 (read, then its probe), leaving its retry
        // asleep.
        for _ in 0..2 {
            let job = rig.actor.jobs.join_next_with_id().await.expect("job");
            rig.actor.joined(job);
        }
        assert_eq!(probe.calls(), 1);

        rig.send(Command::Height {
            wallet_id: wallet(),
            height: 102,
        })
        .await;

        assert_eq!(probe.calls(), 3, "101, 102, then 102's retry");
        assert!(rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)));
    }

    /// Completions that repeat a tip (peer flaps, no new block) probe a
    /// root once per height: the schedule bounds them.
    #[tokio::test]
    async fn should_probe_once_per_height_however_many_completions() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await; // probes at 101

        for _ in 0..3 {
            rig.send(sync_complete(101)).await;
        }

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// A forced pass (an `Uncertain` report's, here at the stored tip before
    /// any step) that no node answered is retried as a forced pass: probed
    /// again without waiting for a block, and no window starts at a height
    /// the wallet may not follow.
    #[tokio::test(start_paused = true)]
    async fn should_retry_a_forced_pass_without_starting_a_window() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(probe.calls(), 2, "the forced probe and its retry");
        assert_eq!(
            rig.actor.state.schedules[&(wallet(), txid(1))].window_start,
            None
        );
        assert!(rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)));
    }

    /// A retry probes only the roots no node answered: the others were
    /// probed at that height.
    #[tokio::test(start_paused = true)]
    async fn should_retry_only_the_roots_no_node_answered() {
        let probe = Arc::new(ScriptedProbe::new(&[
            (txid(1), unreachable_verdict()),
            (txid(2), unresolved()),
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );

        rig.follow(wallet(), 100).await;

        let probed = probe.probed();
        assert_eq!(
            probed.iter().filter(|probed| **probed == txid(2)).count(),
            1,
            "answered: not retried"
        );
        assert_eq!(
            probed.iter().filter(|probed| **probed == txid(1)).count(),
            1 + MAX_NO_ANSWER_RETRIES as usize
        );
    }

    /// A completion at a height whose probe reached no node adds no probe of
    /// its own: the waiting retry is the next try.
    #[tokio::test(start_paused = true)]
    async fn should_leave_an_unanswered_height_to_its_retry_at_completion() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        for _ in 0..2 {
            let job = rig
                .actor
                .jobs
                .join_next_with_id()
                .await
                .expect("read, then probe");
            rig.actor.joined(job);
        }
        assert_eq!(probe.calls(), 1);

        rig.handle(sync_complete(101));
        let entry = &rig.actor.wallets[&wallet()];
        assert!(entry.run.is_none() && entry.pending.is_none(), "no pass");

        rig.settle().await; // the retry
        assert_eq!(probe.calls(), 2);
    }

    /// While probing is off a completion does nothing; the next one, after
    /// it is on again, runs the pass.
    #[tokio::test]
    async fn should_ignore_a_completion_while_probing_is_off() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.height(wallet(), 500).await; // catch-up

        rig.send(Command::SetEnabled(false)).await;
        rig.send(sync_complete(500)).await;
        assert!(probe.probed().is_empty());

        rig.send(Command::SetEnabled(true)).await;
        rig.send(sync_complete(500)).await;
        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// A wallet with no step since launch that reads far below the tip (a
    /// new wallet scanning from its birth height) is not anchored at the tip.
    #[tokio::test]
    async fn should_not_probe_a_wallet_with_no_step_reading_far_below_the_tip() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 1_000);

        rig.send(sync_complete(1_566_577)).await;

        assert!(probe.probed().is_empty());
    }

    /// dash-spv's completion with an unknown tip (0) means nothing to follow.
    #[test]
    fn should_ignore_a_completion_without_a_tip() {
        let complete = |header_tip| SyncEvent::SyncComplete {
            header_tip,
            cycle: 1,
        };
        assert!(sync_command(&complete(0)).is_none());
        assert!(matches!(
            sync_command(&complete(7)),
            Some(Command::SyncComplete { tip: 7 })
        ));
    }

    /// A probe that reached no node does not count in the schedule's pace:
    /// the root is due at the next height even in its slow phase — but not
    /// at the same height, where only a retry probes it again.
    #[test]
    fn should_make_an_unanswered_root_due_at_the_next_height_in_any_phase() {
        let mut schedule = anchored(100);
        schedule.probed_at(130); // past the every-block window
        assert!(!schedule.is_due(131));

        schedule.unanswered = true;

        assert!(!schedule.is_due(130), "not again at the same height");
        assert!(schedule.is_due(131));
        assert_eq!(schedule.next_due(), 131);
    }

    /// Once the retries at a height are used up, completions at that height
    /// probe nothing more; the next block does.
    #[tokio::test(start_paused = true)]
    async fn should_probe_an_unanswered_root_at_the_next_block_after_its_retries() {
        let probe = Arc::new(SequenceProbe::new(&vec![unreachable_verdict(); 10]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        let at_101 = 1 + MAX_NO_ANSWER_RETRIES as usize;
        assert_eq!(probe.calls(), at_101);

        rig.send(sync_complete(101)).await;
        assert_eq!(probe.calls(), at_101, "the retries were the tries at 101");

        rig.height(wallet(), 102).await;
        assert!(probe.calls() > at_101, "the next block probes it");
    }

    /// The production order: the block's completion arrives while that
    /// block's probe is still out. The probe reaches no node; the queued
    /// completion pass probes nothing more at that height — the retry does.
    #[tokio::test(start_paused = true)]
    async fn should_not_probe_twice_at_a_height_when_its_completion_comes_mid_probe() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // the probe is out
        rig.handle(sync_complete(101)); // queued behind the running pass
        let probed = rig.actor.jobs.join_next_with_id().await.expect("probe");
        // No answer: the root is not due again at 101, so the run's end
        // drops the queued pass (its height is below the wait).
        rig.actor.joined(probed);
        assert!(rig.actor.wallets[&wallet()].pending.is_none());
        assert_eq!(probe.calls(), 1, "nothing due again at 101");

        rig.settle().await; // the retry
        assert_eq!(probe.calls(), 2);
    }

    /// A new probe of a root clears its unanswered mark: only that probe's
    /// own answer decides it (a retry must not withdraw a root whose probe is
    /// out).
    #[test]
    fn should_clear_the_unanswered_mark_when_a_probe_goes_out() {
        let mut schedule = anchored(100);
        schedule.probed_at(100);
        schedule.unanswered = true;

        schedule.probed_at(101);

        assert!(!schedule.unanswered);
    }

    /// A rewind or a switch while a retry waits resets the series: the old
    /// timer then does nothing.
    #[tokio::test(start_paused = true)]
    async fn should_drop_a_waiting_retry_on_a_rewind_or_a_switch() {
        for reset in [
            vec![Command::SetEnabled(false), Command::SetEnabled(true)],
            vec![Command::Height {
                wallet_id: wallet(),
                height: 50,
            }],
        ] {
            let probe = Arc::new(SequenceProbe::new(&[unreachable_verdict()]));
            let mut rig = enabled(probe.clone()).await;
            rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
            rig.handle(Command::Height {
                wallet_id: wallet(),
                height: 100,
            });
            rig.handle(Command::Height {
                wallet_id: wallet(),
                height: 101,
            });
            for _ in 0..2 {
                let job = rig
                    .actor
                    .jobs
                    .join_next_with_id()
                    .await
                    .expect("read, then probe");
                rig.actor.joined(job);
            }
            assert!(rig.actor.wallets[&wallet()].retry_series.is_some());

            for command in reset {
                rig.handle(command);
            }
            assert!(rig.actor.wallets[&wallet()].retry_series.is_none());
            let calls = probe.calls();
            rig.settle().await; // the old timer fires
            assert_eq!(probe.calls(), calls, "the old retry does nothing");
        }
    }

    /// The completion after a block whose pass already read the wallet adds
    /// no second walk over its records.
    #[tokio::test]
    async fn should_not_walk_the_records_again_for_the_blocks_completion() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        let walks = rig.walks();

        rig.send(sync_complete(101)).await;

        assert_eq!(rig.walks(), walks);
    }

    /// A retried root that has since become a child (its parent showed up
    /// unsettled) is not forced: the retry would resolve it to the parent,
    /// already probed at that height.
    #[tokio::test(start_paused = true)]
    async fn should_not_force_a_retried_root_that_is_no_longer_a_root() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(5), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        // Send 1 was the root no node answered; its parent 5 has shown up
        // unsettled since, so 1 is a child now.
        rig.views(
            wallet(),
            vec![incoming(5, &[outpoint(80, 0)]), send(1, &[outpoint(5, 0)])],
        );
        let mut pending = PendingPass::default();
        pending.retried.insert(txid(1));
        rig.actor
            .wallets
            .get_mut(&wallet())
            .expect("wallet")
            .pending = Some(pending);

        rig.actor.start(wallet());
        rig.settle().await;

        assert!(
            probe.probed().is_empty(),
            "neither the child nor its parent"
        );
    }

    /// A retry forces a root that still is one.
    #[tokio::test]
    async fn should_force_a_retried_root_that_still_is_one() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        let mut pending = PendingPass::default();
        pending.retried.insert(txid(1));
        rig.actor
            .wallets
            .get_mut(&wallet())
            .expect("wallet")
            .pending = Some(pending);

        rig.actor.start(wallet());
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// The launch case end to end: a wallet with no step yet, its first
    /// completion's probe reaches no node, and the retry probes it again
    /// without waiting for a block.
    #[tokio::test(start_paused = true)]
    async fn should_retry_a_completion_probe_of_a_wallet_with_no_step() {
        let probe = Arc::new(SequenceProbe::new(&[
            unreachable_verdict(),
            ProbeVerdict::Accepted,
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 1_566_577);

        rig.send(sync_complete(1_566_577)).await;

        assert_eq!(probe.calls(), 2);
        assert!(rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)));
    }

    /// An older report's holder lookup that completes after a newer report
    /// of the same txid is obsolete: it found no holder (the record was not
    /// there yet), but must not delete the newer report's echo window.
    #[tokio::test]
    async fn should_not_let_an_obsolete_lookup_delete_a_newer_reports_window() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), Vec::new());
        rig.handle(Command::Uncertain(txid(1)));
        // Its lookup finds no holder: held back.
        let old = rig
            .actor
            .jobs
            .join_next_with_id()
            .await
            .expect("old lookup");
        assert!(matches!(
            &old,
            Ok((_, JobDone::Listed { own, .. })) if own.is_empty()
        ));

        // The owning record arrives, and dash-spv reports the send again.
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Uncertain(txid(1)));
        rig.actor.joined(old); // the old, empty completion lands now
        assert!(
            rig.actor.uncertain.contains_key(&txid(1)),
            "the window stays"
        );
        rig.settle().await;
        rig.sent();

        rig.send(Command::Echoed(txid(1))).await;

        assert!(rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)));
    }

    /// The reverse order: the newer report's lookup completes first, then
    /// the older, empty one — still obsolete.
    #[tokio::test]
    async fn should_drop_an_older_lookup_that_completes_after_the_newer_one() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), Vec::new());
        rig.handle(Command::Uncertain(txid(1)));
        let old = rig
            .actor
            .jobs
            .join_next_with_id()
            .await
            .expect("old lookup");
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Uncertain(txid(1)));
        let newer = rig
            .actor
            .jobs
            .join_next_with_id()
            .await
            .expect("newer lookup");
        rig.actor.joined(newer);

        rig.actor.joined(old);

        assert!(
            rig.actor.uncertain.contains_key(&txid(1)),
            "the window stays"
        );
    }

    /// A lookup from before a switch-off is obsolete after it.
    #[tokio::test]
    async fn should_drop_a_lookup_from_before_a_switch_off() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Uncertain(txid(1)));
        let old = rig.actor.jobs.join_next_with_id().await.expect("lookup");

        rig.handle(Command::SetEnabled(false));
        rig.handle(Command::SetEnabled(true));
        rig.actor.joined(old);
        rig.settle().await;

        assert!(
            probe.probed().is_empty(),
            "no forced pass from the old report"
        );
    }

    /// Records a peer can feed the wallet: unsettled "incoming" parents,
    /// each with a child that spends the wallet's coins as matched — every
    /// parent a root to probe.
    fn peer_injected(parents: std::ops::Range<u8>) -> Vec<OutgoingView> {
        parents
            .flat_map(|parent| {
                [
                    incoming(parent, &[outpoint(parent.wrapping_add(100), 0)]),
                    send(parent.wrapping_add(60), &[outpoint(parent, 0)]),
                ]
            })
            .collect()
    }

    /// Reported sends' roots go first, in turn among themselves, and leave
    /// half of the slots to the rest: a peer whose outputs a reported send
    /// spends makes its parents such roots, and they must not fill every pass.
    #[test]
    fn should_share_a_pass_between_reported_and_other_roots() {
        let views = peer_injected(10..30);
        // Ten reported sends, each the child of one peer parent (10..20).
        let reported: HashSet<Txid> = (70..80).map(txid).collect();
        let reported_roots: HashSet<Txid> = (10..20).map(txid).collect();
        let mut state = ResolverState::default();
        let mut passes = Vec::new();
        for height in 100..103 {
            let start = begin_pass(
                &mut state,
                wallet(),
                height,
                &views,
                &HashSet::new(),
                &reported,
                Some(height),
            );
            let due: Vec<Txid> = start.due.iter().map(|root| root.txid).collect();
            for root in &due {
                mark_sent(&mut state, wallet(), *root, height, Some(height));
            }
            passes.push(due);
        }

        for due in &passes {
            assert_eq!(due.len(), MAX_ROOTS_PER_PASS);
            let first = due.iter().filter(|t| reported_roots.contains(*t)).count();
            assert_eq!(first, MAX_ROOTS_PER_PASS / 2, "half of the slots each");
            assert!(
                due[..first].iter().all(|t| reported_roots.contains(t)),
                "reported roots go first"
            );
        }
        let probed: HashSet<Txid> = passes.iter().flatten().copied().collect();
        assert!(
            reported_roots.iter().all(|root| probed.contains(root)),
            "every reported root came round in turn"
        );
    }

    /// Peer-injected records make many roots: a pass probes at most
    /// `MAX_ROOTS_PER_PASS` of them, the ones probed longest ago first, so
    /// each comes round in turn at the following heights.
    #[tokio::test]
    async fn should_bound_the_roots_a_pass_probes() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), peer_injected(10..40)); // 30 roots

        rig.follow(wallet(), 100).await;
        assert_eq!(probe.probed().len(), MAX_ROOTS_PER_PASS);

        rig.height(wallet(), 102).await;
        let probed = probe.probed();
        assert_eq!(probed.len(), 2 * MAX_ROOTS_PER_PASS);
        let distinct: HashSet<Txid> = probed.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            2 * MAX_ROOTS_PER_PASS,
            "the next ones in turn"
        );
    }

    /// A genuine send reported `Uncertain` while a pass works through
    /// peer-injected roots does not wait behind them: the run gives way
    /// after its probe in flight, and the send is probed next.
    #[tokio::test]
    async fn should_probe_a_genuine_uncertain_send_before_a_backlog() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), peer_injected(10..40));
        rig.follow(wallet(), 100).await;
        let before = probe.probed().len();
        // The genuine send, broadcast now — sorting after the backlog, so
        // only the yield can bring it forward.
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // a scheduled probe is out

        rig.handle(Command::Uncertain(txid(200)));
        while !probe.probed().contains(&txid(200)) {
            let job = rig.actor.jobs.join_next_with_id().await.expect("job");
            rig.actor.joined(job);
        }

        let since = &probe.probed()[before..];
        let genuine_at = since
            .iter()
            .position(|probed| *probed == txid(200))
            .expect("probed");
        assert!(
            genuine_at <= 2,
            "only the probe in flight (and at most one sent before the report \
             was routed) goes first: {since:?}"
        );
        rig.settle().await;
        // Two passes at that height — the one that gave way, and the forced
        // one that takes the roots it dropped — each bounded by the cap.
        assert!(
            probe.probed().len() - before <= 2 * MAX_ROOTS_PER_PASS + 1,
            "bounded work at that height"
        );
    }

    /// A peer feeding fresh roots every block cannot starve older ones: the
    /// queue serves roots in the order they were first seen.
    #[tokio::test]
    async fn should_not_let_fresh_roots_starve_older_ones() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..30); // 20 roots, seen at 101
        rig.views(wallet(), views.clone());
        rig.follow(wallet(), 100).await;

        for (height, fresh) in [(102, 30..38), (103, 38..46), (104, 46..54)] {
            views.extend(peer_injected(fresh)); // 8 fresh roots each block
            rig.views(wallet(), views.clone());
            rig.height(wallet(), height).await;
        }

        let probed: HashSet<Txid> = probe.probed().into_iter().collect();
        assert!(
            (10..30).all(|n| probed.contains(&txid(n))),
            "every older root came round before the fresh ones"
        );
    }

    /// A root waiting in a backlog keeps its every-block window: it starts at
    /// its first probe, not when the root was first seen.
    #[tokio::test]
    async fn should_start_a_window_at_the_first_probe_not_while_waiting() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), peer_injected(10..30)); // 20 roots
        rig.follow(wallet(), 100).await; // 8 probed at 101

        let waiting = &rig.actor.state.schedules[&(wallet(), txid(29))];
        assert_eq!(waiting.window_start, None);
        assert_eq!(waiting.seen_at, Some(101));
        rig.height(wallet(), 102).await;
        rig.height(wallet(), 103).await; // 29 is in the third batch

        assert_eq!(
            rig.actor.state.schedules[&(wallet(), txid(29))].window_start,
            Some(103)
        );
    }

    /// A forced request routed while the run is still reading makes it give
    /// way as soon as the read is in.
    #[tokio::test]
    async fn should_give_way_to_a_forced_request_routed_during_the_read() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..30);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.follow(wallet(), 100).await;
        let before = probe.probed().len();
        rig.source.pause(true);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.actor.handle(Command::Uncertain(txid(200)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed); // routed while the read waits
        rig.source.pause(false);

        rig.settle().await;

        let since = &probe.probed()[before..];
        assert_eq!(since.first(), Some(&txid(200)), "{since:?}");
    }

    /// A forced root no node answered stays ahead of the cap at its retry,
    /// whatever the backlog.
    #[tokio::test(start_paused = true)]
    async fn should_retry_a_forced_root_ahead_of_a_backlog() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(200), unreachable_verdict())]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        rig.handle(Command::Uncertain(txid(200)));

        rig.settle().await; // the anchored pass with the forced root, then retries

        let genuine = probe
            .probed()
            .iter()
            .filter(|probed| **probed == txid(200))
            .count();
        assert!(
            genuine > MAX_NO_ANSWER_RETRIES as usize,
            "its probe and every retry, whatever the backlog: {genuine}"
        );
    }

    /// A genuine send reported `Uncertain` keeps its place ahead of a
    /// peer-fed backlog after its first probe: it is probed at every block
    /// of its window, not once per trip round the queue.
    #[tokio::test]
    async fn should_keep_an_uncertain_send_ahead_of_a_backlog_after_its_first_probe() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(200))).await;
        let after_report = probe.probed().iter().filter(|p| **p == txid(200)).count();

        for height in 102..=104 {
            rig.height(wallet(), height).await;
        }

        let probed = probe.probed().iter().filter(|p| **p == txid(200)).count();
        assert_eq!(probed, after_report + 3, "once at each block");
    }

    /// While no node answers, a retry probes only the roots that went
    /// unanswered: the work at one height does not grow with each retry.
    #[tokio::test(start_paused = true)]
    async fn should_not_widen_a_retry_with_more_scheduled_roots() {
        let probe = Arc::new(SequenceProbe::new(&vec![unreachable_verdict(); 200]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), peer_injected(10..40));

        rig.follow(wallet(), 100).await;

        assert_eq!(
            probe.calls(),
            MAX_ROOTS_PER_PASS * (1 + MAX_NO_ANSWER_RETRIES as usize)
        );
    }

    /// Roots a run planned but dropped when it gave way are not lost: the
    /// forced pass, following the tip at the same height, takes them right
    /// after the forced root.
    #[tokio::test]
    async fn should_take_the_roots_a_run_dropped_for_a_forced_request_after_it() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // the first of 8 scheduled roots is out
        let planned: Vec<Txid> = rig.actor.wallets[&wallet()]
            .run
            .as_ref()
            .and_then(|run| run.probing.as_ref())
            .map(|probing| probing.due.iter().map(|root| root.txid).collect())
            .expect("probing");
        let before = probe.probed().len();

        rig.send(Command::Uncertain(txid(200))).await;

        let since = probe.probed()[before..].to_vec();
        let genuine = since
            .iter()
            .position(|probed| *probed == txid(200))
            .expect("probed");
        // The probe in flight finishes and the next planned root goes out
        // before the report is routed; the rest follow the forced root.
        assert!(genuine <= 2, "{since:?}");
        assert!(planned.iter().all(|txid| since.contains(txid)), "{since:?}");
    }

    /// A genuine send keeps its place ahead of a backlog for as long as it
    /// stays unsettled — past its echo window, which ends after
    /// `UNCERTAIN_EXPIRY` blocks.
    #[tokio::test]
    async fn should_keep_an_uncertain_send_first_past_its_echo_window() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(200))).await;
        let last = 102 + UNCERTAIN_EXPIRY + SLOW_INTERVAL;
        for height in 102..=last {
            rig.height(wallet(), height).await;
        }
        assert!(rig.actor.uncertain.is_empty(), "the echo window is over");
        let before = probe.probed().iter().filter(|p| **p == txid(200)).count();

        for height in last + 1..=last + SLOW_INTERVAL {
            rig.height(wallet(), height).await;
        }

        let after = probe.probed().iter().filter(|p| **p == txid(200)).count();
        assert_eq!(after, before + 1, "probed when due, ahead of the backlog");
    }

    /// A reported send stops being a priority once it leaves the wallet: the
    /// set does not grow for the session.
    #[tokio::test]
    async fn should_forget_a_reported_send_that_left_the_wallet() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..12);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(200))).await;
        rig.height(wallet(), 101).await;
        assert!(rig.actor.reported.contains_key(&txid(200)));

        rig.views(wallet(), peer_injected(10..12));
        rig.height(wallet(), 102).await;

        assert!(rig.actor.reported.is_empty(), "left with the send");
    }

    /// A repeated report of a send the run already forces changes nothing:
    /// the run keeps its scheduled roots.
    #[tokio::test]
    async fn should_not_give_way_to_a_report_the_run_already_carries() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        let mut views = peer_injected(10..40);
        views.push(send(200, &[outpoint(250, 0)]));
        rig.views(wallet(), views);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 100,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read);
        let run = rig
            .actor
            .wallets
            .get_mut(&wallet())
            .and_then(|e| e.run.as_mut())
            .expect("run");
        run.forced.insert(txid(200)); // the run carries this report already
        let planned = run.probing.as_ref().map(|p| p.due.len()).expect("probing");

        rig.handle(Command::Uncertain(txid(200)));
        loop {
            let job = rig.actor.jobs.join_next_with_id().await.expect("lookup");
            let listed = matches!(&job, Ok((_, JobDone::Listed { .. })));
            rig.actor.joined(job);
            if listed {
                break;
            }
        }

        let due = rig.actor.wallets[&wallet()]
            .run
            .as_ref()
            .and_then(|run| run.probing.as_ref())
            .map(|p| p.due.len())
            .expect("probing");
        assert!(
            due + 1 >= planned,
            "kept its scheduled roots: {due} of {planned}"
        );
    }

    /// A refusal is an answer: no early retry.
    #[tokio::test(start_paused = true)]
    async fn should_not_retry_a_probe_a_node_answered() {
        let probe = Arc::new(SequenceProbe::new(&[unresolved(), unresolved()]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.follow(wallet(), 100).await;

        assert_eq!(probe.calls(), 1);
    }

    /// Turning probing off forgets every `Uncertain` send.
    #[tokio::test]
    async fn should_forget_uncertain_results_when_probing_goes_off() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Uncertain(txid(1))).await;
        assert!(rig.actor.uncertain.contains_key(&txid(1)));

        rig.send(Command::SetEnabled(false)).await;

        assert!(rig.actor.uncertain.is_empty());
    }

    /// dash-spv may still see a send it reported `Uncertain` echoed by a peer
    /// later: with no block, no InstantSend lock and no probe answer, that
    /// echo alone turns its verdict into Accepted.
    #[tokio::test]
    async fn should_accept_a_send_echoed_after_its_uncertain_result() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;
        rig.send(Command::Uncertain(txid(1))).await;
        assert_eq!(
            rig.sent(),
            vec![ResolverEvent::Verdict(txid(1), unresolved())]
        );

        rig.send(Command::Echoed(txid(1))).await;

        assert_eq!(
            rig.sent(),
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Accepted)]
        );
    }

    /// An echo adds nothing to a decided verdict: a send probed Mined stays
    /// Mined, not turned back into Accepted.
    #[tokio::test]
    async fn should_not_turn_a_mined_send_back_into_accepted_on_its_echo() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), mined())]));
        let mut rig = enabled(probe).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;
        rig.send(Command::Uncertain(txid(1))).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), mined())]);

        rig.send(Command::Echoed(txid(1))).await;

        assert!(rig.sent().is_empty());
        assert_eq!(
            rig.actor.state.published.get(&(wallet(), txid(1))),
            Some(&mined())
        );
    }

    /// An acceptance whose lookup was out when the send settled must not
    /// reach the host after the send's verdict was cleared.
    #[tokio::test]
    async fn should_drop_an_acceptance_that_lands_after_the_send_settled() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;
        rig.send(Command::Uncertain(txid(1))).await;
        rig.sent();

        // The echo's lookup is out, then the send settles before it lands.
        rig.handle(Command::Echoed(txid(1)));
        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        });
        rig.settle().await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
    }

    /// A late acceptance is news only to a wallet where the send is still an
    /// unsettled own send: one that only receives it hears nothing.
    #[tokio::test]
    async fn should_publish_a_late_acceptance_only_to_an_own_unsettled_send() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe).await;
        rig.views(wallet(), vec![incoming(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;
        rig.send(Command::Uncertain(txid(1))).await;
        rig.sent();

        rig.send(Command::Echoed(txid(1))).await;

        assert!(rig.sent().is_empty());
    }

    /// Every healthy send is echoed: only one reported `Uncertain` first gets
    /// a verdict from it.
    #[tokio::test]
    async fn should_publish_nothing_for_the_echo_of_a_send_never_uncertain() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;

        rig.send(Command::Echoed(txid(1))).await;

        assert!(rig.sent().is_empty());
    }

    /// With no root left to probe the wallet is idle: height advances do not
    /// walk its records again until an event that touched them.
    #[tokio::test]
    async fn should_go_idle_when_no_root_is_left_until_a_touch() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), Vec::new());

        rig.follow(wallet(), 100).await;
        assert!(rig.sent().is_empty());
        assert_eq!(rig.walks(), 1);

        rig.height(wallet(), 102).await;
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: false,
        })
        .await;
        rig.height(wallet(), 103).await;
        assert_eq!(
            rig.walks(),
            1,
            "idle: no walk, even after an untouching event"
        );

        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        rig.height(wallet(), 104).await;
        assert_eq!(rig.walks(), 2);
    }

    /// Something touched the wallet while its pass ran: the pass may have read
    /// it before, so it must not leave the wallet idle.
    #[tokio::test]
    async fn should_not_leave_idle_a_wallet_touched_during_its_pass() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.height(wallet(), 100).await;

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        rig.actor.handle(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        });
        rig.settle().await;
        rig.height(wallet(), 102).await;

        assert_eq!(rig.walks(), 2);
    }

    /// Off clears every verdict and wakes every wallet; back on, the next
    /// height re-publishes what was cleared.
    #[tokio::test]
    async fn should_republish_after_probing_goes_off_and_on() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), mined())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.send(Command::SetEnabled(false)).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);

        rig.send(Command::SetEnabled(true)).await;
        rig.height(wallet(), 102).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), mined())]);
        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// Probing turned off while a pass is out on the network: its result is
    /// dropped, nothing is published after the clears.
    #[tokio::test]
    async fn should_drop_a_pass_in_flight_when_probing_goes_off() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.height(wallet(), 100).await;

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        rig.actor.handle(Command::SetEnabled(false));
        rig.settle().await;

        assert!(rig.sent().is_empty());
        assert!(rig.actor.state.published.is_empty());
    }

    #[tokio::test]
    async fn should_clear_a_removed_wallets_verdicts_and_forget_it() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.send(Command::WalletRemoved(wallet())).await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        assert!(!rig.actor.wallets.contains_key(&wallet()));
    }

    /// An `Uncertain` result right after launch, before any height step
    /// reached the wallet, still gets its immediate probe.
    #[tokio::test]
    async fn should_route_an_uncertain_result_to_a_wallet_no_event_introduced() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// The fail-safe for a registered wallet read as gone (removed by an
    /// embedder through `wallet_manager_arc`, bypassing the notification): its
    /// verdicts are cleared and it goes idle rather than being re-read every
    /// block.
    #[tokio::test]
    async fn should_clear_the_verdicts_of_a_wallet_gone_without_a_removal() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.source.views.lock().expect("views").remove(&wallet());
        rig.height(wallet(), 102).await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        assert!(rig.actor.state.schedules.is_empty());
        assert_eq!(
            rig.actor.wallets[&wallet()].wait_until,
            Some(u32::MAX),
            "idle: not re-read on the next block"
        );
    }

    /// An `Uncertain` result forces its root even if a height pass sent it at
    /// this height already: one repeat probe is the accepted price of not
    /// tracking which report came first.
    #[tokio::test]
    async fn should_force_a_reported_root_even_if_probed_at_this_height() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;

        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// A run whose read finds nothing to do still drops a height queued
    /// during it that no root is due at.
    #[tokio::test]
    async fn should_drop_a_queued_height_when_a_forced_read_finds_nothing() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        // Idle with no verdicts out: nothing to probe, nothing to clear.
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        let walks = rig.walks();

        // An `Uncertain` for a txid the wallet no longer holds by read time.
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.actor.handle(Command::Uncertain(txid(1)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.views(wallet(), Vec::new());
        rig.actor.joined(listed);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.settle().await;

        assert_eq!(rig.walks(), walks, "idle: neither pass walked the records");
    }

    /// A wallet waiting for nothing (idle) is forced by an `Uncertain` result;
    /// a height that arrives while that pass runs is kept, so the new root
    /// gets its next-block probe.
    #[tokio::test]
    async fn should_keep_a_height_that_arrives_during_a_forced_pass() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(2), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.height(wallet(), 102).await;
        assert!(probe.probed().is_empty(), "idle: nothing to probe");
        rig.views(wallet(), vec![send(2, &[outpoint(91, 0)])]);

        rig.actor.handle(Command::Uncertain(txid(2)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        // The forced pass has read the wallet; its probe of root 2 is out.
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 103,
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(2), txid(2)]);
    }

    /// A run that ends early (a probe panicked) leaves the roots it never sent
    /// due at once, so the next pass probes them.
    #[test]
    fn should_keep_due_the_roots_a_run_never_sent() {
        let mut state = ResolverState::default();
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])];
        let start = begin_pass(
            &mut state,
            wallet(),
            100,
            &views,
            &HashSet::new(),
            &HashSet::new(),
            Some(100),
        );
        let planned: Vec<Txid> = start.due.iter().map(|root| root.txid).collect();
        assert_eq!(planned.len(), 2);

        // Only the first root goes out before the run ends.
        mark_sent(&mut state, wallet(), planned[0], 100, Some(100));
        let again = begin_pass(
            &mut state,
            wallet(),
            100,
            &views,
            &HashSet::new(),
            &HashSet::new(),
            Some(100),
        );

        assert_eq!(
            again.due.iter().map(|root| root.txid).collect::<Vec<_>>(),
            vec![planned[1]]
        );
    }

    /// A probe that never produced a verdict (its job panicked) is withdrawn:
    /// the root is due again at once, not after the slow interval.
    #[test]
    fn should_make_a_root_due_at_once_when_its_probe_is_withdrawn() {
        let mut state = ResolverState::default();
        let views = [send(1, &[outpoint(90, 0)])];
        begin_pass(
            &mut state,
            wallet(),
            100,
            &views,
            &HashSet::new(),
            &HashSet::new(),
            Some(100),
        );
        mark_sent(&mut state, wallet(), txid(1), 100, Some(100));
        assert!(!state.schedules[&(wallet(), txid(1))].is_due(100));

        withdraw_send(&mut state, wallet(), txid(1));

        assert!(state.schedules[&(wallet(), txid(1))].is_due(100));
    }

    /// A task that ends only when the test releases it: no stop can finish
    /// before then, whatever the executor's timing.
    fn held_task() -> (JoinHandle<()>, oneshot::Sender<()>) {
        let (release, released) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let _ = released.await;
        });
        (handle, release)
    }

    fn is_stopping(slot: &Mutex<ActorSlot>) -> bool {
        matches!(*slot.lock().expect("slot"), ActorSlot::Stopping(_))
    }

    fn is_taken(slot: &Mutex<ActorSlot>) -> bool {
        matches!(*slot.lock().expect("slot"), ActorSlot::Closed)
    }

    /// Poll `future` exactly once: its output if it is ready then.
    async fn poll_once<F: std::future::Future + Unpin>(future: &mut F) -> Option<F::Output> {
        tokio::select! {
            biased;
            output = future => Some(output),
            () = std::future::ready(()) => None,
        }
    }

    /// A safety bound for a stop the test expects to return: a regression
    /// that waits on the held task forever fails instead of hanging.
    async fn bounded<F: std::future::Future>(future: F) -> F::Output {
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("the stop returned within its bound")
    }

    /// Two stops at once: the second waits for the first and never reports
    /// the task gone while it still runs.
    #[tokio::test]
    async fn should_not_report_a_running_task_as_not_running_to_a_concurrent_stop() {
        let (stop, _stopped) = oneshot::channel();
        let (handle, release) = held_task();
        let slot = Mutex::new(ActorSlot::Running(handle, stop));
        let stopping = AsyncMutex::new(());

        let mut first = Box::pin(stop_slot(&slot, &stopping, Duration::from_secs(5)));
        let mut second = Box::pin(stop_slot(&slot, &stopping, Duration::from_secs(5)));
        assert!(
            poll_once(&mut first).await.is_none(),
            "waits on the held task"
        );
        assert!(
            poll_once(&mut second).await.is_none(),
            "waits for the first stop, not reporting the live task gone"
        );

        release.send(()).expect("task alive");
        assert_eq!(bounded(first).await, WorkerStatus::Ok);
        assert_eq!(bounded(second).await, WorkerStatus::NotRunning);
    }

    /// A stop that runs out of budget keeps the task without aborting it: a
    /// retried stop waits for it to end and reports it clean (an aborted task
    /// would join as stopped, not Ok).
    #[tokio::test]
    async fn should_report_a_task_that_outlived_a_stop_clean_on_retry() {
        let (stop, _stopped) = oneshot::channel();
        let (handle, release) = held_task();
        let slot = Mutex::new(ActorSlot::Running(handle, stop));
        let stopping = AsyncMutex::new(());

        // No budget at all: the held task cannot end within it.
        let first = bounded(stop_slot(&slot, &stopping, Duration::ZERO)).await;
        assert_eq!(first, WorkerStatus::Timeout, "the task is held");
        assert!(is_stopping(&slot), "put back as Stopping");

        release.send(()).expect("task alive");
        let second = bounded(stop_slot(&slot, &stopping, Duration::from_secs(5))).await;
        let third = bounded(stop_slot(&slot, &stopping, Duration::from_secs(5))).await;

        assert_eq!(second, WorkerStatus::Ok);
        assert_eq!(third, WorkerStatus::NotRunning);
    }

    /// A stop whose caller gives up mid-wait (its future dropped) must not take
    /// the task with it: a retried stop still finds it and waits for it.
    #[tokio::test]
    async fn should_keep_the_task_joinable_when_a_stop_is_cancelled() {
        let (stop, _stopped) = oneshot::channel();
        let (handle, release) = held_task();
        let slot = Mutex::new(ActorSlot::Running(handle, stop));
        let stopping = AsyncMutex::new(());

        // Poll the stop once — it takes the task and waits on it — then drop
        // it mid-wait.
        let mut cancelled = Box::pin(stop_slot(&slot, &stopping, Duration::from_secs(5)));
        assert!(
            poll_once(&mut cancelled).await.is_none(),
            "the task is held"
        );
        assert!(
            is_taken(&slot),
            "the stop took the task and is waiting on it"
        );
        drop(cancelled);
        assert!(is_stopping(&slot), "the cancelled stop put the task back");

        release.send(()).expect("task alive");
        let retried = bounded(stop_slot(&slot, &stopping, Duration::from_secs(5))).await;

        assert_eq!(retried, WorkerStatus::Ok, "the retry joined the live task");
    }

    /// An `Uncertain` result right after launch runs a pass at the stale stored
    /// tip; once the wallet follows the tip again, the root still gets its
    /// every-block window.
    #[tokio::test]
    async fn should_give_a_root_forced_before_the_first_height_its_every_block_window() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Uncertain(txid(1))).await;
        assert_eq!(probe.probed().len(), 1, "forced at the stored tip (0 here)");

        rig.follow(wallet(), 500).await;
        for height in 502..=505 {
            rig.height(wallet(), height).await;
        }

        assert_eq!(probe.probed().len(), 6, "every block from 501 to 505");
    }

    /// A return to following the tip does not restart the every-block window
    /// of a root already past it: exactly one probe, when the slow interval
    /// is up.
    #[tokio::test]
    async fn should_not_restart_the_window_of_a_root_past_it_after_a_catch_up() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        // Window 101..=124, then 134.
        let past = 101 + EVERY_BLOCK_WINDOW + SLOW_INTERVAL;
        for height in 102..=past {
            rig.height(wallet(), height).await;
        }
        assert_eq!(probe.probed().len(), EVERY_BLOCK_WINDOW as usize + 1);

        // Away for a while (a catch-up step), then following again.
        rig.height(wallet(), past + 10).await;
        for height in past + 11..=past + 14 {
            rig.height(wallet(), height).await;
        }

        assert_eq!(
            probe.probed().len(),
            EVERY_BLOCK_WINDOW as usize + 2,
            "one probe at the first following height past the slow interval"
        );
    }

    /// A forced read whose result is joined only after the wallet follows the
    /// tip again does not start the window at the stale stored height: the
    /// next height-driven pass starts it at the tip, for exactly
    /// EVERY_BLOCK_WINDOW blocks.
    #[tokio::test]
    async fn should_anchor_a_root_read_before_the_wallet_followed_at_the_tip() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.actor.handle(Command::Uncertain(txid(1)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        for height in [500, 501] {
            rig.handle(Command::Height {
                wallet_id: wallet(),
                height,
            });
        }
        rig.settle().await;
        for height in 502..=530 {
            rig.height(wallet(), height).await;
        }

        // The forced probe, then 501..=524 every block, nothing up to 530.
        assert_eq!(probe.probed().len(), 1 + EVERY_BLOCK_WINDOW as usize);
    }

    /// Back from the background with no height event yet, a send reported
    /// `Uncertain` is probed at once but its window waits for the wallet to
    /// follow the tip again — it does not start at the height from before.
    #[tokio::test]
    async fn should_not_start_a_window_at_the_height_from_before_the_background() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.send(Command::Uncertain(txid(1))).await;
        assert_eq!(probe.probed().len(), 1, "forced at once");
        // 30 blocks later: a catch-up step, then following.
        rig.height(wallet(), 131).await;
        for height in 132..=136 {
            rig.height(wallet(), height).await;
        }

        assert_eq!(probe.probed().len(), 1 + 5, "every block from 132");
    }

    /// A height pass queued before the app went to the background, whose read
    /// is handled only after the wallet follows the tip again, does not start
    /// a window at its old height; the next pass starts it at the tip.
    #[tokio::test]
    async fn should_not_anchor_a_pass_overtaken_by_a_catch_up() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;

        for height in [102, 132, 133] {
            rig.handle(Command::Height {
                wallet_id: wallet(),
                height,
            });
        }
        rig.settle().await;
        for height in 134..=137 {
            rig.height(wallet(), height).await;
        }

        // Nothing at 102 (stale: catch-up skips it), then every block from 133.
        assert_eq!(probe.probed().len(), 5);
    }

    /// A height pass whose read finds the wallet far ahead of the height it was
    /// queued at — a catch-up whose height step has not arrived yet — is
    /// stale: it neither probes by schedule nor starts a window.
    #[tokio::test]
    async fn should_treat_a_pass_whose_read_is_far_ahead_as_catch_up() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 1_101);

        rig.height(wallet(), 102).await;
        assert!(
            probe.probed().is_empty(),
            "catch-up: no probe during the scan"
        );
        rig.height(wallet(), 1_101).await;
        for height in 1_102..=1_105 {
            rig.height(wallet(), height).await;
        }

        assert_eq!(probe.probed().len(), 4, "every block from 1102");
    }

    /// A pass without an anchor — here a forced one during a catch-up —
    /// probes only the root the `Uncertain` result forced, not every root
    /// due by schedule.
    #[tokio::test]
    async fn should_probe_only_forced_roots_in_a_pass_without_an_anchor() {
        let probe = Arc::new(ScriptedProbe::new(&[
            (txid(1), ProbeVerdict::Accepted),
            (txid(2), ProbeVerdict::Accepted),
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.follow(wallet(), 100).await;
        assert_eq!(probe.probed().len(), 2);

        rig.height(wallet(), 140).await; // catch-up
        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(probe.probed()[2..], [txid(1)]);
    }

    /// A pass whose read finds the wallet behind the height it was queued at —
    /// a rewind for a rescan — is confined to forced roots too.
    #[tokio::test]
    async fn should_treat_a_pass_whose_read_is_behind_as_catch_up() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        let before = probe.probed().len();

        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 50); // rewound
        rig.settle().await;

        assert_eq!(probe.probed().len(), before);
    }

    /// A catch-up step stops a pass that is still reading and carries nothing
    /// forced: its walk over the records could only be thrown away.
    #[tokio::test]
    async fn should_stop_a_reading_pass_with_nothing_forced_on_a_catch_up() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        // No verdict published: nothing for the pass to clear.
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        let walks = rig.walks();

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 140,
        });
        rig.settle().await;

        assert_eq!(rig.walks(), walks);
        assert!(rig.actor.wallets[&wallet()].run.is_none());
    }

    /// A catch-up takes the anchor from a queued pass that carries a forced
    /// txid; the pass still runs, confined to what was forced, and — with the
    /// txid no longer held — without walking the records.
    #[tokio::test]
    async fn should_run_a_forced_pass_overtaken_by_a_catch_up_without_an_anchor() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        // No verdict published: nothing for the pass to clear.
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(9, &[outpoint(99, 0)])],
        );
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        let walks = rig.walks();

        // A height pass holds in its read; an `Uncertain` result and the next
        // height queue behind it.
        rig.source.pause(true);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.actor.handle(Command::Uncertain(txid(9)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 103,
        });
        // The forced txid leaves the wallet; then a catch-up.
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 140,
        });

        let run = rig.actor.wallets[&wallet()]
            .run
            .as_ref()
            .expect("the forced pass");
        assert_eq!(run.anchor_at, None);
        assert!(!run.walk.load(Ordering::SeqCst));
        rig.source.pause(false);
        rig.settle().await;
        assert_eq!(rig.walks(), walks, "txid 9 is not held: no walk");
    }

    /// A wallet event that settles a send clears its verdict at once — during
    /// a catch-up too — without a pass or a walk over the records, and only
    /// that send's.
    #[tokio::test]
    async fn should_clear_a_verdict_as_soon_as_an_event_settles_its_send() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[
            (txid(1), mined()),
            (txid(2), unresolved()),
        ])))
        .await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.follow(wallet(), 100).await;
        rig.sent();
        rig.height(wallet(), 140).await; // catch-up
        let walks = rig.walks();

        rig.send(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        })
        .await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        assert_eq!(rig.walks(), walks);
        assert!(rig.actor.state.published.contains_key(&(wallet(), txid(2))));
    }

    /// Which transactions a wallet event reports settled or gone.
    #[test]
    fn should_read_settled_and_gone_txids_from_wallet_events() {
        let lock = WalletEvent::ChainLockProcessed {
            wallet_id: wallet(),
            chain_lock: chain_lock(),
            locked_transactions: BTreeMap::from([(
                AccountType::CoinJoin { index: 0 },
                vec![txid(1)],
            )]),
        };
        let swept = WalletEvent::TransactionsSwept {
            wallet_id: wallet(),
            txids: vec![txid(2)],
            superseded_by: txid(3),
            winner_mined_height: None,
            released_outpoints: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        };
        let height = WalletEvent::SyncHeightAdvanced {
            wallet_id: wallet(),
            height: 100,
        };

        assert!(
            settled_or_gone(&lock).is_empty(),
            "a chain lock settles nothing new: its block did"
        );
        assert_eq!(settled_or_gone(&swept), HashSet::from([txid(2)]));
        assert!(settled_or_gone(&height).is_empty());
    }

    /// A step back (a rewind for a rescan) lowers a queued forced pass to the
    /// replayed height: a probe marked above it would keep the root from
    /// being due until the rescan passed that height again.
    #[tokio::test]
    async fn should_lower_a_queued_pass_to_the_height_of_a_rewind() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.source.pause(true);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.actor.handle(Command::Uncertain(txid(1)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 103,
        });

        // The rewind: synced height goes back, then the step is reported.
        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 50);
        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 50,
        });
        rig.source.pause(false);
        rig.settle().await;

        // The forced probe went out at the replayed height, not at 103.
        assert_eq!(
            rig.actor.state.schedules[&(wallet(), txid(1))].last_probe,
            Some(50)
        );
    }

    /// A send settled while its probe was out: the event clears it, and the
    /// probe's late answer publishes nothing for it again.
    #[tokio::test]
    async fn should_not_republish_a_send_settled_while_its_probe_was_out() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // its probe is out now
        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        });
        rig.settle().await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
    }

    /// A step back (a rewind for a rescan) starts the wallet over: every
    /// verdict cleared, every schedule gone, so every root is due at once.
    #[tokio::test]
    async fn should_start_over_on_a_rewind() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])],
        );
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.height(wallet(), 50).await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        assert!(rig.actor.state.schedules.is_empty());
        assert_eq!(rig.actor.wallets[&wallet()].wait_until, None);
    }

    fn record(n: u8, context: TransactionContext) -> TransactionRecord {
        TransactionRecord::new(
            tx(n, &[outpoint(90, 0)]),
            AccountType::CoinJoin { index: 0 },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn in_block() -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(100, BlockHash::all_zeros(), 0))
    }

    /// Block and new-transaction events name only the records that are
    /// settled; a mempool record is not.
    #[test]
    fn should_read_settled_records_from_block_and_transaction_events() {
        let block = WalletEvent::BlockProcessed {
            wallet_id: wallet(),
            height: 100,
            chain_lock: None,
            inserted: vec![
                record(1, in_block()),
                record(2, TransactionContext::Mempool),
            ],
            updated: vec![record(3, in_block())],
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let detected = |context| WalletEvent::TransactionDetected {
            wallet_id: wallet(),
            record: Box::new(record(4, context)),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let txid_of = |n: u8| tx(n, &[outpoint(90, 0)]).txid();

        assert_eq!(
            settled_or_gone(&block),
            HashSet::from([txid_of(1), txid_of(3)])
        );
        assert_eq!(
            settled_or_gone(&detected(in_block())),
            HashSet::from([txid_of(4)])
        );
        assert!(settled_or_gone(&detected(TransactionContext::Mempool)).is_empty());
    }

    /// A reorg brings a settled send back unsettled: a run that starts after
    /// the settle event reads it as current, and the send is probed again.
    #[tokio::test]
    async fn should_probe_a_send_again_after_a_reorg_unsettles_it() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        })
        .await;
        rig.sent();

        // Reorged back to the mempool: the next read lists it again.
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        rig.height(wallet(), 102).await;

        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
        assert_eq!(
            rig.sent(),
            vec![ResolverEvent::Verdict(txid(1), unresolved())]
        );
    }

    /// A rewind stops a forced pass still reading; what was forced runs in
    /// the pass after it, at the rewound height.
    #[tokio::test]
    async fn should_carry_a_forced_probe_over_a_rewind() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        let before = probe.probed().len();
        rig.source.pause(true);
        rig.actor.handle(Command::Uncertain(txid(1)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed); // forced pass reading, queued at 101

        rig.source
            .synced
            .lock()
            .expect("synced")
            .insert(wallet(), 50);
        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 50,
        });
        rig.source.pause(false);
        rig.settle().await;

        assert_eq!(
            probe.probed()[before..],
            [txid(1)],
            "probed once, after the rewind"
        );
        assert_eq!(
            rig.actor.state.schedules[&(wallet(), txid(1))].last_probe,
            Some(50)
        );
    }

    /// A rewind stops the pass probing: the probe out is dropped and the
    /// roots still due are not sent.
    #[tokio::test]
    async fn should_stop_probing_on_a_rewind() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1's probe is out, root 2 due next

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 50,
        });
        rig.settle().await;

        assert!(probe.probed().is_empty());
        assert!(rig.actor.wallets[&wallet()].run.is_none());
        assert!(rig.actor.state.schedules.is_empty());
    }

    /// A send settled while the pass is still reading is left out of the read
    /// that may still list it: not probed, nothing published.
    #[tokio::test]
    async fn should_leave_a_send_settled_during_the_read_out_of_the_pass() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(2), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(wallet(), vec![send(2, &[outpoint(91, 0)])]);
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;

        rig.source.pause(true);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(2)]),
        });
        rig.source.pause(false);
        rig.settle().await;

        assert!(probe.probed().is_empty());
        assert!(rig.sent().is_empty());
    }

    /// Two roots due; while the first is out, the second settles: it is not
    /// sent to the network.
    #[tokio::test]
    async fn should_not_probe_a_queued_root_settled_while_probing() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1 out, root 2 queued

        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(2)]),
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// The root whose probe is out settles: that probe is stopped and the
    /// pass goes on with the next root.
    #[tokio::test]
    async fn should_stop_the_probe_of_a_root_that_settles_while_out() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1's probe spawned, not yet run

        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(2)]);
    }

    /// The first root's probe finishes Mined, then the root settles before
    /// the result is handled: the leftover answer is not recorded against
    /// the next root in flight.
    #[tokio::test]
    async fn should_not_record_a_stopped_probe_answer_against_the_next_root() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );
        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1's probe spawned

        // Its finished answer, held back: completed, not yet handled.
        let probed = rig.actor.jobs.join_next_with_id().await.expect("probe");
        assert!(matches!(
            &probed,
            Ok((_, JobDone::Probed { root, outcome, .. }))
                if *root == txid(1) && outcome.verdict == ProbeVerdict::Mined
        ));

        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        });
        rig.actor.joined(probed);
        rig.settle().await;

        assert!(!rig
            .sent()
            .contains(&ResolverEvent::Verdict(txid(2), mined())));
        assert_ne!(
            rig.actor.state.published.get(&(wallet(), txid(2))),
            Some(&mined())
        );
    }

    /// A settle unrelated to a forced send's chain does not probe that send
    /// again.
    #[tokio::test]
    async fn should_not_requeue_a_forced_send_for_an_unrelated_settle() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(
            wallet(),
            vec![
                incoming(5, &[outpoint(80, 0)]),
                send(6, &[outpoint(5, 0)]),
                incoming(8, &[outpoint(88, 0)]),
            ],
        );
        rig.actor.handle(Command::Uncertain(txid(6)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 5 out

        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(8)]),
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(5)]);
    }

    /// A height step queued behind a wallet's removal does not leave an entry
    /// behind.
    #[tokio::test]
    async fn should_not_keep_an_entry_for_a_height_after_removal() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.source.views.lock().expect("views").remove(&wallet());

        rig.send(Command::WalletRemoved(wallet())).await;
        // Straight to the actor: a step queued behind the removal.
        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.actor.handle(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        });
        rig.settle().await;

        assert!(!rig.actor.wallets.contains_key(&wallet()));
    }

    /// A forced send built on two unsettled roots: one of them settling
    /// leaves the other as its only root — no new root, no requeue.
    #[tokio::test]
    async fn should_not_requeue_when_a_settle_leaves_no_new_root() {
        let probe = Arc::new(ScriptedProbe::new(&[]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), Vec::new());
        rig.follow(wallet(), 100).await;
        rig.views(
            wallet(),
            vec![
                incoming(1, &[outpoint(80, 0)]),
                incoming(2, &[outpoint(81, 0)]),
                send(3, &[outpoint(1, 0), outpoint(2, 0)]),
            ],
        );
        rig.actor.handle(Command::Uncertain(txid(3)));
        let listed = rig.actor.jobs.join_next_with_id().await.expect("lookup");
        rig.actor.joined(listed);
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1 out, root 2 queued

        rig.handle(Command::Settled {
            wallet_id: wallet(),
            txids: HashSet::from([txid(1)]),
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(2)], "root 1 stopped, root 2 once");
    }

    /// An `Uncertain` lookup that lands after its wallet was removed does not
    /// bring the wallet's entry back.
    #[tokio::test]
    async fn should_not_bring_back_a_removed_wallet_from_a_lookup() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.actor.handle(Command::Uncertain(txid(1)));
        rig.actor.handle(Command::WalletRemoved(wallet()));

        rig.settle().await;

        assert!(!rig.actor.wallets.contains_key(&wallet()));
    }

    /// A same-id wallet registered again starts fresh: a stale idle wait and
    /// stale verdicts from before do not carry over.
    #[tokio::test]
    async fn should_start_a_re_registered_wallet_fresh() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), mined())]);
        assert!(rig.actor.wallets[&wallet()].wait_until.is_some());

        rig.send(Command::WalletAdded(wallet())).await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        let entry = &rig.actor.wallets[&wallet()];
        assert_eq!(entry.wait_until, None);
        assert_eq!(entry.height, None);
    }

    /// An echoed send that leaves the wallet is forgotten by the next pass,
    /// its echo's verdict cleared.
    #[tokio::test]
    async fn should_forget_the_echo_of_a_send_that_left() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.send(Command::Uncertain(txid(1))).await;
        rig.send(Command::Echoed(txid(1))).await;
        assert_eq!(
            rig.actor.state.published.get(&(wallet(), txid(1))),
            Some(&ProbeVerdict::Accepted)
        );

        rig.sent();

        rig.views(wallet(), Vec::new());
        rig.send(Command::Seen {
            wallet_id: wallet(),
            touched: true,
        })
        .await;
        rig.height(wallet(), 102).await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
        assert!(rig.actor.state.published.is_empty());
    }

    /// A probe sent before a rewind would answer Mined: the answer belongs to
    /// the state the rewind dropped — the rewind stops it, and nothing is told
    /// or recorded.
    #[tokio::test]
    async fn should_neither_tell_nor_record_a_pre_rewind_answer() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), mined())]))).await;
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])],
        );
        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read); // root 1's probe is out

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 50,
        });
        rig.settle().await;

        assert!(rig.sent().is_empty());
        assert!(rig.actor.state.published.is_empty());
    }

    /// A probe that panics on its `nth` call and accepts otherwise.
    struct PanicOnCall {
        nth: usize,
        calls: StdMutex<usize>,
    }

    #[async_trait]
    impl AcceptanceProbe for PanicOnCall {
        async fn probe(&self, _transaction: &Transaction) -> ProbeOutcome {
            let call = {
                let mut calls = self.calls.lock().expect("calls");
                *calls += 1;
                *calls
            };
            if call == self.nth {
                panic!("probe bug");
            }
            ProbeOutcome::answered(ProbeVerdict::Accepted)
        }
    }

    /// A root in its slow interval whose probe panicked is probed again on
    /// the next block, not SLOW_INTERVAL blocks later.
    #[tokio::test]
    async fn should_probe_again_on_the_next_block_after_a_panicked_slow_probe() {
        let first = 101;
        let slow = first + EVERY_BLOCK_WINDOW - 1 + SLOW_INTERVAL;
        let probe = Arc::new(PanicOnCall {
            nth: EVERY_BLOCK_WINDOW as usize + 1,
            calls: StdMutex::new(0),
        });
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), first - 1).await;
        for height in first + 1..=slow {
            rig.height(wallet(), height).await;
        }
        assert_eq!(
            *probe.calls.lock().expect("calls"),
            EVERY_BLOCK_WINDOW as usize + 1,
            "the slow probe went out and panicked"
        );

        rig.height(wallet(), slow + 1).await;

        assert_eq!(
            *probe.calls.lock().expect("calls"),
            EVERY_BLOCK_WINDOW as usize + 2
        );
    }

    /// A probe that panics.
    struct PanickingProbe;

    #[async_trait]
    impl AcceptanceProbe for PanickingProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeOutcome {
            panic!("probe bug");
        }
    }

    /// A job that panics ends its run; the wallet's next pass still starts.
    #[tokio::test]
    async fn should_not_leave_a_wallet_stuck_behind_a_panicked_job() {
        let mut rig = enabled(Arc::new(PanickingProbe)).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.follow(wallet(), 100).await;
        rig.height(wallet(), 102).await;

        assert_eq!(rig.walks(), 2);
        assert!(rig.actor.wallets[&wallet()].run.is_none());
    }

    /// After the every-block window a root is due only every SLOW_INTERVAL
    /// blocks: the heights in between do not walk the wallet's records.
    #[tokio::test]
    async fn should_not_read_the_wallet_on_heights_where_no_root_is_due() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(
            txid(1),
            ProbeVerdict::Accepted,
        )])))
        .await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        for height in 102..=101 + EVERY_BLOCK_WINDOW {
            rig.height(wallet(), height).await;
        }
        let walks = rig.walks();

        for height in 102 + EVERY_BLOCK_WINDOW..=101 + EVERY_BLOCK_WINDOW + SLOW_INTERVAL {
            rig.height(wallet(), height).await;
        }

        assert_eq!(rig.walks(), walks + 1, "one walk in SLOW_INTERVAL blocks");
    }

    /// A probe that never answers.
    /// A probe that never answers, and reports when it started and when its
    /// future was torn down.
    #[derive(Default)]
    struct HangingProbe {
        started: tokio::sync::Notify,
        dropped: Arc<std::sync::atomic::AtomicBool>,
    }

    /// Sets its flag when the probe future holding it is dropped.
    struct DropFlag(Arc<std::sync::atomic::AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl AcceptanceProbe for HangingProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeOutcome {
            let _torn_down = DropFlag(Arc::clone(&self.dropped));
            self.started.notify_one();
            std::future::pending().await
        }
    }

    /// Stopping the task aborts and awaits every job it started: by the time
    /// the task has ended, the hanging probe's future is gone too.
    #[tokio::test]
    async fn should_end_with_every_job_when_stopped() {
        let probe = Arc::new(HangingProbe::default());
        let mut rig = Rig::new(probe.clone());
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        let (commands, receiver) = mpsc::unbounded_channel();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(rig.actor.run(receiver, stopped));
        for command in [Command::SetEnabled(true), Command::Uncertain(txid(1))] {
            commands.send(command).expect("send");
        }
        timeout(Duration::from_secs(5), probe.started.notified())
            .await
            .expect("the probe started");
        assert!(
            !probe.dropped.load(Ordering::SeqCst),
            "still running before the stop"
        );

        stop.send(()).expect("stop");

        timeout(Duration::from_secs(5), task)
            .await
            .expect("stopped in time")
            .expect("no panic");
        assert!(
            probe.dropped.load(Ordering::SeqCst),
            "the probe was torn down before the task ended"
        );
    }

    /// A probe that records what the host had been sent by the time it ran.
    struct SnapshotProbe {
        sink: Arc<CollectSink>,
        seen: StdMutex<Vec<ResolverEvent>>,
    }

    #[async_trait]
    impl AcceptanceProbe for SnapshotProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeOutcome {
            *self.seen.lock().expect("seen") = self.sink.0.lock().expect("sink").clone();
            ProbeOutcome::answered(ProbeVerdict::Accepted)
        }
    }

    /// A clear reaches the host before the pass goes out to the network.
    #[tokio::test]
    async fn should_deliver_clears_before_probing() {
        let sink = Arc::new(CollectSink::default());
        let probe = Arc::new(SnapshotProbe {
            sink: Arc::clone(&sink),
            seen: StdMutex::new(Vec::new()),
        });
        let mut rig = Rig::with_sink(probe.clone(), sink);
        rig.send(Command::SetEnabled(true)).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.views(wallet(), vec![send(2, &[outpoint(91, 0)])]);
        rig.height(wallet(), 102).await;

        assert_eq!(
            *probe.seen.lock().expect("seen"),
            vec![ResolverEvent::Cleared(txid(1))]
        );
    }

    fn chain_lock() -> ChainLock {
        ChainLock {
            block_height: 100,
            block_hash: BlockHash::all_zeros(),
            signature: BLSSignature::from([0u8; 96]),
        }
    }

    /// Only a block that changed records (or a new, locked or swept
    /// transaction) may leave work; a chain lock — about one per block for
    /// every wallet — only promotes what its block reported.
    #[test]
    fn should_wake_only_on_events_that_touched_records() {
        let block = |inserted: Vec<TransactionRecord>| WalletEvent::BlockProcessed {
            wallet_id: wallet(),
            height: 100,
            chain_lock: None,
            inserted,
            updated: Vec::new(),
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let lock = |txids: Vec<Txid>| WalletEvent::ChainLockProcessed {
            wallet_id: wallet(),
            chain_lock: chain_lock(),
            locked_transactions: BTreeMap::from([(AccountType::CoinJoin { index: 0 }, txids)]),
        };

        assert!(!may_leave_work(&block(Vec::new())));
        assert!(!may_leave_work(&lock(Vec::new())));
        assert!(
            !may_leave_work(&lock(vec![txid(1)])),
            "a chain lock promotes what its block reported"
        );
        assert!(!may_leave_work(&WalletEvent::SyncHeightAdvanced {
            wallet_id: wallet(),
            height: 100,
        }));
    }

    #[test]
    fn should_act_on_forced_txids_only_when_held() {
        let held = |txid: &Txid| *txid == self::txid(1);
        let forced = HashSet::from([txid(1), txid(2)]);

        assert_eq!(
            forced_held(false, forced.clone(), held),
            Some(HashSet::from([txid(1)]))
        );
        assert_eq!(forced_held(false, HashSet::from([txid(2)]), held), None);
        assert_eq!(
            forced_held(true, HashSet::from([txid(2)]), held),
            Some(HashSet::new())
        );
    }
}
