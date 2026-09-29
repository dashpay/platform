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
//! cleared says only "forget the verdict", not that the send settled. Events
//! are queued under the state lock and delivered with no lock held.
//!
//! When: once as soon as dash-spv reports an `Uncertain` broadcast (the root
//! of that transaction's chain), then on each advance of the wallet's synced
//! height that follows the tip — steps of more than [`CATCH_UP_STEP`] blocks,
//! and the first step after launch, are catch-up and skipped — every block
//! for [`EVERY_BLOCK_WINDOW`] blocks and every [`SLOW_INTERVAL`] blocks after.
//! A wallet whose last pass had nothing left to probe is idle and not scanned
//! again until a wallet event that touched its records (a new or
//! InstantSend-locked transaction, a block or chainlock that changed records, a
//! sweep) or an `Uncertain` result for one of its transactions wakes it. One
//! worker per wallet drains a queue of merged requests. `Dead` and `Mined` end
//! the probing of a root; `Accepted` does not — a mempool is not settlement.
//! Probes run only while the SPV client delivers blocks, i.e. while the app is
//! in the foreground.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use dash_spv::sync::SyncEvent;
use dash_spv::{BroadcastResult, EventHandler};
use dashcore::{Transaction, Txid};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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

/// What a pass has to tell the host, per own send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolverEvent {
    /// A send's verdict changed.
    Verdict(Txid, ProbeVerdict),
    /// The host must drop the verdict it holds for a send: the send settled
    /// or left the wallet, its wallet was removed, or probing was turned off.
    /// Not a statement that the send settled.
    Cleared(Txid),
}

/// One event waiting to reach the host, with what the log line needs.
#[derive(Debug, Clone)]
struct Outgoing {
    wallet_id: WalletId,
    event: ResolverEvent,
    height: Option<u32>,
    trigger: &'static str,
}

/// Which clears and removals a pass started under. A pass compares it with
/// the current one under the state lock before recording anything: a wallet
/// removed, or probing turned off, since the pass read the wallet makes it
/// stop instead of rebuilding state that was just dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Generation {
    all: u64,
    wallet: u64,
}

/// Bookkeeping shared by every pass. Schedules and final verdicts are per
/// probed root; published verdicts are per own send, which is what the host
/// shows. Everything for a transaction is kept while it stays unsettled in the
/// wallet, and dropped once it settles or leaves.
///
/// Events for the host are queued in `outbox` under the same lock that records
/// them, so they reach the host in the order the state changed; they are
/// delivered afterwards with no lock held (see [`deliver`]).
#[derive(Debug, Default)]
pub(crate) struct ResolverState {
    schedules: HashMap<(WalletId, Txid), ProbeSchedule>,
    finished: HashMap<(WalletId, Txid), Final>,
    published: HashMap<(WalletId, Txid), ProbeVerdict>,
    generation_all: u64,
    /// Kept for removed wallets too: a wallet re-added under the same id must
    /// not match a generation read before its removal.
    generations: HashMap<WalletId, u64>,
    /// Probing is off. Set under this lock together with the clears, so a
    /// pass started after probing was turned off stops as surely as one that
    /// started before.
    off: bool,
    outbox: VecDeque<Outgoing>,
}

impl ResolverState {
    pub(crate) fn generation(&self, wallet_id: &WalletId) -> Generation {
        Generation {
            all: self.generation_all,
            wallet: self.generations.get(wallet_id).copied().unwrap_or(0),
        }
    }

    /// Whether a pass started under `started` may still probe and record.
    fn is_current(&self, wallet_id: &WalletId, started: Generation) -> bool {
        !self.off && self.generation(wallet_id) == started
    }

    fn enqueue(
        &mut self,
        wallet_id: WalletId,
        event: ResolverEvent,
        height: Option<u32>,
        trigger: &'static str,
    ) {
        self.outbox.push_back(Outgoing {
            wallet_id,
            event,
            height,
            trigger,
        });
    }

    /// Drop everything kept for `wallet_id` and queue a clear for every send
    /// the host had a verdict for. Passes that started before are stopped.
    fn forget_wallet(&mut self, wallet_id: &WalletId) {
        *self.generations.entry(*wallet_id).or_insert(0) += 1;
        self.schedules.retain(|(wallet, _), _| wallet != wallet_id);
        self.finished.retain(|(wallet, _), _| wallet != wallet_id);
        let mut cleared: Vec<Txid> = self
            .published
            .keys()
            .filter(|(wallet, _)| wallet == wallet_id)
            .map(|(_, txid)| *txid)
            .collect();
        cleared.sort();
        self.published.retain(|(wallet, _), _| wallet != wallet_id);
        for txid in cleared {
            self.enqueue(*wallet_id, ResolverEvent::Cleared(txid), None, "removed");
        }
    }

    /// Drop everything and queue a clear for every published send. Passes
    /// that started before are stopped.
    fn forget_all(&mut self) {
        self.generation_all += 1;
        self.schedules.clear();
        self.finished.clear();
        let mut cleared: Vec<(WalletId, Txid)> =
            self.published.drain().map(|(key, _)| key).collect();
        cleared.sort();
        for (wallet_id, txid) in cleared {
            self.enqueue(wallet_id, ResolverEvent::Cleared(txid), None, "disabled");
        }
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

    fn take_outbox(&mut self) -> Vec<Outgoing> {
        self.outbox.drain(..).collect()
    }
}

/// The first advance after launch has no previous height to compare with, and
/// it is usually the stored tip from before the scan: treat it as catch-up
/// too. Probing starts from the next step.
pub(crate) fn is_catch_up_step(previous: Option<u32>, height: u32) -> bool {
    previous.is_none_or(|previous| height.saturating_sub(previous) > CATCH_UP_STEP)
}

/// What a pass leaves behind besides the events it queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassOutcome {
    /// Nothing left to probe: no roots, or every root has a final verdict.
    /// The wallet can go idle until something new happens.
    pub nothing_to_probe: bool,
}

/// One pass over one wallet at `height`: probe every due root, record and
/// queue what the host must hear. The roots of every `forced` transaction's
/// chain are due regardless of their schedule (dash-spv has just reported it
/// `Uncertain`). `started` is the [`Generation`] the caller read the wallet
/// under; once it is no longer current (wallet removed, probing turned off),
/// the pass records nothing more and sends nothing more to the network.
/// `flush` delivers what is queued so far; the pass calls it with no lock
/// held after each locked stretch, so a clear or a verdict does not wait for
/// the probes of other roots.
///
/// A root's verdict is published for the wallet's own sends: the root itself
/// when it is one, and — when the root is dead — every own send built on it,
/// on every pass, so a send built on a dead root's change later is covered
/// too. A root found mined counts as settled when picking roots, so its
/// children are probed next.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn resolve_pass(
    state: &Mutex<ResolverState>,
    probe: &dyn AcceptanceProbe,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
    started: Generation,
    flush: &(dyn Fn() + Sync),
) -> PassOutcome {
    let trigger = if forced.is_empty() {
        "height"
    } else {
        "uncertain"
    };
    // What the wallet itself still holds unsettled — what state is kept for.
    let present: HashSet<Txid> = views.iter().map(|view| view.txid).collect();
    let own_present: HashSet<Txid> = views
        .iter()
        .filter(|view| view.spends_own_coins)
        .map(|view| view.txid)
        .collect();
    let stopped = PassOutcome {
        nothing_to_probe: false,
    };

    // Each locked stretch is its own block: a guard must not live across the
    // probes' awaits.
    let mined: HashSet<Txid> = {
        let mut guard = state.lock().expect("resolver state poisoned");
        if !guard.is_current(&wallet_id, started) {
            return stopped;
        }
        // Forget what settled or left the wallet; clear the sends the host had a
        // verdict for.
        let is_gone =
            |wallet: &WalletId, txid: &Txid| *wallet == wallet_id && !present.contains(txid);
        guard
            .schedules
            .retain(|(wallet, txid), _| !is_gone(wallet, txid));
        guard
            .finished
            .retain(|(wallet, txid), _| !is_gone(wallet, txid));
        let mut cleared: Vec<Txid> = guard
            .published
            .keys()
            .filter(|(wallet, txid)| *wallet == wallet_id && !own_present.contains(txid))
            .map(|(_, txid)| *txid)
            .collect();
        cleared.sort();
        guard
            .published
            .retain(|(wallet, txid), _| *wallet != wallet_id || own_present.contains(txid));
        for txid in cleared {
            guard.enqueue(
                wallet_id,
                ResolverEvent::Cleared(txid),
                Some(height),
                trigger,
            );
        }

        guard
            .finished
            .iter()
            .filter(|((wallet, _), result)| *wallet == wallet_id && **result == Final::Mined)
            .map(|((_, txid), _)| *txid)
            .collect()
    };
    let graph = ChainGraph::new(views, &mined);
    let roots = graph.roots();
    let forced_roots: BTreeSet<Txid> = forced
        .iter()
        .flat_map(|txid| graph.roots_of(txid))
        .collect();

    let due = {
        let mut guard = state.lock().expect("resolver state poisoned");
        if !guard.is_current(&wallet_id, started) {
            return stopped;
        }
        // A dead root takes every own send built on it — including ones built on
        // its change after it was found dead.
        for view in &roots {
            if let Some(Final::Dead { reason }) =
                guard.finished.get(&(wallet_id, view.txid)).cloned()
            {
                let verdict = ProbeVerdict::Dead { reason };
                for send in graph.own_in_chain(&view.txid) {
                    if guard.publish((wallet_id, send), &verdict) {
                        guard.enqueue(
                            wallet_id,
                            ResolverEvent::Verdict(send, verdict.clone()),
                            Some(height),
                            trigger,
                        );
                    }
                }
            }
        }

        let mut due = Vec::new();
        for view in &roots {
            let key = (wallet_id, view.txid);
            if guard.finished.contains_key(&key) {
                continue;
            }
            let schedule = guard
                .schedules
                .entry(key)
                .or_insert_with(|| ProbeSchedule::new(height));
            if forced_roots.contains(&view.txid) || schedule.is_due(height) {
                schedule.probed_at(height);
                due.push((view.txid, view.transaction.clone()));
            }
        }
        due
    };
    flush();

    for (root, transaction) in due {
        if !state
            .lock()
            .expect("resolver state poisoned")
            .is_current(&wallet_id, started)
        {
            return stopped;
        }
        let verdict = probe.probe(&transaction).await;
        let key = (wallet_id, root);
        {
            let mut guard = state.lock().expect("resolver state poisoned");
            if !guard.is_current(&wallet_id, started) {
                return stopped;
            }
            // Dropped by a newer pass meanwhile (it settled or left): recording
            // now would resurrect state that was cleared.
            if !guard.schedules.contains_key(&key) {
                continue;
            }
            match &verdict {
                ProbeVerdict::Dead { reason } => {
                    guard.finished.insert(
                        key,
                        Final::Dead {
                            reason: reason.clone(),
                        },
                    );
                }
                ProbeVerdict::Mined => {
                    guard.finished.insert(key, Final::Mined);
                }
                _ => {}
            }
            let sends: BTreeSet<Txid> = if matches!(verdict, ProbeVerdict::Dead { .. }) {
                graph.own_in_chain(&root)
            } else if graph.is_own(&root) {
                BTreeSet::from([root])
            } else {
                BTreeSet::new()
            };
            for send in sends {
                if guard.publish((wallet_id, send), &verdict) {
                    guard.enqueue(
                        wallet_id,
                        ResolverEvent::Verdict(send, verdict.clone()),
                        Some(height),
                        trigger,
                    );
                }
            }
        }
        flush();
    }

    let guard = state.lock().expect("resolver state poisoned");
    let nothing_to_probe = roots
        .iter()
        .all(|view| guard.finished.contains_key(&(wallet_id, view.txid)));
    PassOutcome { nothing_to_probe }
}

/// Deliver queued events to the host, in order, with no lock held while a
/// host callback runs — a callback may turn probing off or remove a wallet,
/// which queues more events and returns at once; the running delivery sends
/// those too. `delivering` makes sure only one thread delivers at a time.
fn deliver(
    state: &Mutex<ResolverState>,
    delivering: &AtomicBool,
    events: &Weak<PlatformEventManager>,
) {
    loop {
        if delivering.swap(true, Ordering::SeqCst) {
            return; // the thread already delivering will send ours
        }
        let events = events.upgrade();
        loop {
            let batch = state.lock().expect("resolver state poisoned").take_outbox();
            if batch.is_empty() {
                break;
            }
            for item in batch {
                log_outgoing(&item);
                if let Some(events) = &events {
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
        }
        delivering.store(false, Ordering::SeqCst);
        // Something queued between the last empty check and the flag reset
        // would otherwise wait for the next delivery.
        if state
            .lock()
            .expect("resolver state poisoned")
            .outbox
            .is_empty()
        {
            return;
        }
    }
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

/// Admission gate for probe tasks. A probe only reads the wallet and
/// resubmits bytes that were already sent, so a task may be aborted at any
/// point without losing anything.
struct ProbeTasks {
    state: Mutex<(bool, Vec<JoinHandle<()>>)>,
}

impl ProbeTasks {
    fn new() -> Self {
        Self {
            state: Mutex::new((true, Vec::new())),
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut state = self.state.lock().expect("probe task mutex poisoned");
        if !state.0 {
            return;
        }
        state.1.retain(|handle| !handle.is_finished());
        state.1.push(tokio::spawn(future));
    }

    /// Close admission and stop every task, waiting at most `budget` for each
    /// aborted task to unwind. Returns `false` if one outlived it.
    async fn quiesce_within(&self, budget: Duration) -> bool {
        let handles = {
            let mut state = self.state.lock().expect("probe task mutex poisoned");
            state.0 = false;
            std::mem::take(&mut state.1)
        };
        let deadline = tokio::time::Instant::now() + budget;
        let mut clean = true;
        for handle in &handles {
            handle.abort();
        }
        for mut handle in handles {
            if tokio::time::timeout_at(deadline, &mut handle)
                .await
                .is_err()
            {
                clean = false;
            }
        }
        clean
    }
}

/// Which wallets have a pass running, what the next pass for each must do,
/// and which have nothing to probe. A pass for a wallet never runs twice at
/// once; requests that arrive while one runs are merged into the next.
#[derive(Default)]
struct PassQueue {
    running: HashSet<WalletId>,
    pending: HashMap<WalletId, PendingPass>,
    /// Wallets whose last pass had nothing to probe: height advances skip
    /// them (no scan of every record per block) until something wakes them.
    idle: HashSet<WalletId>,
    /// Bumped by every wake-up, so a pass that read the wallet before one
    /// happened does not mark it idle after it.
    wakes: HashMap<WalletId, u64>,
}

impl PassQueue {
    fn wake(&mut self, wallet_id: &WalletId) {
        self.idle.remove(wallet_id);
        *self.wakes.entry(*wallet_id).or_insert(0) += 1;
    }

    fn wakes(&self, wallet_id: &WalletId) -> u64 {
        self.wakes.get(wallet_id).copied().unwrap_or(0)
    }
}

#[derive(Default)]
struct PendingPass {
    height: u32,
    /// A height advance asked for this pass; otherwise only `Uncertain`
    /// results did, and a wallet that holds none of `forced` is skipped.
    by_height: bool,
    forced: HashSet<Txid>,
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

/// Runs probe passes off the wallet-event fan-out and delivers each change of
/// a send's verdict as [`PlatformEventHandler::on_outgoing_transaction_probed`],
/// and each verdict the host must drop as
/// [`PlatformEventHandler::on_outgoing_transaction_cleared`].
///
/// Off until the host turns it on with [`set_enabled`](Self::set_enabled): a
/// probe sends the signed transaction to evonodes over DAPI, which is a
/// product decision, not something a wallet should start doing on upgrade.
pub(crate) struct BroadcastResolver {
    enabled: AtomicBool,
    probe: Arc<dyn AcceptanceProbe>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    state: Arc<Mutex<ResolverState>>,
    delivering: Arc<AtomicBool>,
    /// Every wallet seen in an event, with its last synced height if one has
    /// been reported: an SPV-level `Uncertain` carries no wallet id, so it is
    /// queued for every known wallet. A worker that finds its wallet gone
    /// drops the entry, so a late event for a removed wallet cannot keep it.
    wallets: Arc<Mutex<HashMap<WalletId, Option<u32>>>>,
    passes: Arc<Mutex<PassQueue>>,
    events: Mutex<Weak<PlatformEventManager>>,
    tasks: ProbeTasks,
}

impl BroadcastResolver {
    pub(crate) fn new(
        probe: Arc<dyn AcceptanceProbe>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    ) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            probe,
            wallet_manager,
            state: Arc::new(Mutex::new(ResolverState {
                off: true,
                ..ResolverState::default()
            })),
            delivering: Arc::new(AtomicBool::new(false)),
            wallets: Arc::new(Mutex::new(HashMap::new())),
            passes: Arc::new(Mutex::new(PassQueue::default())),
            events: Mutex::new(Weak::new()),
            tasks: ProbeTasks::new(),
        }
    }

    /// Where verdicts are delivered. Weak: the event manager holds this
    /// resolver as one of its handlers.
    pub(crate) fn set_event_manager(&self, events: Weak<PlatformEventManager>) {
        *self.events.lock().expect("events mutex poisoned") = events;
    }

    fn deliver(&self) {
        let events = self.events.lock().expect("events mutex poisoned").clone();
        deliver(&self.state, &self.delivering, &events);
    }

    /// Turning probing off forgets every send and tells the host to drop every
    /// verdict it holds: with no passes running, nothing would ever clear them.
    /// Passes still running, and ones already queued, stop before their next
    /// probe and record nothing. Either way every wallet leaves idle, so
    /// turning probing back on re-publishes what was cleared.
    ///
    /// The clears are delivered on the calling thread before this returns.
    pub(crate) fn set_enabled(&self, enabled: bool) {
        let changed = {
            let mut state = self.state.lock().expect("resolver state poisoned");
            let was = self.enabled.swap(enabled, Ordering::SeqCst);
            state.off = !enabled;
            if was && !enabled {
                state.forget_all();
            }
            was != enabled
        };
        if changed {
            let mut passes = self.passes.lock().expect("pass queue poisoned");
            passes.idle.clear();
            if !enabled {
                passes.pending.clear();
            }
        }
        self.deliver();
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Forget a removed wallet — call after it has left the wallet manager:
    /// its queued pass, its entry and every verdict, each of which the host is
    /// told to drop. A pass that read the wallet before the removal stops
    /// before recording anything.
    pub(crate) fn wallet_removed(&self, wallet_id: &WalletId) {
        {
            let mut passes = self.passes.lock().expect("pass queue poisoned");
            passes.pending.remove(wallet_id);
            // A wake, not a removal of the counter: a pass still running for
            // the wallet must see it changed and not mark the wallet idle.
            passes.wake(wallet_id);
        }
        self.wallets
            .lock()
            .expect("wallets mutex poisoned")
            .remove(wallet_id);
        self.state
            .lock()
            .expect("resolver state poisoned")
            .forget_wallet(wallet_id);
        self.deliver();
    }

    /// Stop admitting passes and stop the running ones. Idempotent.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        self.tasks.quiesce_within(budget).await
    }

    /// Queue a pass for `wallet_id`; start a worker for it unless one runs.
    /// A height-driven request (no `force`) is dropped for an idle wallet; a
    /// forced one is queued, and the worker skips it cheaply if the wallet
    /// does not hold the transaction.
    fn request_pass(&self, wallet_id: WalletId, height: u32, force: Option<Txid>) {
        {
            let mut passes = self.passes.lock().expect("pass queue poisoned");
            if force.is_none() && passes.idle.contains(&wallet_id) {
                return;
            }
            let pending = passes.pending.entry(wallet_id).or_default();
            pending.height = pending.height.max(height);
            pending.by_height |= force.is_none();
            pending.forced.extend(force);
            if !passes.running.insert(wallet_id) {
                return; // the running worker picks it up
            }
        }
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let probe = Arc::clone(&self.probe);
        let state = Arc::clone(&self.state);
        let passes = Arc::clone(&self.passes);
        let wallets = Arc::clone(&self.wallets);
        let delivering = Arc::clone(&self.delivering);
        let events = self.events.lock().expect("events mutex poisoned").clone();
        self.tasks.spawn(async move {
            let mut running = Running {
                passes: Arc::clone(&passes),
                wallet_id,
                armed: true,
            };
            let flush = || deliver(&state, &delivering, &events);
            loop {
                // Take the next request, or stop — under the same lock a
                // request_pass would take, so a request can never land between
                // "queue empty" and "no longer running" and be left unserved.
                let (next, wakes) = {
                    let mut queue = passes.lock().expect("pass queue poisoned");
                    let next = queue.pending.remove(&wallet_id);
                    if next.is_none() {
                        queue.running.remove(&wallet_id);
                    }
                    (next, queue.wakes(&wallet_id))
                };
                let Some(PendingPass {
                    height,
                    by_height,
                    forced,
                }) = next
                else {
                    // Already left the running set under the lock; the guard
                    // must not remove a marker a new worker may have set since.
                    running.armed = false;
                    break;
                };
                let started = state
                    .lock()
                    .expect("resolver state poisoned")
                    .generation(&wallet_id);
                // Read under the lock, probe without it: a probe is network I/O.
                let read = {
                    let manager = wallet_manager.read().await;
                    manager.get_wallet_info(&wallet_id).map(|info| {
                        // An `Uncertain` result is queued for every wallet;
                        // only the one holding the transaction acts on it.
                        let forced: HashSet<Txid> = forced
                            .into_iter()
                            .filter(|txid| holds(info, txid))
                            .collect();
                        if !by_height && forced.is_empty() {
                            return None;
                        }
                        // A request queued before any height was reported
                        // carries 0; the wallet's own synced height is better.
                        let height = height.max(info.core_wallet.synced_height());
                        Some((collect_views(info), height, forced))
                    })
                };
                let (views, height, forced) = match read {
                    Some(Some(pass)) => pass,
                    Some(None) => continue, // not this wallet's transaction
                    None => {
                        // Removed (wallet_removed did the clearing), or an id a
                        // late event brought back after the removal.
                        wallets
                            .lock()
                            .expect("wallets mutex poisoned")
                            .remove(&wallet_id);
                        let mut queue = passes.lock().expect("pass queue poisoned");
                        queue.idle.remove(&wallet_id);
                        queue.wakes.remove(&wallet_id);
                        continue;
                    }
                };
                if !forced.is_empty() {
                    passes
                        .lock()
                        .expect("pass queue poisoned")
                        .idle
                        .remove(&wallet_id);
                }
                let outcome = resolve_pass(
                    &state,
                    probe.as_ref(),
                    wallet_id,
                    height,
                    &views,
                    &forced,
                    started,
                    &flush,
                )
                .await;
                if outcome.nothing_to_probe {
                    let mut queue = passes.lock().expect("pass queue poisoned");
                    if queue.wakes(&wallet_id) == wakes {
                        queue.idle.insert(wallet_id);
                    }
                }
                flush();
            }
        });
    }
}

/// A handle that turns automatic probing on or off, usable after the lock that
/// guards the manager is released: turning probing off delivers a clear per
/// published send to the host on the calling thread.
#[derive(Clone)]
pub struct BroadcastProbeSwitch(pub(crate) Arc<BroadcastResolver>);

impl BroadcastProbeSwitch {
    /// See [`PlatformWalletManager::set_broadcast_probe_enabled`](crate::PlatformWalletManager::set_broadcast_probe_enabled).
    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled);
    }
}

/// Takes a wallet's worker out of the running set if it ends without reaching
/// its own exit — an abort at shutdown — so a later request is not refused
/// forever. Disarmed on the normal exit, which leaves the set under the queue
/// lock itself.
struct Running {
    passes: Arc<Mutex<PassQueue>>,
    wallet_id: WalletId,
    armed: bool,
}

impl Drop for Running {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(mut passes) = self.passes.lock() {
            passes.running.remove(&self.wallet_id);
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
        if let WalletEvent::SyncHeightAdvanced { height, .. } = event {
            let previous = self
                .wallets
                .lock()
                .expect("wallets mutex poisoned")
                .insert(wallet_id, Some(*height))
                .flatten();
            if self.is_enabled() && !is_catch_up_step(previous, *height) {
                self.request_pass(wallet_id, *height, None);
            }
            return;
        }
        self.wallets
            .lock()
            .expect("wallets mutex poisoned")
            .entry(wallet_id)
            .or_insert(None);
        if may_leave_work(event) {
            self.passes
                .lock()
                .expect("pass queue poisoned")
                .wake(&wallet_id);
        }
    }

    fn on_sync_event(&self, event: &SyncEvent) {
        let SyncEvent::TransactionBroadcastResult {
            txid,
            result: BroadcastResult::Uncertain,
        } = event
        else {
            return;
        };
        if !self.is_enabled() {
            return;
        }
        let wallets: Vec<(WalletId, u32)> = self
            .wallets
            .lock()
            .expect("wallets mutex poisoned")
            .iter()
            .map(|(wallet, height)| (*wallet, height.unwrap_or(0)))
            .collect();
        for (wallet_id, height) in wallets {
            self.request_pass(wallet_id, height, Some(*txid));
        }
    }
}

impl PlatformEventHandler for BroadcastResolver {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, TxIn};

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

    // ---- resolve_pass -------------------------------------------------------

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

    async fn pass(
        state: &Mutex<ResolverState>,
        probe: &dyn AcceptanceProbe,
        height: u32,
        views: &[OutgoingView],
    ) -> Vec<ResolverEvent> {
        run(state, probe, wallet(), height, views, &none()).await;
        outbox(state)
    }

    /// A pass started under the current generation, as the worker runs it.
    async fn run(
        state: &Mutex<ResolverState>,
        probe: &dyn AcceptanceProbe,
        wallet_id: WalletId,
        height: u32,
        views: &[OutgoingView],
        forced: &HashSet<Txid>,
    ) -> PassOutcome {
        let started = state.lock().expect("state").generation(&wallet_id);
        resolve_pass(
            state,
            probe,
            wallet_id,
            height,
            views,
            forced,
            started,
            &|| {},
        )
        .await
    }

    fn outbox(state: &Mutex<ResolverState>) -> Vec<ResolverEvent> {
        state
            .lock()
            .expect("state")
            .take_outbox()
            .into_iter()
            .map(|item| item.event)
            .collect()
    }

    #[tokio::test]
    async fn should_probe_only_roots_and_report_each_verdict() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = pass(&state, &probe, 100, &views).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(1), unresolved())]);
    }

    #[tokio::test]
    async fn should_stop_probing_a_root_proven_dead() {
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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

    /// Probing turned off (or the wallet removed) after the worker read the
    /// wallet: the pass sends nothing to the network and records nothing.
    #[tokio::test]
    async fn should_do_nothing_in_a_pass_started_before_a_clear() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(91, 0)])];
        let started = state.lock().expect("state").generation(&wallet());
        state.lock().expect("state").forget_all();

        resolve_pass(
            &state,
            &probe,
            wallet(),
            100,
            &views,
            &none(),
            started,
            &|| {},
        )
        .await;

        assert!(probe.probed().is_empty());
        assert!(outbox(&state).is_empty());
        assert!(state.lock().expect("state").schedules.is_empty());
    }

    /// A send that settled or left the wallet is forgotten, and the host is
    /// told to drop the verdict it was shown.
    #[tokio::test]
    async fn should_clear_a_published_send_that_is_no_longer_unconfirmed() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        let gone = pass(&state, &probe, 101, &[]).await;

        assert_eq!(gone, vec![ResolverEvent::Cleared(txid(1))]);
        let guard = state.lock().expect("state");
        assert!(guard.schedules.is_empty());
        assert!(guard.finished.is_empty());
        assert!(guard.published.is_empty());
    }

    #[tokio::test]
    async fn should_not_let_one_wallets_pass_forget_another_wallets_roots() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        pass(&state, &probe, 100, &views).await;
        run(&state, &probe, [9u8; 32], 100, &[], &none()).await;

        assert_eq!(state.lock().expect("state").schedules.len(), 1);
    }

    /// A mined transaction the wallet has not caught up with yet: nothing left
    /// to ask, so it is not probed again.
    #[tokio::test]
    async fn should_stop_probing_a_root_found_in_a_block() {
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(5), dead())]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        let events = pass(&state, &probe, 100, &views).await;

        assert_eq!(probe.probed(), vec![txid(5)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(6), dead())]);
    }

    #[tokio::test]
    async fn should_publish_nothing_while_an_incoming_parent_is_only_accepted() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(5), ProbeVerdict::Accepted)]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        assert!(pass(&state, &probe, 100, &views).await.is_empty());
    }

    /// A root found in a block that the wallet has not caught up with is
    /// settled for picking roots, so the send built on it is probed next.
    #[tokio::test]
    async fn should_probe_the_children_of_a_root_found_in_a_block() {
        let state = Mutex::new(ResolverState::default());
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

    /// A probe that, while it is out on the network, lets a concurrent pass
    /// clear the root (it settled): the late verdict must not be published.
    struct ClearingProbe {
        state: Arc<Mutex<ResolverState>>,
    }

    #[async_trait]
    impl AcceptanceProbe for ClearingProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
            self.state.lock().expect("state").forget_wallet(&wallet());
            ProbeVerdict::Accepted
        }
    }

    #[tokio::test]
    async fn should_not_publish_a_verdict_for_a_root_cleared_during_the_probe() {
        let state = Arc::new(Mutex::new(ResolverState::default()));
        let probe = ClearingProbe {
            state: Arc::clone(&state),
        };

        run(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &none(),
        )
        .await;

        assert!(outbox(&state).is_empty());
        assert!(state.lock().expect("state").published.is_empty());
    }

    /// A send built on a dead root's change after the root was found dead is
    /// dead too, and hears it without the root being asked again.
    #[tokio::test]
    async fn should_publish_a_dead_root_to_a_send_built_on_it_later() {
        let state = Mutex::new(ResolverState::default());
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
        let state = Mutex::new(ResolverState::default());
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

        assert!(empty.nothing_to_probe);
        assert!(dead_only.nothing_to_probe);
        assert!(!open.nothing_to_probe);
    }

    // ---- ResolverState ------------------------------------------------------

    #[tokio::test]
    async fn should_forget_one_wallet_and_queue_a_clear_for_its_published_sends() {
        let state = Mutex::new(ResolverState::default());
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
        assert_eq!(guard.published.len(), 1, "the other wallet is untouched");
        assert_eq!(guard.schedules.len(), 1);
    }

    #[tokio::test]
    async fn should_forget_everything_and_queue_a_clear_for_every_published_send() {
        let state = Mutex::new(ResolverState::default());
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
            guard.published.is_empty() && guard.schedules.is_empty() && guard.finished.is_empty()
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

    /// Probing off: a pass that read its generation after the switch stops
    /// as surely as one that started before it.
    #[tokio::test]
    async fn should_do_nothing_while_probing_is_off() {
        let state = Mutex::new(ResolverState {
            off: true,
            ..ResolverState::default()
        });
        let probe = ScriptedProbe::new(&[]);

        run(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &none(),
        )
        .await;

        assert!(probe.probed().is_empty());
        assert!(outbox(&state).is_empty());
        assert!(state.lock().expect("state").schedules.is_empty());
    }

    /// A probe that records what the host had been told by the time it ran.
    struct SnapshotProbe {
        delivered: Arc<StdMutex<Vec<ResolverEvent>>>,
        seen: StdMutex<Vec<ResolverEvent>>,
    }

    #[async_trait]
    impl AcceptanceProbe for SnapshotProbe {
        async fn probe(&self, _transaction: &Transaction) -> ProbeVerdict {
            *self.seen.lock().expect("seen") = self.delivered.lock().expect("delivered").clone();
            unresolved()
        }
    }

    /// A clear reaches the host before the pass goes out to the network, not
    /// after every root's probe.
    #[tokio::test]
    async fn should_deliver_clears_before_probing() {
        let state = Mutex::new(ResolverState::default());
        pass(
            &state,
            &ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]),
            100,
            &[send(1, &[outpoint(90, 0)])],
        )
        .await;
        let delivered = Arc::new(StdMutex::new(Vec::new()));
        let probe = SnapshotProbe {
            delivered: Arc::clone(&delivered),
            seen: StdMutex::new(Vec::new()),
        };
        let flush = || {
            let batch = outbox(&state);
            delivered.lock().expect("delivered").extend(batch);
        };
        let started = state.lock().expect("state").generation(&wallet());

        resolve_pass(
            &state,
            &probe,
            wallet(),
            101,
            &[send(2, &[outpoint(91, 0)])],
            &none(),
            started,
            &flush,
        )
        .await;

        assert_eq!(
            *probe.seen.lock().expect("seen"),
            vec![ResolverEvent::Cleared(txid(1))]
        );
        assert_eq!(
            *delivered.lock().expect("delivered"),
            vec![
                ResolverEvent::Cleared(txid(1)),
                ResolverEvent::Verdict(txid(2), unresolved()),
            ]
        );
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
}
