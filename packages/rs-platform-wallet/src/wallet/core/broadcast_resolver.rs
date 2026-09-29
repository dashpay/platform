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
//! and the first step after launch, are catch-up and skipped — every block
//! for [`EVERY_BLOCK_WINDOW`] blocks and every [`SLOW_INTERVAL`] blocks after.
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
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::{self, AbortHandle, JoinError, JoinHandle, JoinSet};

use crate::broadcast_probe::{AcceptanceProbe, ProbeVerdict};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Blocks after a root is first seen during which it is probed on every block.
pub(crate) const EVERY_BLOCK_WINDOW: u32 = 24;

/// Blocks between probes once [`EVERY_BLOCK_WINDOW`] has passed.
pub(crate) const SLOW_INTERVAL: u32 = 10;

/// How far a wallet's synced height may advance in one step and still count
/// as following the tip. Larger steps are catch-up after time offline: the
/// scan itself settles most roots, so a height-driven pass waits for it.
pub(crate) const CATCH_UP_STEP: u32 = 3;

/// One unsettled transaction of the wallet, merged across the accounts that
/// record it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutgoingView {
    pub txid: Txid,
    /// The transaction spends at least one of this wallet's coins, i.e. the
    /// wallet signed it.
    pub spends_own_coins: bool,
    pub transaction: Transaction,
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
            transaction: transaction.clone(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProbeSchedule {
    first_seen: u32,
    last_probe: Option<u32>,
}

impl ProbeSchedule {
    pub(crate) fn new(first_seen: u32) -> Self {
        Self {
            first_seen,
            last_probe: None,
        }
    }

    pub(crate) fn is_due(&self, height: u32) -> bool {
        match self.last_probe {
            None => true,
            Some(last) if height <= last => false,
            Some(_) if height.saturating_sub(self.first_seen) < EVERY_BLOCK_WINDOW => true,
            Some(last) => height - last >= SLOW_INTERVAL,
        }
    }

    fn probed_at(&mut self, height: u32) {
        self.last_probe = Some(height);
    }

    /// The lowest height at which [`is_due`](Self::is_due) holds.
    pub(crate) fn next_due(&self) -> u32 {
        match self.last_probe {
            None => 0,
            Some(last)
                if last.saturating_add(1).saturating_sub(self.first_seen) < EVERY_BLOCK_WINDOW =>
            {
                last.saturating_add(1)
            }
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
fn trigger_of(forced: &HashSet<Txid>) -> &'static str {
    if forced.is_empty() {
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
}

impl ResolverState {
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
/// too. Probing starts from the next step.
pub(crate) fn is_catch_up_step(previous: Option<u32>, height: u32) -> bool {
    previous.is_none_or(|previous| height.saturating_sub(previous) > CATCH_UP_STEP)
}

/// The start of a pass: what the host must hear now, and the roots to probe.
#[derive(Debug)]
pub(crate) struct PassStart {
    pub events: Vec<Outgoing>,
    pub due: VecDeque<(Txid, Transaction)>,
}

/// Start a pass over one wallet at `height`: forget what settled or left
/// (clearing the sends the host had a verdict for), re-publish every dead
/// root to every own send built on it — a send built on its change after it
/// was found dead is dead too — and pick the roots due for a probe. The
/// roots of every `forced` transaction's chain are due regardless of their
/// schedule (dash-spv has just reported it `Uncertain`). A root found mined
/// counts as settled when picking roots, so its children are probed next.
pub(crate) fn begin_pass(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
) -> PassStart {
    let trigger = trigger_of(forced);
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
    events.extend(state.clear_where(
        |wallet, txid| *wallet == wallet_id && !own_present.contains(txid),
        Some(height),
        trigger,
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
            state.publish_all(wallet_id, sends, &verdict, height, trigger, &mut events);
        }
    }

    let mut due = VecDeque::new();
    for view in &roots {
        let key = (wallet_id, view.txid);
        if state.finished.contains_key(&key) {
            continue;
        }
        let schedule = state
            .schedules
            .entry(key)
            .or_insert_with(|| ProbeSchedule::new(height));
        if forced_roots.contains(&view.txid) || schedule.is_due(height) {
            schedule.probed_at(height);
            due.push_back((view.txid, view.transaction.clone()));
        }
    }
    PassStart { events, due }
}

/// Record one root's probe result: a final verdict ends its probing, and the
/// verdict goes to the wallet's own sends — the root when it is one, and,
/// when it is dead, every own send built on it.
pub(crate) fn record_probe(
    state: &mut ResolverState,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
    root: Txid,
    verdict: &ProbeVerdict,
) -> Vec<Outgoing> {
    let mut events = Vec::new();
    let key = (wallet_id, root);
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
        _ => {}
    }
    let graph = ChainGraph::new(views, &HashSet::new());
    let sends: BTreeSet<Txid> = if matches!(verdict, ProbeVerdict::Dead { .. }) {
        graph.own_in_chain(&root)
    } else if graph.is_own(&root) {
        BTreeSet::from([root])
    } else {
        BTreeSet::new()
    };
    state.publish_all(
        wallet_id,
        sends,
        verdict,
        height,
        trigger_of(forced),
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
/// transaction acts on it. `None`: nothing to do for this wallet — no height
/// asked for the pass and it holds none of `forced`.
fn forced_held(
    by_height: bool,
    forced: HashSet<Txid>,
    holds: impl Fn(&Txid) -> bool,
) -> Option<HashSet<Txid>> {
    let forced: HashSet<Txid> = forced.into_iter().filter(|txid| holds(txid)).collect();
    (by_height || !forced.is_empty()).then_some(forced)
}

/// Where passes read wallets from.
#[async_trait]
pub(crate) trait WalletSource: Send + Sync {
    /// The wallets whose records hold `txid` — one look under one lock.
    async fn holders(&self, txid: Txid) -> Vec<WalletId>;

    /// `None`: no such wallet. `Some(None)`: nothing to do (see
    /// [`forced_held`]) — decided without walking the records.
    async fn read(
        &self,
        wallet_id: WalletId,
        by_height: bool,
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
        by_height: bool,
        forced: HashSet<Txid>,
    ) -> Option<Option<WalletRead>> {
        let manager = self.0.read().await;
        let info = manager.get_wallet_info(&wallet_id)?;
        // Checked again under this lock: the record may have left since the
        // holders lookup (a sweep, a removal).
        let forced = forced_held(by_height, forced, |txid| holds(info, txid));
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

/// Hand `events` to the host. A handler that panics does not take the
/// resolver down, but the event it panicked on is lost — it is not sent
/// again. A panicking handler is a host bug; this only keeps probing alive
/// where panics unwind (the iOS profiles abort instead).
fn deliver(sink: &Arc<dyn VerdictSink>, events: Vec<Outgoing>) {
    for item in &events {
        if catch_unwind(AssertUnwindSafe(|| sink.deliver(item))).is_err() {
            tracing::error!(
                wallet_id = %hex::encode(item.wallet_id),
                event = ?item.event,
                "broadcast probe: a host handler panicked; its event is lost"
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
    /// The wallets holding a transaction reported `Uncertain` at command
    /// `since`.
    Listed {
        txid: Txid,
        since: u64,
        wallets: Vec<WalletId>,
    },
    Read {
        wallet_id: WalletId,
        run: u64,
        height: u32,
        forced_since: Option<u64>,
        read: Option<Option<WalletRead>>,
    },
    Probed {
        wallet_id: WalletId,
        run: u64,
        root: Txid,
        verdict: ProbeVerdict,
    },
}

#[derive(Default)]
struct PendingPass {
    height: u32,
    /// A height advance asked for this pass; otherwise only `Uncertain`
    /// results did.
    by_height: bool,
    forced: HashSet<Txid>,
    /// The latest command at which one of `forced` was reported.
    forced_since: Option<u64>,
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
    abort: AbortHandle,
    probing: Option<Probing>,
}

struct Probing {
    views: Vec<OutgoingView>,
    height: u32,
    forced: HashSet<Txid>,
    due: VecDeque<(Txid, Transaction)>,
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
    /// A probe job also names its root, whose send time a panic withdraws.
    job_runs: HashMap<task::Id, (WalletId, u64, Option<Txid>)>,
    /// Commands handled so far: orders an `Uncertain` result against probes.
    tick: u64,
    /// When each root was last sent to the network: the command count and the
    /// pass height. A forced pass skips a root already probed at its height
    /// since the `Uncertain` result arrived.
    probed: HashMap<(WalletId, Txid), (u64, u32)>,
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
            tick: 0,
            probed: HashMap::new(),
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
                // but its roots are due to the schedule. A panicked lookup
                // for an `Uncertain` result belongs to no run: wake every
                // wallet instead.
                match owner {
                    Some((wallet_id, run, root)) => {
                        // The probe never reached a verdict: it must not
                        // count as sent for the forced-probe dedup.
                        if let Some(root) = root {
                            self.probed.remove(&(wallet_id, root));
                        }
                        if let Some(entry) = self.wallets.get_mut(&wallet_id) {
                            if entry.run.as_ref().is_some_and(|current| current.id == run) {
                                entry.run = None;
                                entry.wait_until = None;
                                self.start(wallet_id);
                            }
                        }
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
        self.tick += 1;
        match command {
            Command::Height { wallet_id, height } => {
                let enabled = self.enabled;
                let entry = self.entry(wallet_id);
                let previous = entry.height.replace(height);
                // While a pass runs, its end decides the wait: queue the height
                // and let `probe_next` drop it if no root turns out due.
                if !enabled
                    || (entry.run.is_none() && entry.wait_until.is_some_and(|until| height < until))
                    || is_catch_up_step(previous, height)
                {
                    return;
                }
                let pending = entry.pending.get_or_insert_with(PendingPass::default);
                pending.height = pending.height.max(height);
                pending.by_height = true;
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
                let since = self.tick;
                self.jobs.spawn(async move {
                    JobDone::Listed {
                        txid,
                        since,
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
                    self.probed.clear();
                    let events = self.state.forget_all();
                    deliver(&self.sink, events);
                }
            }
            Command::WalletRemoved(wallet_id) => {
                if let Some(run) = self.wallets.remove(&wallet_id).and_then(|entry| entry.run) {
                    run.abort.abort();
                }
                self.probed.retain(|(wallet, _), _| *wallet != wallet_id);
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
        self.next_run += 1;
        let run = self.next_run;
        let source = Arc::clone(&self.source);
        let abort = self.jobs.spawn(async move {
            let read = source
                .read(wallet_id, pending.by_height, pending.forced)
                .await;
            JobDone::Read {
                wallet_id,
                run,
                height: pending.height,
                forced_since: pending.forced_since,
                read,
            }
        });
        self.job_runs.insert(abort.id(), (wallet_id, run, None));
        entry.run = Some(Run {
            id: run,
            seq: entry.seq,
            woken: false,
            abort,
            probing: None,
        });
    }

    fn job_done(&mut self, done: JobDone) {
        match done {
            JobDone::Listed {
                txid,
                since,
                wallets,
            } => {
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
                    // The latest result: a probe sent before it must not
                    // stand in for the forced probe it asks for.
                    pending.forced_since =
                        Some(pending.forced_since.map_or(since, |s| s.max(since)));
                    self.start(wallet_id);
                }
            }
            JobDone::Read {
                wallet_id,
                run,
                height,
                forced_since,
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
                        // Gone from the wallet manager — not always through
                        // `wallet_removed` (a failed registration or load is
                        // rolled back without it) — so forget it here too;
                        // clearing twice is harmless. Keep the entry if a
                        // command for a same-id wallet arrived meanwhile.
                        if run_seq == entry.seq {
                            self.wallets.remove(&wallet_id);
                        } else {
                            entry.run = None;
                        }
                        self.probed.retain(|(wallet, _), _| *wallet != wallet_id);
                        let events = self.state.forget_wallet(&wallet_id);
                        deliver(&self.sink, events);
                        self.start(wallet_id);
                    }
                    Some(None) => {
                        entry.run = None;
                        self.start(wallet_id);
                    }
                    Some(Some(read)) => {
                        // A request queued before any height was reported
                        // carries 0; the wallet's own synced height is better.
                        let height = height.max(read.synced_height);
                        let mut start = begin_pass(
                            &mut self.state,
                            wallet_id,
                            height,
                            &read.views,
                            &read.forced,
                        );
                        // Forget send times of roots that settled or left.
                        let present: HashSet<Txid> =
                            read.views.iter().map(|view| view.txid).collect();
                        self.probed.retain(|(wallet, txid), _| {
                            *wallet != wallet_id || present.contains(txid)
                        });
                        // A root already sent to the network at this height
                        // after the `Uncertain` result arrived needs no second
                        // probe — the forced one would only repeat it.
                        if let Some(since) = forced_since {
                            let probed = &self.probed;
                            start.due.retain(|(root, _)| {
                                !probed.get(&(wallet_id, *root)).is_some_and(
                                    |&(tick, probed_height)| {
                                        tick >= since && probed_height == height
                                    },
                                )
                            });
                        }
                        if let Some(current) = entry.run.as_mut() {
                            current.probing = Some(Probing {
                                views: read.views,
                                height,
                                forced: read.forced,
                                due: start.due,
                            });
                        }
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
                root,
                verdict,
            } => {
                let Some(probing) = self
                    .wallets
                    .get_mut(&wallet_id)
                    .and_then(|entry| entry.run.as_mut())
                    .filter(|current| current.id == run)
                    .and_then(|current| current.probing.as_ref())
                else {
                    return;
                };
                let events = record_probe(
                    &mut self.state,
                    wallet_id,
                    probing.height,
                    &probing.views,
                    &probing.forced,
                    root,
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
        if let Some((root, transaction)) = probing.due.pop_front() {
            let probe = Arc::clone(&self.probe);
            let id = run.id;
            run.abort = self.jobs.spawn(async move {
                let verdict = probe.probe(&transaction).await;
                JobDone::Probed {
                    wallet_id,
                    run: id,
                    root,
                    verdict,
                }
            });
            self.job_runs
                .insert(run.abort.id(), (wallet_id, id, Some(root)));
            self.probed
                .insert((wallet_id, root), (self.tick, probing.height));
            return;
        }
        entry.wait_until =
            (!run.woken).then(|| next_pass_height(&self.state, wallet_id, &probing.views));
        entry.run = None;
        // A height queued during the run that no root is due at is dropped;
        // a forced request never is.
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
    commands: mpsc::UnboundedSender<Command>,
    /// The actor has been started (or the resolver closed): `send` no longer
    /// needs the slot.
    started: AtomicBool,
    actor: Mutex<ActorSlot>,
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
    /// stopped; `Timeout` if it did not end in time (it is then aborted, and
    /// a later call waits for it again); `Panicked` if it had died.
    pub(crate) async fn stop_within(&self, budget: Duration) -> WorkerStatus {
        self.started.store(true, Ordering::Release);
        stop_slot(&self.actor, budget).await
    }
}

/// See [`BroadcastResolver::stop_within`].
async fn stop_slot(actor: &Mutex<ActorSlot>, budget: Duration) -> WorkerStatus {
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
    match tokio::time::timeout(budget, &mut handle).await {
        Ok(Ok(())) => WorkerStatus::Ok,
        Ok(Err(error)) if error.is_panic() => WorkerStatus::Panicked(error.to_string()),
        Ok(Err(error)) => WorkerStatus::Stopped(Some(error.to_string())),
        Err(_) => {
            // An abort cannot interrupt a host callback in progress: keep
            // the handle so a retried stop waits for it again.
            handle.abort();
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
            transaction: tx(n, inputs),
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

    #[test]
    fn should_be_due_immediately_when_never_probed() {
        assert!(ProbeSchedule::new(100).is_due(100));
    }

    #[test]
    fn should_probe_at_most_once_per_block() {
        let mut s = ProbeSchedule::new(100);
        s.probed_at(100);
        assert!(!s.is_due(100));
        assert!(s.is_due(101));
    }

    #[test]
    fn should_slow_down_after_the_every_block_window() {
        let first = 100;
        let mut s = ProbeSchedule::new(first);
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
        let mut schedule = ProbeSchedule::new(first);
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
            let start = begin_pass(&mut guard.state, wallet_id, height, views, forced);
            guard.push(start.events);
            start.due
        };
        for (root, transaction) in due {
            let verdict = probe.probe(&transaction).await;
            let mut guard = state.lock().expect("state");
            let events = record_probe(
                &mut guard.state,
                wallet_id,
                height,
                views,
                forced,
                root,
                &verdict,
            );
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
            by_height: bool,
            forced: HashSet<Txid>,
        ) -> Option<Option<WalletRead>> {
            let views = self.views.lock().expect("views").get(&wallet_id)?.clone();
            let forced = forced_held(by_height, forced, |txid| {
                views.iter().any(|view| view.txid == *txid)
            });
            Some(forced.map(|forced| {
                self.walks.lock().expect("walks").push(wallet_id);
                WalletRead {
                    views,
                    synced_height: 0,
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
            self.actor.handle(command);
            self.settle().await;
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

        rig.actor.handle(Command::Height {
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

        rig.actor.handle(Command::Height {
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
        rig.actor.handle(Command::Height {
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

    /// An `Uncertain` result that lands while a height pass already probes the
    /// root at that height does not send it to the network a second time.
    #[tokio::test]
    async fn should_not_probe_a_root_twice_at_one_height_for_an_uncertain_result() {
        let probe = Arc::new(ScriptedProbe::new(&[(txid(1), unresolved())]));
        let mut rig = enabled(probe.clone()).await;
        rig.views(wallet(), vec![send(1, &[outpoint(90, 0)])]);
        rig.height(wallet(), 100).await;

        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 101,
        });
        rig.actor.handle(Command::Uncertain(txid(1)));
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(1)]);
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
        rig.actor.handle(Command::Height {
            wallet_id: wallet(),
            height: 103,
        });
        rig.settle().await;

        assert_eq!(probe.probed(), vec![txid(1), txid(2), txid(2)]);
    }

    /// A stop that runs out of budget keeps the task: a retried stop waits for
    /// it again and never reports it gone while it still runs.
    #[tokio::test]
    async fn should_not_report_a_task_that_outlived_a_stop_as_not_running() {
        let (stop, _stopped) = oneshot::channel();
        // A blocking task stands in for a host callback that abort cannot cut.
        let handle = tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(300)));
        let slot = Mutex::new(ActorSlot::Running(handle, stop));

        let first = stop_slot(&slot, Duration::from_millis(20)).await;
        let second = stop_slot(&slot, Duration::from_secs(5)).await;
        let third = stop_slot(&slot, Duration::from_secs(5)).await;

        assert_eq!(first, WorkerStatus::Timeout);
        assert_eq!(second, WorkerStatus::Ok);
        assert_eq!(third, WorkerStatus::NotRunning);
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
