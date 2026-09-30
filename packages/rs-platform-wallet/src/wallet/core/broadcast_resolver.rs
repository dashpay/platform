//! Automatically asking the network about sends whose broadcast outcome is
//! unknown.
//!
//! A send the SPV broadcaster could not confirm ends as `MaybeSent`
//! (`BroadcastResult::Uncertain`): the wallet keeps its record as an
//! unconfirmed spend and nothing ever tells the user whether the payment
//! landed. This module re-asks the network through
//! [`AcceptanceProbe`](crate::broadcast_probe::AcceptanceProbe) and publishes
//! the verdict; it **changes nothing** in the wallet. Cancelling a dead send is
//! a separate, user-driven decision.
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
//! shows. A root's verdict goes to the root when it is the wallet's send; a
//! dead root also takes every own send built on it (spending a dead
//! transaction is dead too), including sends built on it after it was found
//! dead. A root found in a block that the wallet has not
//! seen yet counts as settled, so the sends built on it are probed next.
//! Only changes are published — Unresolved never replaces a decided verdict,
//! and Accepted and Mined count as the same. The host is told to drop a
//! send's verdict (cleared) when the send settles or leaves the wallet, when
//! its wallet is removed and, for every send, when probing is turned off;
//! cleared says only "forget the verdict", not that the send settled.
//!
//! When: once as soon as dash-spv reports an `Uncertain` broadcast (the root
//! of that transaction's chain), then on each advance of the wallet's synced
//! height that follows the tip — steps of more than [`CATCH_UP_STEP`] blocks,
//! steps back (a rescan) and the first step after launch are catch-up and
//! skipped: passes queued before them probe only what an `Uncertain` result
//! forced (and still read the records, for the clears, while the host holds
//! verdicts for the wallet) — every block
//! for [`EVERY_BLOCK_WINDOW`] blocks, counted from the first height-driven
//! pass that sees the root while the wallet follows the tip (a forced pass
//! does not start it), and every [`SLOW_INTERVAL`]
//! blocks after. Each Uncertain report adds one immediate probe.
//! Between passes a wallet waits for the height at which its next root is due;
//! one whose last pass had nothing left to probe is idle and not scanned
//! again until a wallet event that touched its records (a new or
//! InstantSend-locked transaction, a block or chainlock that changed records, a
//! sweep) or an `Uncertain` result for one of its transactions wakes it. `Dead` and `Mined` end
//! the probing of a root; `Accepted` does not — a mempool is not settlement.
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
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

use async_trait::async_trait;
use dash_async::WorkerStatus;
use dash_spv::sync::SyncEvent;
use dash_spv::{BroadcastResult, EventHandler};
use dashcore::{Transaction, Txid};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex, RwLock};
use tokio::task::{self, AbortHandle, JoinError, JoinHandle, JoinSet};
use tokio::time::{timeout_at, Instant};

use crate::broadcast_probe::{AcceptanceProbe, ProbeVerdict};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Blocks from the start of a root's window (see [`ProbeSchedule`]) during
/// which it is probed on every block.
pub(crate) const EVERY_BLOCK_WINDOW: u32 = 24;

/// Blocks between probes once [`EVERY_BLOCK_WINDOW`] has passed.
pub(crate) const SLOW_INTERVAL: u32 = 10;

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
pub(crate) fn collect_views(info: &PlatformWalletInfo) -> Vec<OutgoingView> {
    let accounts = info.core_wallet.accounts.all_accounts();
    merge_records(accounts.iter().flat_map(|account| {
        account
            .transactions()
            .values()
            .map(move |record| RecordFacts {
                txid: record.txid,
                settled: record.is_confirmed()
                    || matches!(record.context, TransactionContext::InstantSend(_))
                    || account.transaction_is_finalized(&record.txid),
                own: !record.input_details.is_empty(),
                transaction: &record.transaction,
            })
    }))
}

/// This wallet's unsettled transactions as chains. `views` hold one entry per
/// transaction (see [`merge_records`]).
pub(crate) struct ChainGraph<'a> {
    by_txid: HashMap<Txid, &'a OutgoingView>,
    /// Unsettled for the purpose of picking roots: every view except those a
    /// probe found mined.
    unsettled: BTreeSet<Txid>,
    /// Unsettled transactions spending an output of each unsettled one.
    children: HashMap<Txid, Vec<Txid>>,
}

impl<'a> ChainGraph<'a> {
    /// `mined` are transactions a probe found in a block that this wallet has
    /// not seen yet: settled for the purpose of picking roots, so the sends
    /// built on them become roots in turn.
    pub(crate) fn new(views: &'a [OutgoingView], mined: &HashSet<Txid>) -> Self {
        let by_txid: HashMap<Txid, &OutgoingView> =
            views.iter().map(|view| (view.txid, view)).collect();
        let unsettled: BTreeSet<Txid> = by_txid
            .keys()
            .filter(|txid| !mined.contains(*txid))
            .copied()
            .collect();
        let mut children: HashMap<Txid, Vec<Txid>> = HashMap::new();
        for txid in &unsettled {
            let parents: BTreeSet<Txid> = by_txid[txid]
                .transaction
                .input
                .iter()
                .map(|input| input.previous_output.txid)
                .filter(|parent| unsettled.contains(parent))
                .collect();
            for parent in parents {
                children.entry(parent).or_default().push(*txid);
            }
        }
        Self {
            by_txid,
            unsettled,
            children,
        }
    }

    fn is_own(&self, txid: &Txid) -> bool {
        self.by_txid
            .get(txid)
            .is_some_and(|view| view.spends_own_coins)
    }

    fn is_unsettled(&self, txid: &Txid) -> bool {
        self.unsettled.contains(txid)
    }

    /// The wallet's unsettled sends.
    pub(crate) fn own_unsettled(&self) -> BTreeSet<Txid> {
        self.unsettled
            .iter()
            .filter(|txid| self.is_own(txid))
            .copied()
            .collect()
    }

    /// Unsettled transactions `txid` depends on, transitively, including
    /// itself.
    fn with_ancestors(&self, txid: &Txid) -> BTreeSet<Txid> {
        let mut found = BTreeSet::new();
        let mut frontier = vec![*txid];
        while let Some(txid) = frontier.pop() {
            if !self.is_unsettled(&txid) || !found.insert(txid) {
                continue;
            }
            for input in &self.by_txid[&txid].transaction.input {
                frontier.push(input.previous_output.txid);
            }
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
        let relevant: BTreeSet<Txid> = self
            .own_unsettled()
            .iter()
            .flat_map(|txid| self.with_ancestors(txid))
            .collect();
        relevant
            .iter()
            .filter(|txid| self.is_root(txid))
            .map(|txid| self.by_txid[txid])
            .collect()
    }

    /// The roots of `txid`'s chain — what to probe when dash-spv reports
    /// `txid` uncertain but it spends an unsettled parent.
    pub(crate) fn roots_of(&self, txid: &Txid) -> BTreeSet<Txid> {
        self.with_ancestors(txid)
            .into_iter()
            .filter(|txid| self.is_root(txid))
            .collect()
    }

    /// The wallet's own sends in `root`'s chain: `root` itself if it is one,
    /// and every unsettled send built on it, transitively. A dead root takes
    /// all of them with it.
    pub(crate) fn own_in_chain(&self, root: &Txid) -> BTreeSet<Txid> {
        let mut chain = BTreeSet::new();
        let mut frontier = vec![*root];
        while let Some(txid) = frontier.pop() {
            if !chain.insert(txid) {
                continue;
            }
            if let Some(children) = self.children.get(&txid) {
                frontier.extend(children.iter().copied());
            }
        }
        chain.into_iter().filter(|txid| self.is_own(txid)).collect()
    }
}

/// The roots of `views` with nothing known to be mined — see [`ChainGraph::roots`].
#[cfg(test)]
fn ambiguous_roots(views: &[OutgoingView]) -> Vec<&OutgoingView> {
    ChainGraph::new(views, &HashSet::new()).roots()
}

/// When a root is next due for a probe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProbeSchedule {
    /// Where the every-block window starts: the first height-driven pass that
    /// saw the root while the wallet followed the tip. `None` until then —
    /// the root was only seen by a forced pass, whose height may be a stale
    /// stored tip. Only anchored passes probe unforced roots, and they set the
    /// window first; `None` counting as in-window matters for `next_due`.
    window_start: Option<u32>,
    last_probe: Option<u32>,
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
            Some(_) if self.in_window(height) => true,
            Some(last) => height - last >= SLOW_INTERVAL,
        }
    }

    fn probed_at(&mut self, height: u32) {
        self.last_probe = Some(height);
    }

    /// The last probe produced nothing: due again at once.
    fn withdraw(&mut self) {
        self.last_probe = None;
    }

    /// The lowest height at which [`is_due`](Self::is_due) holds.
    pub(crate) fn next_due(&self) -> u32 {
        match self.last_probe {
            None => 0,
            Some(last) if self.in_window(last.saturating_add(1)) => last.saturating_add(1),
            Some(last) => last.saturating_add(SLOW_INTERVAL),
        }
    }
}

/// A root's final verdict: nothing is left to ask.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Final {
    Dead { reason: String },
    Mined,
}

/// Whether two verdicts say the same thing to the host. Accepted and Mined
/// both mean "going through" and reach the host as the same code; every
/// Unresolved counts as one, since its reason varies from probe to probe.
fn same_for_host(a: &ProbeVerdict, b: &ProbeVerdict) -> bool {
    matches!(
        (a, b),
        (
            ProbeVerdict::Accepted | ProbeVerdict::Mined,
            ProbeVerdict::Accepted | ProbeVerdict::Mined
        ) | (
            ProbeVerdict::Unresolved { .. },
            ProbeVerdict::Unresolved { .. }
        ) | (ProbeVerdict::Dead { .. }, ProbeVerdict::Dead { .. })
    )
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

/// Bookkeeping for every wallet. Schedules and final verdicts are per probed
/// root; published verdicts are per own send, which is what the host shows.
/// Everything for a transaction is kept while it stays unsettled in the
/// wallet, and dropped once it settles or leaves. Owned by the resolver's
/// actor, so every change happens in one sequence.
#[derive(Debug, Default)]
pub(crate) struct ResolverState {
    schedules: HashMap<(WalletId, Txid), ProbeSchedule>,
    finished: HashMap<(WalletId, Txid), Final>,
    published: HashMap<(WalletId, Txid), ProbeVerdict>,
    /// How many sends of each wallet have a published verdict.
    published_per_wallet: HashMap<WalletId, usize>,
}

impl ResolverState {
    /// The host holds a verdict for some send of `wallet_id`.
    fn has_published(&self, wallet_id: &WalletId) -> bool {
        self.published_per_wallet.contains_key(wallet_id)
    }

    /// Drop everything kept for `wallet_id`; a clear for every send the host
    /// had a verdict for.
    fn forget_wallet(&mut self, wallet_id: &WalletId) -> Vec<Outgoing> {
        self.schedules.retain(|(wallet, _), _| wallet != wallet_id);
        self.finished.retain(|(wallet, _), _| wallet != wallet_id);
        self.clear_where(|wallet, _| wallet == wallet_id, None, "removed")
    }

    /// Drop everything; a clear for every published send.
    fn forget_all(&mut self) -> Vec<Outgoing> {
        self.schedules.clear();
        self.finished.clear();
        self.clear_where(|_, _| true, None, "disabled")
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
        for (wallet_id, _) in &cleared {
            if let Some(count) = self.published_per_wallet.get_mut(wallet_id) {
                *count -= 1;
                if *count == 0 {
                    self.published_per_wallet.remove(wallet_id);
                }
            }
        }
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
    /// Unresolved never replaces a decided verdict (Accepted, Mined, Dead) —
    /// one probe that only met transport errors says nothing new.
    fn publish(&mut self, key: (WalletId, Txid), verdict: &ProbeVerdict) -> bool {
        let changed = match self.published.get(&key) {
            None => true,
            Some(previous)
                if !matches!(previous, ProbeVerdict::Unresolved { .. })
                    && matches!(verdict, ProbeVerdict::Unresolved { .. }) =>
            {
                false
            }
            Some(previous) => !same_for_host(previous, verdict),
        };
        if changed && self.published.insert(key, verdict.clone()).is_none() {
            *self.published_per_wallet.entry(key.0).or_insert(0) += 1;
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

    /// Roots of `wallet_id` a probe found in a block the wallet has not seen.
    fn mined(&self, wallet_id: &WalletId) -> HashSet<Txid> {
        self.finished
            .iter()
            .filter(|((wallet, _), result)| wallet == wallet_id && **result == Final::Mined)
            .map(|((_, txid), _)| *txid)
            .collect()
    }
}

/// The first advance after launch has no previous height to compare with, and
/// it is usually the stored tip from before the scan: treat it as catch-up
/// too. So is a step back — a rescan after a rewind (a new account) replays
/// old heights. Probing starts from the next step that follows the tip.
pub(crate) fn is_catch_up_step(previous: Option<u32>, height: u32) -> bool {
    previous.is_none_or(|previous| height < previous || height - previous > CATCH_UP_STEP)
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
}

/// The start of a pass: what the host must hear now, and the roots to probe.
#[derive(Debug)]
pub(crate) struct PassStart {
    pub events: Vec<Outgoing>,
    pub due: VecDeque<DueRoot>,
}

/// Start a pass over one wallet at `height`: forget what settled or left
/// (clearing the sends the host had a verdict for), re-publish every dead
/// root to every own send built on it — a send built on its change after it
/// was found dead is dead too — and pick the roots due for a probe. The
/// roots of every `forced` transaction's chain (dash-spv has just reported
/// it `Uncertain`) are due regardless of their schedule — even if one went
/// out at this height already: a repeat probe is cheaper than bookkeeping
/// that tells the two apart. A root found mined counts as settled when
/// picking roots, so its children are probed next. `anchor`: the tip height
/// a height-driven pass knows the wallet follows — it starts the every-block
/// window of every root that has none yet.
pub(crate) fn begin_pass(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
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
    state
        .finished
        .retain(|(wallet, txid), _| !is_gone(wallet, txid));
    // Cleared because the send is no longer an unsettled own send: it
    // settled, or left the wallet — not a statement that it settled.
    events.extend(state.clear_where(
        |wallet, txid| *wallet == wallet_id && !own_present.contains(txid),
        Some(height),
        "settled-or-left",
    ));

    let mined = state.mined(&wallet_id);
    let graph = ChainGraph::new(views, &mined);
    let roots = graph.roots();
    let forced_roots: BTreeSet<Txid> = forced
        .iter()
        .flat_map(|txid| graph.roots_of(txid))
        .collect();

    for view in &roots {
        if let Some(Final::Dead { reason }) = state.finished.get(&(wallet_id, view.txid)).cloned() {
            let verdict = ProbeVerdict::Dead { reason };
            let sends = graph.own_in_chain(&view.txid);
            let trigger = trigger_of(forced_roots.contains(&view.txid));
            state.publish_all(wallet_id, sends, &verdict, height, trigger, &mut events);
        }
    }

    let mut due = VecDeque::new();
    for view in &roots {
        let key = (wallet_id, view.txid);
        if state.finished.contains_key(&key) {
            continue;
        }
        let schedule = state.schedules.entry(key).or_default();
        if let Some(anchor) = anchor {
            schedule.anchor(anchor);
        }
        let forced = forced_roots.contains(&view.txid);
        // Without an anchor the pass is confined to forced roots.
        if forced || (anchor.is_some() && schedule.is_due(height)) {
            due.push_back(DueRoot {
                txid: view.txid,
                transaction: Arc::clone(&view.transaction),
                own: graph.is_own(&view.txid),
                forced,
            });
        }
    }
    PassStart { events, due }
}

/// A root is being sent to the network at `height`: its schedule counts from
/// here. Marked when the probe goes out, not when the pass is planned, so a
/// root the pass never reached stays due.
pub(crate) fn mark_sent(state: &mut ResolverState, wallet_id: WalletId, root: Txid, height: u32) {
    if let Some(schedule) = state.schedules.get_mut(&(wallet_id, root)) {
        schedule.probed_at(height);
    }
}

/// A root's probe never produced a verdict (its job panicked): nothing was
/// learned, so it is due again at once.
pub(crate) fn withdraw_send(state: &mut ResolverState, wallet_id: WalletId, root: Txid) {
    if let Some(schedule) = state.schedules.get_mut(&(wallet_id, root)) {
        schedule.withdraw();
    }
}

/// Record one root's probe result: a final verdict ends its probing, and the
/// verdict goes to the wallet's own sends — the root when it is one, and,
/// when it is dead, every own send built on it (the pass's `views`, walked
/// only then: Dead is terminal and rare).
pub(crate) fn record_probe(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    root: &DueRoot,
    verdict: &ProbeVerdict,
) -> Vec<Outgoing> {
    let mut events = Vec::new();
    let key = (wallet_id, root.txid);
    match verdict {
        ProbeVerdict::Dead { reason } => {
            state.finished.insert(
                key,
                Final::Dead {
                    reason: reason.clone(),
                },
            );
        }
        ProbeVerdict::Mined => {
            state.finished.insert(key, Final::Mined);
        }
        ProbeVerdict::Accepted | ProbeVerdict::Unresolved { .. } => {}
    }
    let sends: BTreeSet<Txid> = if matches!(verdict, ProbeVerdict::Dead { .. }) {
        ChainGraph::new(views, &state.mined(&wallet_id)).own_in_chain(&root.txid)
    } else if root.own {
        BTreeSet::from([root.txid])
    } else {
        BTreeSet::new()
    };
    state.publish_all(
        wallet_id,
        sends,
        verdict,
        height,
        trigger_of(root.forced),
        &mut events,
    );
    events
}

/// The lowest height at which a root of the wallet can be due, if nothing new
/// happens to it: a root without a schedule yet (the child of one just found
/// mined) is due at once; `u32::MAX` when every root has a final verdict
/// (nothing left to probe) or there are none.
pub(crate) fn next_pass_height(
    state: &ResolverState,
    wallet_id: WalletId,
    views: &[OutgoingView],
) -> u32 {
    let mined = state.mined(&wallet_id);
    ChainGraph::new(views, &mined)
        .roots()
        .iter()
        .map(|view| (wallet_id, view.txid))
        .filter(|key| !state.finished.contains_key(key))
        .map(|key| state.schedules.get(&key).map_or(0, ProbeSchedule::next_due))
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
/// need not `walk` the records (it is confined to forced roots and has no
/// verdicts to clear) and the wallet holds none of `forced`.
fn forced_held(
    walk: bool,
    forced: HashSet<Txid>,
    holds: impl Fn(&Txid) -> bool,
) -> Option<HashSet<Txid>> {
    let forced: HashSet<Txid> = forced.into_iter().filter(|txid| holds(txid)).collect();
    (walk || !forced.is_empty()).then_some(forced)
}

/// Where passes read wallets from.
#[async_trait]
pub(crate) trait WalletSource: Send + Sync {
    /// The wallets whose records hold `txid` — one look under one lock.
    async fn holders(&self, txid: Txid) -> Vec<WalletId>;

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

struct ManagerSource(Arc<RwLock<WalletManager<PlatformWalletInfo>>>);

#[async_trait]
impl WalletSource for ManagerSource {
    async fn holders(&self, txid: Txid) -> Vec<WalletId> {
        let manager = self.0.read().await;
        manager
            .list_wallets()
            .into_iter()
            .filter(|wallet_id| {
                manager
                    .get_wallet_info(wallet_id)
                    .is_some_and(|info| holds(info, &txid))
            })
            .copied()
            .collect()
    }

    async fn read(
        &self,
        wallet_id: WalletId,
        walk: &AtomicBool,
        forced: HashSet<Txid>,
    ) -> Option<Option<WalletRead>> {
        let manager = self.0.read().await;
        let info = manager.get_wallet_info(&wallet_id)?;
        // Checked again under this lock: the record may have left since the
        // holders lookup (a sweep, a removal).
        let forced = forced_held(walk.load(Ordering::SeqCst), forced, |txid| {
            holds(info, txid)
        });
        Some(forced.map(|forced| WalletRead {
            views: collect_views(info),
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
    Uncertain(Txid),
    SetEnabled(bool),
    WalletRemoved(WalletId),
}

/// What a finished job reports back to the actor.
enum JobDone {
    /// The wallets holding a transaction reported `Uncertain`.
    Listed { txid: Txid, wallets: Vec<WalletId> },
    Read {
        wallet_id: WalletId,
        run: u64,
        height: u32,
        read: Option<Option<WalletRead>>,
    },
    Probed {
        wallet_id: WalletId,
        run: u64,
        verdict: ProbeVerdict,
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
    /// Queued by a catch-up while the host holds verdicts for the wallet: it
    /// runs for the clears (sends settled in the scanned blocks) even with
    /// nothing to probe.
    clears: bool,
}

/// A pass in flight: reading the wallet, then probing its due roots one at
/// a time. Results carry the run id; a result for a run that was dropped
/// (probing off, wallet removed) is ignored.
struct Run {
    id: u64,
    /// The wallet's command count when the run started.
    seq: u64,
    /// Something touched the wallet during the run: its read may be stale, so
    /// the next height must not be skipped.
    woken: bool,
    /// Where this pass starts windows; `None` confines it to forced roots
    /// (see [`PendingPass::anchor_at`]).
    anchor_at: Option<u32>,
    /// It carries txids an `Uncertain` result forced.
    has_forced: bool,
    /// Whether the read walks the records even when none of the forced txids
    /// is held: shared with the read job, and cleared by a catch-up when the
    /// pass has nothing left to do there — no anchor, no verdicts to clear.
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
    /// Commands seen for this wallet. A run that found the wallet gone drops
    /// the entry only if none arrived since it started — a same-id wallet
    /// added meanwhile keeps its entry.
    seq: u64,
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
    job_runs: HashMap<task::Id, (WalletId, u64)>,
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
                    Some((wallet_id, run)) => {
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

    /// The wallet's entry, created on first sight, with this command counted.
    fn entry(&mut self, wallet_id: WalletId) -> &mut WalletEntry {
        let entry = self.wallets.entry(wallet_id).or_default();
        entry.seq += 1;
        entry
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Height { wallet_id, height } => {
                let enabled = self.enabled;
                let entry = self.entry(wallet_id);
                let previous = entry.height.replace(height);
                if is_catch_up_step(previous, height) {
                    // Behind the tip: passes queued or running from before are
                    // confined to what an `Uncertain` result forced — the scan
                    // settles most roots — and start no windows. While the host
                    // holds verdicts for the wallet, a pass still reads the
                    // records, for the clears of sends the scan settles.
                    let has_verdicts = self.state.has_published(&wallet_id);
                    let Some(entry) = self.wallets.get_mut(&wallet_id) else {
                        return;
                    };
                    if let Some(pending) = entry.pending.as_mut() {
                        pending.anchor_at = None;
                        pending.clears |= has_verdicts;
                        if pending.forced.is_empty() && !pending.clears {
                            entry.pending = None;
                        }
                    } else if has_verdicts && enabled {
                        entry.pending = Some(PendingPass {
                            height,
                            clears: true,
                            ..PendingPass::default()
                        });
                    }
                    if let Some(run) = entry.run.as_mut() {
                        run.anchor_at = None;
                        if !has_verdicts {
                            run.walk.store(false, Ordering::SeqCst);
                        }
                        if let Some(probing) = run.probing.as_mut() {
                            probing.due.retain(|root| root.forced);
                        }
                        // Still reading, with nothing forced and nothing to
                        // clear: stop its walk over the records now.
                        if run.probing.is_none() && !run.has_forced && !has_verdicts {
                            run.abort.abort();
                            self.end_run(wallet_id);
                            return;
                        }
                    }
                    self.start(wallet_id);
                    return;
                }
                // While a pass runs, its end decides the wait: queue the height
                // and let `probe_next` drop it if no root turns out due.
                if !enabled
                    || (entry.run.is_none() && entry.wait_until.is_some_and(|until| height < until))
                {
                    return;
                }
                let pending = entry.pending.get_or_insert_with(PendingPass::default);
                pending.height = pending.height.max(height);
                pending.anchor_at = Some(height);
                self.start(wallet_id);
            }
            Command::Seen { wallet_id, touched } => {
                let entry = self.entry(wallet_id);
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
                // Routed to the wallets holding it, including ones no event
                // has introduced yet.
                let source = Arc::clone(&self.source);
                self.jobs.spawn(async move {
                    JobDone::Listed {
                        txid,
                        wallets: source.holders(txid).await,
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
                    if !enabled {
                        entry.pending = None;
                        if let Some(run) = entry.run.take() {
                            run.abort.abort();
                        }
                    }
                }
                if !enabled {
                    let events = self.state.forget_all();
                    deliver(&self.sink, events);
                }
            }
            Command::WalletRemoved(wallet_id) => {
                if let Some(run) = self.wallets.remove(&wallet_id).and_then(|entry| entry.run) {
                    run.abort.abort();
                }
                let events = self.state.forget_wallet(&wallet_id);
                deliver(&self.sink, events);
            }
        }
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
        let has_forced = !pending.forced.is_empty();
        // Walk the records if the pass may probe by schedule or has verdicts
        // to clear; a pass confined to forced roots with none held skips it.
        let walk = Arc::new(AtomicBool::new(
            anchor_at.is_some() || pending.clears || self.state.has_published(&wallet_id),
        ));
        self.next_run += 1;
        let run = self.next_run;
        let source = Arc::clone(&self.source);
        let job_walk = Arc::clone(&walk);
        let abort = self.jobs.spawn(async move {
            let read = source.read(wallet_id, &job_walk, pending.forced).await;
            JobDone::Read {
                wallet_id,
                run,
                height: pending.height,
                read,
            }
        });
        self.job_runs.insert(abort.id(), (wallet_id, run));
        entry.run = Some(Run {
            id: run,
            seq: entry.seq,
            woken: false,
            anchor_at,
            has_forced,
            walk,
            abort,
            probing: None,
        });
    }

    fn job_done(&mut self, done: JobDone) {
        match done {
            JobDone::Listed { txid, wallets } => {
                if !self.enabled {
                    return;
                }
                for wallet_id in wallets {
                    // Counted like a command: a run that read the id as gone
                    // before this must not drop the entry and its request.
                    let entry = self.entry(wallet_id);
                    let pending = entry.pending.get_or_insert_with(PendingPass::default);
                    pending.height = pending.height.max(entry.height.unwrap_or(0));
                    pending.forced.insert(txid);
                    self.start(wallet_id);
                }
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
                let run_seq = match entry.run.as_ref() {
                    Some(current) if current.id == run => current.seq,
                    _ => return,
                };
                match read {
                    None => {
                        // Gone from the wallet manager. Every removal path
                        // also sends `WalletRemoved`, but that command may
                        // still be queued, or a lookup may have raced it:
                        // forget the wallet here too — clearing twice is
                        // harmless. Keep the entry if a command for a same-id
                        // wallet arrived meanwhile.
                        let keep = run_seq != entry.seq;
                        if !keep {
                            self.wallets.remove(&wallet_id);
                        }
                        let events = self.state.forget_wallet(&wallet_id);
                        deliver(&self.sink, events);
                        if keep {
                            // The wait belonged to the state just forgotten.
                            if let Some(entry) = self.wallets.get_mut(&wallet_id) {
                                entry.wait_until = None;
                            }
                            self.end_run(wallet_id);
                        }
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
                        let start = begin_pass(
                            &mut self.state,
                            wallet_id,
                            height,
                            &read.views,
                            &read.forced,
                            current.anchor_at,
                        );
                        current.probing = Some(Probing {
                            views: read.views,
                            height,
                            due: start.due,
                            in_flight: None,
                        });
                        // Clears and re-published dead verdicts reach the host
                        // before the pass goes out to the network.
                        deliver(&self.sink, start.events);
                        self.probe_next(wallet_id);
                    }
                }
            }
            JobDone::Probed {
                wallet_id,
                run,
                verdict,
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
                let Some(root) = probing.in_flight.take() else {
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        "broadcast probe: a probe result with no root in flight"
                    );
                    self.end_run(wallet_id);
                    return;
                };
                let events = record_probe(
                    &mut self.state,
                    wallet_id,
                    probing.height,
                    &probing.views,
                    &root,
                    &verdict,
                );
                deliver(&self.sink, events);
                self.probe_next(wallet_id);
            }
        }
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
            run.abort = self.jobs.spawn(async move {
                let verdict = probe.probe(&transaction).await;
                JobDone::Probed {
                    wallet_id,
                    run: id,
                    verdict,
                }
            });
            self.job_runs.insert(run.abort.id(), (wallet_id, id));
            mark_sent(&mut self.state, wallet_id, root.txid, probing.height);
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
            if pending.forced.is_empty() && !pending.clears && pending.height < until {
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
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return; // queued; started by the first command sent from a runtime
        };
        let mut slot = self.actor.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*slot, ActorSlot::NotStarted(..)) {
            if let ActorSlot::NotStarted(actor, receiver) =
                std::mem::replace(&mut *slot, ActorSlot::Closed)
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
    let slot = std::mem::replace(
        &mut *actor.lock().unwrap_or_else(PoisonError::into_inner),
        ActorSlot::Closed,
    );
    let mut handle = match slot {
        ActorSlot::Running(handle, stop) => {
            let _ = stop.send(());
            handle
        }
        ActorSlot::Stopping(handle) => handle,
        ActorSlot::NotStarted(..) | ActorSlot::Closed => return WorkerStatus::NotRunning,
    };
    match timeout_at(deadline, &mut handle).await {
        Ok(Ok(())) => WorkerStatus::Ok,
        Ok(Err(error)) if error.is_panic() => WorkerStatus::Panicked(error.to_string()),
        Ok(Err(error)) => WorkerStatus::Stopped(Some(error.to_string())),
        Err(_) => {
            // Told to stop, still going — likely inside a host callback. Not
            // aborted: a retried stop waits for it to end on its own and can
            // then report it clean.
            *actor.lock().unwrap_or_else(PoisonError::into_inner) = ActorSlot::Stopping(handle);
            WorkerStatus::Timeout
        }
    }
}

/// Whether a wallet event may have left something new to probe. A chainlock
/// that locked nothing, or a block that only matured coins or moved balances,
/// does not — chainlocks arrive about once per block for every wallet.
fn may_leave_work(event: &WalletEvent) -> bool {
    match event {
        WalletEvent::TransactionDetected { .. }
        | WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::TransactionsSwept { .. } => true,
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => !inserted.is_empty() || !updated.is_empty(),
        WalletEvent::ChainLockProcessed {
            locked_transactions,
            ..
        } => locked_transactions.values().any(|txids| !txids.is_empty()),
        WalletEvent::SyncHeightAdvanced { .. } => false,
    }
}

impl EventHandler for BroadcastResolver {
    fn on_wallet_event(&self, event: &WalletEvent) {
        let wallet_id = event.wallet_id();
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
        if let SyncEvent::TransactionBroadcastResult {
            txid,
            result: BroadcastResult::Uncertain,
        } = event
        {
            self.send(Command::Uncertain(*txid));
        }
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
    use dashcore::{BlockHash, OutPoint, TxIn};
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionRecord;
    use key_wallet::WalletCoreBalance;

    use super::*;

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
        let graph = ChainGraph::new(&views, &none());
        assert_eq!(graph.roots_of(&txid(3)), BTreeSet::from([txid(1)]));
        assert!(graph.roots_of(&txid(9)).is_empty(), "unknown txid");
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
        async fn probe(&self, transaction: &Transaction) -> ProbeVerdict {
            let id = txid(transaction.lock_time as u8);
            self.probed.lock().expect("probed").push(id);
            self.answers
                .get(&id)
                .cloned()
                .unwrap_or(ProbeVerdict::Unresolved {
                    reason: "scripted".to_string(),
                })
        }
    }

    fn wallet() -> WalletId {
        [7u8; 32]
    }

    fn dead() -> ProbeVerdict {
        ProbeVerdict::Dead {
            reason: "bad-txns-inputs-missingorspent".to_string(),
        }
    }

    fn unresolved() -> ProbeVerdict {
        ProbeVerdict::Unresolved {
            reason: "no quorum".to_string(),
        }
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
            let events = self.state.forget_wallet(wallet_id);
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
            );
            let verdict = probe.probe(&root.transaction).await;
            let mut guard = state.lock().expect("state");
            let events = record_probe(&mut guard.state, wallet_id, height, views, &root, &verdict);
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

    #[tokio::test]
    async fn should_stop_probing_a_root_proven_dead() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        let again = pass(&state, &probe, 101, &views).await;

        assert!(again.is_empty());
        assert_eq!(probe.probed(), vec![txid(1)]);
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
        assert!(guard.state.finished.is_empty());
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

    /// A mined transaction the wallet has not caught up with yet: nothing left
    /// to ask, so it is not probed again.
    #[tokio::test]
    async fn should_stop_probing_a_root_found_in_a_block() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Mined)]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        let again = pass(&state, &probe, 101, &views).await;

        assert!(again.is_empty());
        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// Spending a dead transaction is dead too: the whole chain of own sends
    /// built on a dead root is published dead, not just the root.
    #[tokio::test]
    async fn should_publish_a_dead_root_for_every_own_send_built_on_it() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = pass(&state, &probe, 100, &views).await;

        assert_eq!(
            events,
            vec![
                ResolverEvent::Verdict(txid(1), dead()),
                ResolverEvent::Verdict(txid(2), dead()),
            ]
        );
        let again = pass(&state, &probe, 101, &views).await;
        assert!(
            again.is_empty(),
            "a dead child is not cleared for not being a root"
        );
    }

    /// Someone else's incoming payment is probed, but the host hears only
    /// about the wallet's own send — the only row it can attach a verdict to.
    #[tokio::test]
    async fn should_publish_a_dead_incoming_parent_on_the_own_send_only() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(5), dead())]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        let events = pass(&state, &probe, 100, &views).await;

        assert_eq!(probe.probed(), vec![txid(5)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(6), dead())]);
    }

    #[tokio::test]
    async fn should_publish_nothing_while_an_incoming_parent_is_only_accepted() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(5), ProbeVerdict::Accepted)]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        assert!(pass(&state, &probe, 100, &views).await.is_empty());
    }

    /// A root found in a block that the wallet has not caught up with is
    /// settled for picking roots, so the send built on it is probed next.
    #[tokio::test]
    async fn should_probe_the_children_of_a_root_found_in_a_block() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Mined), (txid(2), dead())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let first = pass(&state, &probe, 100, &views).await;
        let second = pass(&state, &probe, 101, &views).await;

        assert_eq!(
            first,
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Mined)]
        );
        assert_eq!(second, vec![ResolverEvent::Verdict(txid(2), dead())]);
        assert_eq!(probe.probed(), vec![txid(1), txid(2)]);
    }

    /// A send built on a dead root's change after the root was found dead is
    /// dead too, and hears it without the root being asked again.
    #[tokio::test]
    async fn should_publish_a_dead_root_to_a_send_built_on_it_later() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);

        pass(&state, &probe, 100, &[send(1, &[outpoint(90, 0)])]).await;
        let later = pass(
            &state,
            &probe,
            101,
            &[send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])],
        )
        .await;

        assert_eq!(later, vec![ResolverEvent::Verdict(txid(2), dead())]);
        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// A wallet whose every root has a final verdict has nothing to probe and
    /// may go idle; one with a root still open may not.
    #[tokio::test]
    async fn should_report_nothing_to_probe_only_when_every_root_is_final() {
        let state = Mutex::new(Harness::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead()), (txid(2), unresolved())]);

        let empty = run(&state, &probe, wallet(), 100, &[], &none()).await;
        let dead_only = run(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &none(),
        )
        .await;
        let open = run(
            &state,
            &probe,
            wallet(),
            101,
            &[send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
            &none(),
        )
        .await;

        assert_eq!(empty, u32::MAX);
        assert_eq!(dead_only, u32::MAX);
        assert_eq!(open, 102, "the open root is due next block");
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
        assert!(
            guard.state.published.is_empty()
                && guard.state.schedules.is_empty()
                && guard.state.finished.is_empty()
        );
    }

    /// One probe that met only transport errors says nothing new about a send
    /// already seen accepted; the UI must not flicker back to "unknown". And
    /// Accepted → Mined is no news to the host either: both mean "going
    /// through" and reach it as the same code.
    #[test]
    fn should_publish_only_what_the_host_would_see_as_a_change() {
        let mut state = ResolverState::default();
        let key = (wallet(), txid(1));

        assert!(state.publish(key, &ProbeVerdict::Accepted));
        assert!(!state.publish(key, &unresolved()));
        assert!(!state.publish(key, &ProbeVerdict::Mined));
        assert!(!state.publish(key, &ProbeVerdict::Accepted));
        assert!(state.publish(key, &dead()));
        assert!(
            !state.publish(key, &unresolved()),
            "Unresolved never replaces Dead either"
        );
        assert!(!state.publish(
            key,
            &ProbeVerdict::Dead {
                reason: "missing inputs".into()
            }
        ));
    }

    /// A long chain on one root: every own send in it, found in one walk.
    #[test]
    fn should_find_every_own_send_in_a_long_chain() {
        let mut views = vec![send(1, &[outpoint(90, 0)])];
        for n in 2..=20u8 {
            views.push(send(n, &[outpoint(n - 1, 1)]));
        }
        views.push(incoming(30, &[outpoint(80, 0)]));
        let graph = ChainGraph::new(&views, &none());

        assert_eq!(
            graph.own_in_chain(&txid(1)),
            (1..=20u8).map(txid).collect::<BTreeSet<_>>()
        );
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
        resume: tokio::sync::Notify,
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
        async fn holders(&self, txid: Txid) -> Vec<WalletId> {
            self.views
                .lock()
                .expect("views")
                .iter()
                .filter(|(_, views)| views.iter().any(|view| view.txid == txid))
                .map(|(wallet_id, _)| *wallet_id)
                .collect()
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

        fn views(&self, wallet_id: WalletId, views: Vec<OutgoingView>) {
            self.source
                .views
                .lock()
                .expect("views")
                .insert(wallet_id, views);
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

    /// Once every root is final the wallet is idle: height advances do not walk
    /// its records again until an event that touched them.
    #[tokio::test]
    async fn should_go_idle_when_every_root_is_final_until_a_touch() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), dead())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.follow(wallet(), 100).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), dead())]);
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
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), dead())]))).await;
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
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), dead())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.send(Command::SetEnabled(false)).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);

        rig.send(Command::SetEnabled(true)).await;
        rig.height(wallet(), 102).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), dead())]);
        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// Probing turned off while a pass is out on the network: its result is
    /// dropped, nothing is published after the clears.
    #[tokio::test]
    async fn should_drop_a_pass_in_flight_when_probing_goes_off() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), dead())]))).await;
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

    /// A late event can register an id the wallet manager no longer holds;
    /// the first pass that finds it gone drops it.
    #[tokio::test]
    async fn should_drop_a_wallet_the_manager_no_longer_holds() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[]))).await;

        rig.height(wallet(), 100).await;
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        rig.settle().await;

        assert!(!rig.actor.wallets.contains_key(&wallet()));
    }

    /// An `Uncertain` result right after launch, before any event introduced
    /// the wallet, still gets its immediate probe.
    #[tokio::test]
    async fn should_route_an_uncertain_result_to_a_wallet_no_event_introduced() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);

        rig.send(Command::Uncertain(txid(1))).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    /// A wallet the manager dropped without `wallet_removed` (a rolled-back
    /// registration or load): the pass that finds it gone clears its verdicts.
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
        let probe = Arc::new(ScriptedProbe::new(&[
            (txid(1), dead()),
            (txid(2), unresolved()),
        ]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.height(wallet(), 102).await;
        assert_eq!(probe.probed(), vec![txid(1)], "idle after the dead root");
        rig.views(
            wallet(),
            vec![send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])],
        );

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

        assert_eq!(probe.probed(), vec![txid(1), txid(2), txid(2)]);
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
            Some(100),
        );
        let planned: Vec<Txid> = start.due.iter().map(|root| root.txid).collect();
        assert_eq!(planned.len(), 2);

        // Only the first root goes out before the run ends.
        mark_sent(&mut state, wallet(), planned[0], 100);
        let again = begin_pass(
            &mut state,
            wallet(),
            100,
            &views,
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
            Some(100),
        );
        mark_sent(&mut state, wallet(), txid(1), 100);
        assert!(!state.schedules[&(wallet(), txid(1))].is_due(100));

        withdraw_send(&mut state, wallet(), txid(1));

        assert!(state.schedules[&(wallet(), txid(1))].is_due(100));
    }

    /// Two stops at once: the second waits for the first and never reports
    /// the task gone while it still runs.
    #[tokio::test]
    async fn should_not_report_a_running_task_as_not_running_to_a_concurrent_stop() {
        let (stop, _stopped) = oneshot::channel();
        let handle = tokio::spawn(tokio::time::sleep(Duration::from_millis(200)));
        let slot = Mutex::new(ActorSlot::Running(handle, stop));
        let stopping = AsyncMutex::new(());
        let started = Instant::now();

        let (first, second) = tokio::join!(
            stop_slot(&slot, &stopping, Duration::from_secs(5)),
            stop_slot(&slot, &stopping, Duration::from_secs(5)),
        );

        assert_eq!(first, WorkerStatus::Ok);
        assert_eq!(second, WorkerStatus::NotRunning);
        assert!(
            started.elapsed() >= Duration::from_millis(150),
            "waited for the task"
        );
    }

    /// A stop that runs out of budget keeps the task without aborting it: a
    /// retried stop waits for it to end and reports it clean.
    #[tokio::test]
    async fn should_report_a_task_that_outlived_a_stop_clean_on_retry() {
        let (stop, _stopped) = oneshot::channel();
        let handle = tokio::spawn(tokio::time::sleep(Duration::from_millis(300)));
        let slot = Mutex::new(ActorSlot::Running(handle, stop));
        let stopping = AsyncMutex::new(());

        let first = stop_slot(&slot, &stopping, Duration::from_millis(20)).await;
        let second = stop_slot(&slot, &stopping, Duration::from_secs(5)).await;
        let third = stop_slot(&slot, &stopping, Duration::from_secs(5)).await;

        assert_eq!(first, WorkerStatus::Timeout);
        assert_eq!(second, WorkerStatus::Ok);
        assert_eq!(third, WorkerStatus::NotRunning);
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

    /// A catch-up does not stop a reading pass while the host holds verdicts
    /// for the wallet: the pass still clears the send that settled.
    #[tokio::test]
    async fn should_let_a_pass_clear_verdicts_through_a_catch_up() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        assert_eq!(
            rig.sent(),
            vec![ResolverEvent::Verdict(txid(1), unresolved())]
        );
        rig.views(wallet(), Vec::new()); // settled

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 140,
        });
        rig.settle().await;

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
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

    /// A catch-up on a wallet whose host holds verdicts reads the records for
    /// the clears even when no pass was queued or running: a send settled in
    /// the scanned blocks stops showing its verdict during the scan.
    #[tokio::test]
    async fn should_clear_verdicts_during_a_catch_up_with_no_pass_queued() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), dead())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        assert_eq!(rig.sent(), vec![ResolverEvent::Verdict(txid(1), dead())]);
        rig.views(wallet(), Vec::new()); // settled in the blocks being scanned

        rig.height(wallet(), 140).await; // catch-up; idle, nothing queued

        assert_eq!(rig.sent(), vec![ResolverEvent::Cleared(txid(1))]);
    }

    /// A queued pass overtaken by a catch-up is kept while verdicts are out,
    /// and clears what the scan settled.
    #[tokio::test]
    async fn should_keep_a_queued_pass_for_the_clears_through_a_catch_up() {
        let mut rig = enabled(Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]))).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.follow(wallet(), 100).await;
        rig.sent();

        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 102,
        });
        // The running pass has read the records from before the scan and is
        // probing; only the queued one can see the send settle.
        let read = rig.actor.jobs.join_next_with_id().await.expect("read");
        rig.actor.joined(read);
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 103,
        });
        rig.views(wallet(), Vec::new()); // settled in the scan
        rig.handle(Command::Height {
            wallet_id: wallet(),
            height: 140,
        });
        rig.settle().await;

        assert!(rig.sent().contains(&ResolverEvent::Cleared(txid(1))));
    }

    /// A probe that panics on its `nth` call and accepts otherwise.
    struct PanicOnCall {
        nth: usize,
        calls: StdMutex<usize>,
    }

    #[async_trait]
    impl AcceptanceProbe for PanicOnCall {
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
            let call = {
                let mut calls = self.calls.lock().expect("calls");
                *calls += 1;
                *calls
            };
            if call == self.nth {
                panic!("probe bug");
            }
            ProbeVerdict::Accepted
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
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
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
    struct HangingProbe;

    #[async_trait]
    impl AcceptanceProbe for HangingProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
            std::future::pending().await
        }
    }

    /// Stopping the task aborts and awaits every job it started, so nothing
    /// outlives it.
    #[tokio::test]
    async fn should_end_with_every_job_when_stopped() {
        let rig = Rig::new(Arc::new(HangingProbe));
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        let (commands, receiver) = mpsc::unbounded_channel();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(rig.actor.run(receiver, stopped));
        for command in [Command::SetEnabled(true), Command::Uncertain(txid(1))] {
            commands.send(command).expect("send");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;

        stop.send(()).expect("stop");

        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("stopped in time")
            .expect("no panic");
    }

    /// A probe that records what the host had been sent by the time it ran.
    struct SnapshotProbe {
        sink: Arc<CollectSink>,
        seen: StdMutex<Vec<ResolverEvent>>,
    }

    #[async_trait]
    impl AcceptanceProbe for SnapshotProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
            *self.seen.lock().expect("seen") = self.sink.0.lock().expect("sink").clone();
            ProbeVerdict::Accepted
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

    /// A chainlock arrives about once per block for every wallet; only one that
    /// locked something, or a block that changed records, may leave work.
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
        assert!(may_leave_work(&lock(vec![txid(1)])));
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
