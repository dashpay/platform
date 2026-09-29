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
//! transaction is dead too). A root found in a block that the wallet has not
//! seen yet counts as settled, so the sends built on it are probed next.
//! Only changes are published — Unresolved never replaces Accepted or Mined —
//! and a send that settles or leaves the wallet after a verdict is published
//! as cleared, as are all of a removed wallet's and, when probing is turned
//! off, all of them.
//!
//! When: once as soon as dash-spv reports an `Uncertain` broadcast, then on
//! each advance of the wallet's synced height that follows the tip (steps of
//! more than [`CATCH_UP_STEP`] blocks, and the first step after launch, are
//! catch-up and skipped), every block for [`EVERY_BLOCK_WINDOW`] blocks and
//! every [`SLOW_INTERVAL`] blocks after. One worker per wallet drains a
//! queue of merged requests. `Dead` and `Mined` end the probing of a root;
//! `Accepted` does not — a mempool is not settlement. Probes run only while
//! the SPV client delivers blocks, i.e. while the app is in the foreground.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use dash_spv::sync::SyncEvent;
use dash_spv::{BroadcastResult, EventHandler};
use dashcore::{OutPoint, Transaction, Txid};
use key_wallet::transaction_checking::TransactionContext;
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

/// What the resolver needs to know about one recorded transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutgoingView {
    pub txid: Txid,
    pub inputs: Vec<OutPoint>,
    /// In a block, InstantSend-locked, or finalized.
    pub settled: bool,
    /// The transaction spends at least one of this wallet's coins, i.e. the
    /// wallet signed it.
    pub spends_own_coins: bool,
    pub transaction: Transaction,
}

/// This wallet's unsettled transactions as chains.
///
/// Built from the account views, merged per transaction: settled if any
/// account holds it settled (or the caller knows it is mined), the wallet's own
/// send if any account records it spending the wallet's coins — a transfer
/// between accounts is "incoming" in the receiving one.
pub(crate) struct ChainGraph<'a> {
    by_txid: BTreeMap<Txid, (&'a OutgoingView, bool)>,
    unsettled: BTreeSet<Txid>,
}

impl<'a> ChainGraph<'a> {
    /// `mined` are transactions a probe found in a block that this wallet has
    /// not seen yet: settled for the purpose of picking roots, so the sends
    /// built on them become roots in turn.
    pub(crate) fn new(views: &'a [OutgoingView], mined: &HashSet<Txid>) -> Self {
        let mut merged: BTreeMap<Txid, (&OutgoingView, bool, bool)> = BTreeMap::new();
        for view in views {
            merged
                .entry(view.txid)
                .and_modify(|(_, settled, own)| {
                    *settled |= view.settled;
                    *own |= view.spends_own_coins;
                })
                .or_insert((view, view.settled, view.spends_own_coins));
        }
        let unsettled = merged
            .iter()
            .filter(|(txid, (_, settled, _))| !*settled && !mined.contains(*txid))
            .map(|(txid, _)| *txid)
            .collect();
        let by_txid = merged
            .into_iter()
            .map(|(txid, (view, _, own))| (txid, (view, own)))
            .collect();
        Self { by_txid, unsettled }
    }

    fn is_own(&self, txid: &Txid) -> bool {
        self.by_txid.get(txid).is_some_and(|(_, own)| *own)
    }

    /// The wallet's unsettled sends.
    pub(crate) fn own_unsettled(&self) -> BTreeSet<Txid> {
        self.unsettled
            .iter()
            .filter(|txid| self.is_own(txid))
            .copied()
            .collect()
    }

    /// The roots to probe: members of the chains the wallet signed into — its
    /// unsettled sends plus every unsettled ancestor they depend on, including
    /// someone else's incoming payment — that spend no output of an unsettled
    /// transaction. A node that has not seen a parent refuses a valid child.
    pub(crate) fn roots(&self) -> Vec<&'a OutgoingView> {
        let mut relevant = self.own_unsettled();
        let mut frontier: Vec<Txid> = relevant.iter().copied().collect();
        while let Some(txid) = frontier.pop() {
            for input in &self.by_txid[&txid].0.inputs {
                if self.unsettled.contains(&input.txid) && relevant.insert(input.txid) {
                    frontier.push(input.txid);
                }
            }
        }
        relevant
            .iter()
            .map(|txid| self.by_txid[txid].0)
            .filter(|view| {
                !view
                    .inputs
                    .iter()
                    .any(|input| self.unsettled.contains(&input.txid))
            })
            .collect()
    }

    /// The wallet's own sends in `root`'s chain: `root` itself if it is one,
    /// and every unsettled send built on it, transitively. A dead root takes
    /// all of them with it.
    pub(crate) fn own_in_chain(&self, root: &Txid) -> BTreeSet<Txid> {
        let mut chain = BTreeSet::from([*root]);
        loop {
            let before = chain.len();
            for txid in &self.unsettled {
                if !chain.contains(txid)
                    && self.by_txid[txid]
                        .0
                        .inputs
                        .iter()
                        .any(|input| chain.contains(&input.txid))
                {
                    chain.insert(*txid);
                }
            }
            if chain.len() == before {
                break;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Final {
    Dead,
    Mined,
}

/// Bookkeeping shared by every pass. Schedules and final verdicts are per
/// probed root; published verdicts are per own send, which is what the host
/// shows. Everything for a transaction is kept while it stays unsettled in the
/// wallet, and dropped once it settles or leaves.
#[derive(Debug, Default)]
pub(crate) struct ResolverState {
    schedules: HashMap<(WalletId, Txid), ProbeSchedule>,
    finished: HashMap<(WalletId, Txid), Final>,
    published: HashMap<(WalletId, Txid), ProbeVerdict>,
}

/// What a pass has to tell the host, per own send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolverEvent {
    /// A send's verdict changed.
    Verdict(Txid, ProbeVerdict),
    /// A send that had a published verdict settled or left the wallet.
    Cleared(Txid),
}

impl ResolverState {
    /// Drop everything kept for `wallet_id`; returns the sends the host had a
    /// verdict for, which it must now be told are cleared.
    fn forget_wallet(&mut self, wallet_id: &WalletId) -> Vec<Txid> {
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
        cleared
    }

    /// Drop everything; returns every send the host had a verdict for.
    fn forget_all(&mut self) -> Vec<(WalletId, Txid)> {
        self.schedules.clear();
        self.finished.clear();
        let mut cleared: Vec<(WalletId, Txid)> =
            self.published.drain().map(|(key, _)| key).collect();
        cleared.sort();
        cleared
    }

    /// Record `verdict` for `send`; returns whether the host must hear it.
    /// Unresolved never replaces an Accepted or Mined verdict — one probe that
    /// only met transport errors says nothing new — and Unresolved reasons
    /// vary between probes, so they all count as one.
    fn publish(&mut self, key: (WalletId, Txid), verdict: &ProbeVerdict) -> bool {
        let changed = match self.published.get(&key) {
            None => true,
            Some(ProbeVerdict::Unresolved { .. }) => {
                !matches!(verdict, ProbeVerdict::Unresolved { .. })
            }
            Some(ProbeVerdict::Accepted | ProbeVerdict::Mined)
                if matches!(verdict, ProbeVerdict::Unresolved { .. }) =>
            {
                false
            }
            Some(previous) => previous != verdict,
        };
        if changed {
            self.published.insert(key, verdict.clone());
        }
        changed
    }
}

/// How far a wallet's synced height may advance in one step and still count
/// as following the tip. Larger steps are catch-up after time offline: the
/// scan itself settles most roots, so a height-driven pass waits for it.
pub(crate) const CATCH_UP_STEP: u32 = 3;

/// The first advance after launch has no previous height to compare with, and
/// it is usually the stored tip from before the scan: treat it as catch-up
/// too. Probing starts from the next step.
pub(crate) fn is_catch_up_step(previous: Option<u32>, height: u32) -> bool {
    previous.is_none_or(|previous| height.saturating_sub(previous) > CATCH_UP_STEP)
}

/// One pass over one wallet at `height`: probe every due root and return what
/// to publish. `forced` roots are due regardless of their schedule (the SPV
/// broadcaster has just reported them `Uncertain`).
///
/// A root's verdict is published for the wallet's own sends: the root itself
/// when it is one, and — when the root is dead — every own send built on it,
/// since a transaction spending a dead one is dead too. A root found mined
/// counts as settled when picking roots, so its children are probed next.
pub(crate) async fn resolve_pass(
    state: &Mutex<ResolverState>,
    probe: &dyn AcceptanceProbe,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    forced: &HashSet<Txid>,
) -> Vec<ResolverEvent> {
    // What the wallet itself still holds unsettled — what state is kept for.
    let wallet_view = ChainGraph::new(views, &HashSet::new());
    let present: HashSet<Txid> = wallet_view.unsettled.iter().copied().collect();
    let own_present = wallet_view.own_unsettled();

    let mut events = Vec::new();
    let graph_mined: HashSet<Txid> = {
        let mut guard = state.lock().expect("resolver state poisoned");
        let ResolverState {
            schedules,
            finished,
            published,
        } = &mut *guard;
        let is_gone =
            |wallet: &WalletId, txid: &Txid| *wallet == wallet_id && !present.contains(txid);
        schedules.retain(|(wallet, txid), _| !is_gone(wallet, txid));
        finished.retain(|(wallet, txid), _| !is_gone(wallet, txid));
        let mut cleared: Vec<Txid> = published
            .keys()
            .filter(|(wallet, txid)| *wallet == wallet_id && !own_present.contains(txid))
            .map(|(_, txid)| *txid)
            .collect();
        cleared.sort();
        published.retain(|(wallet, txid), _| *wallet != wallet_id || own_present.contains(txid));
        events.extend(cleared.into_iter().map(ResolverEvent::Cleared));

        let mined: HashSet<Txid> = finished
            .iter()
            .filter(|((wallet, _), result)| *wallet == wallet_id && **result == Final::Mined)
            .map(|((_, txid), _)| *txid)
            .collect();
        mined
    };

    let graph = ChainGraph::new(views, &graph_mined);
    let due: Vec<(Txid, Transaction)> = {
        let mut guard = state.lock().expect("resolver state poisoned");
        let ResolverState {
            schedules,
            finished,
            ..
        } = &mut *guard;
        let mut due_now = Vec::new();
        for view in graph.roots() {
            let key = (wallet_id, view.txid);
            if finished.contains_key(&key) {
                continue;
            }
            let schedule = schedules
                .entry(key)
                .or_insert_with(|| ProbeSchedule::new(height));
            if forced.contains(&view.txid) || schedule.is_due(height) {
                schedule.probed_at(height);
                due_now.push((view.txid, view.transaction.clone()));
            }
        }
        due_now
    };

    for (root, transaction) in due {
        let verdict = probe.probe(&transaction).await;
        let key = (wallet_id, root);
        let mut guard = state.lock().expect("resolver state poisoned");
        // A pass that ran meanwhile may have found this root settled or gone
        // and dropped it; a verdict published now would resurrect it.
        if !guard.schedules.contains_key(&key) {
            continue;
        }
        match verdict {
            ProbeVerdict::Dead { .. } => {
                guard.finished.insert(key, Final::Dead);
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
                events.push(ResolverEvent::Verdict(send, verdict.clone()));
            }
        }
    }
    events
}

/// This wallet's unsettled transactions, one view per transaction, merged
/// across the accounts that record it (settled if any account holds it
/// settled; the wallet's own if any account records it spending the wallet's
/// coins). One walk over the records; only unsettled transactions are cloned —
/// on a healthy wallet there are none or a handful.
pub(crate) fn collect_views(info: &PlatformWalletInfo) -> Vec<OutgoingView> {
    let accounts = info.core_wallet.accounts.all_accounts();
    // Settled txids in a set; only unsettled records go into the map, so a
    // wallet with nothing unconfirmed costs one walk and no ordering work.
    let mut settled: HashSet<Txid> = HashSet::new();
    let mut unsettled: HashMap<Txid, (bool, &Transaction)> = HashMap::new();
    for account in accounts.iter() {
        for record in account.transactions().values() {
            if record.is_confirmed()
                || matches!(record.context, TransactionContext::InstantSend(_))
                || account.transaction_is_finalized(&record.txid)
            {
                settled.insert(record.txid);
                continue;
            }
            let own = !record.input_details.is_empty();
            unsettled
                .entry(record.txid)
                .and_modify(|(was_own, _)| *was_own |= own)
                .or_insert((own, &record.transaction));
        }
    }
    unsettled
        .into_iter()
        .filter(|(txid, _)| !settled.contains(txid))
        .map(|(txid, (own, transaction))| OutgoingView {
            txid,
            inputs: transaction
                .input
                .iter()
                .map(|input| input.previous_output)
                .collect(),
            settled: false,
            spends_own_coins: own,
            transaction: transaction.clone(),
        })
        .collect()
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

/// Which wallets have a pass running and what the next pass for each must do.
/// A pass for a wallet never runs twice at once; requests that arrive while
/// one runs are merged into the next.
#[derive(Default)]
struct PassQueue {
    running: HashSet<WalletId>,
    pending: HashMap<WalletId, PendingPass>,
}

#[derive(Default)]
struct PendingPass {
    height: u32,
    forced: HashSet<Txid>,
}

/// Runs probe passes off the wallet-event fan-out and publishes each verdict
/// change as [`PlatformEventHandler::on_outgoing_transaction_probed`], and each
/// root that stops being ambiguous as
/// [`PlatformEventHandler::on_outgoing_transaction_cleared`].
///
/// Off until the host turns it on with [`set_enabled`](Self::set_enabled): a
/// probe sends the signed transaction to evonodes over DAPI, which is a
/// product decision, not something a wallet should start doing on upgrade.
pub(crate) struct BroadcastResolver {
    enabled: Arc<AtomicBool>,
    probe: Arc<dyn AcceptanceProbe>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    state: Arc<Mutex<ResolverState>>,
    /// Last synced height per wallet: an SPV-level `Uncertain` carries no
    /// wallet id, so it is queued for every wallet at its current height.
    heights: Arc<Mutex<HashMap<WalletId, u32>>>,
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
            enabled: Arc::new(AtomicBool::new(false)),
            probe,
            wallet_manager,
            state: Arc::new(Mutex::new(ResolverState::default())),
            heights: Arc::new(Mutex::new(HashMap::new())),
            passes: Arc::new(Mutex::new(PassQueue::default())),
            events: Mutex::new(Weak::new()),
            tasks: ProbeTasks::new(),
        }
    }

    /// Where verdicts are published. Weak: the event manager holds this
    /// resolver as one of its handlers.
    pub(crate) fn set_event_manager(&self, events: Weak<PlatformEventManager>) {
        *self.events.lock().expect("events mutex poisoned") = events;
    }

    /// Turning probing off forgets every root and tells the host to drop every
    /// verdict it holds: with no passes running, nothing would ever clear them.
    pub(crate) fn set_enabled(&self, enabled: bool) {
        let was = self.enabled.swap(enabled, Ordering::SeqCst);
        if was && !enabled {
            let cleared = self
                .state
                .lock()
                .expect("resolver state poisoned")
                .forget_all();
            if let Some(events) = self.events.lock().expect("events mutex poisoned").upgrade() {
                for (wallet_id, txid) in &cleared {
                    events.on_outgoing_transaction_cleared(wallet_id, txid);
                }
            }
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Forget a removed wallet: its queued pass, its last height and every
    /// verdict, each of which the host is told is cleared.
    pub(crate) fn wallet_removed(&self, wallet_id: &WalletId) {
        self.passes
            .lock()
            .expect("pass queue poisoned")
            .pending
            .remove(wallet_id);
        self.heights
            .lock()
            .expect("heights mutex poisoned")
            .remove(wallet_id);
        let cleared = self
            .state
            .lock()
            .expect("resolver state poisoned")
            .forget_wallet(wallet_id);
        if let Some(events) = self.events.lock().expect("events mutex poisoned").upgrade() {
            for txid in &cleared {
                events.on_outgoing_transaction_cleared(wallet_id, txid);
            }
        }
    }

    /// Stop admitting passes and stop the running ones. Idempotent.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        self.tasks.quiesce_within(budget).await
    }

    /// Queue a pass for `wallet_id`; start a worker for it unless one runs.
    fn request_pass(&self, wallet_id: WalletId, height: u32, force: Option<Txid>) {
        {
            let mut passes = self.passes.lock().expect("pass queue poisoned");
            let pending = passes.pending.entry(wallet_id).or_default();
            pending.height = pending.height.max(height);
            pending.forced.extend(force);
            if !passes.running.insert(wallet_id) {
                return; // the running worker picks it up
            }
        }
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let probe = Arc::clone(&self.probe);
        let state = Arc::clone(&self.state);
        let heights = Arc::clone(&self.heights);
        let passes = Arc::clone(&self.passes);
        let enabled = Arc::clone(&self.enabled);
        let events = self.events.lock().expect("events mutex poisoned").clone();
        self.tasks.spawn(async move {
            let _running = Running {
                passes: Arc::clone(&passes),
                wallet_id,
            };
            loop {
                // Take the next request, or stop — under the same lock a
                // request_pass would take, so a request can never land between
                // "queue empty" and "no longer running" and be left unserved.
                // Probing turned off meanwhile: drop what is queued, do not
                // probe again (set_enabled already cleared the host's state).
                let next = {
                    let mut queue = passes.lock().expect("pass queue poisoned");
                    let next = if enabled.load(Ordering::SeqCst) {
                        queue.pending.remove(&wallet_id)
                    } else {
                        queue.pending.remove(&wallet_id);
                        None
                    };
                    if next.is_none() {
                        queue.running.remove(&wallet_id);
                    }
                    next
                };
                let Some(PendingPass { height, forced }) = next else {
                    break;
                };
                // Read under the lock, probe without it: a probe is network I/O.
                let views = {
                    let manager = wallet_manager.read().await;
                    manager.get_wallet_info(&wallet_id).map(collect_views)
                };
                let outcome = match views {
                    Some(views) => {
                        resolve_pass(&state, probe.as_ref(), wallet_id, height, &views, &forced)
                            .await
                    }
                    None => {
                        // The wallet was removed: forget it and clear whatever
                        // the host still shows for it.
                        heights
                            .lock()
                            .expect("heights mutex poisoned")
                            .remove(&wallet_id);
                        state
                            .lock()
                            .expect("resolver state poisoned")
                            .forget_wallet(&wallet_id)
                            .into_iter()
                            .map(ResolverEvent::Cleared)
                            .collect()
                    }
                };
                publish(&events, wallet_id, height, !forced.is_empty(), &outcome);
            }
        });
    }
}

/// Hand a pass's outcome to the host and the log.
fn publish(
    events: &Weak<PlatformEventManager>,
    wallet_id: WalletId,
    height: u32,
    forced: bool,
    outcome: &[ResolverEvent],
) {
    if outcome.is_empty() {
        return;
    }
    let events = events.upgrade();
    // What started this pass: dash-spv's Uncertain result, or an advance of
    // the wallet's synced height.
    let trigger = if forced { "uncertain" } else { "height" };
    for event in outcome {
        match event {
            ResolverEvent::Verdict(txid, verdict) => {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    txid = %txid,
                    height,
                    trigger,
                    ?verdict,
                    "broadcast probe: verdict for an unconfirmed send"
                );
                if let Some(events) = &events {
                    events.on_outgoing_transaction_probed(&wallet_id, txid, verdict);
                }
            }
            ResolverEvent::Cleared(txid) => {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    txid = %txid,
                    height,
                    "broadcast probe: send no longer unconfirmed, verdict cleared"
                );
                if let Some(events) = &events {
                    events.on_outgoing_transaction_cleared(&wallet_id, txid);
                }
            }
        }
    }
}

/// Marks a wallet's worker finished if it ends without reaching its own exit
/// (an abort at shutdown), so a later request is not refused forever.
struct Running {
    passes: Arc<Mutex<PassQueue>>,
    wallet_id: WalletId,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Ok(mut passes) = self.passes.lock() {
            passes.running.remove(&self.wallet_id);
        }
    }
}

impl EventHandler for BroadcastResolver {
    fn on_wallet_event(&self, event: &WalletEvent) {
        let WalletEvent::SyncHeightAdvanced { wallet_id, height } = event else {
            return;
        };
        let previous = self
            .heights
            .lock()
            .expect("heights mutex poisoned")
            .insert(*wallet_id, *height);
        if self.is_enabled() && !is_catch_up_step(previous, *height) {
            self.request_pass(*wallet_id, *height, None);
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
        let heights: Vec<(WalletId, u32)> = self
            .heights
            .lock()
            .expect("heights mutex poisoned")
            .iter()
            .map(|(wallet, height)| (*wallet, *height))
            .collect();
        for (wallet_id, height) in heights {
            self.request_pass(wallet_id, height, Some(*txid));
        }
    }
}

impl PlatformEventHandler for BroadcastResolver {}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use dashcore::hashes::Hash;

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

    fn tx(lock_time: u32) -> Transaction {
        Transaction {
            version: 2,
            lock_time,
            input: Vec::new(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn send(n: u8, inputs: &[OutPoint]) -> OutgoingView {
        OutgoingView {
            txid: txid(n),
            inputs: inputs.to_vec(),
            settled: false,
            spends_own_coins: true,
            transaction: tx(n as u32),
        }
    }

    fn roots(views: &[OutgoingView]) -> BTreeSet<Txid> {
        ambiguous_roots(views).into_iter().map(|v| v.txid).collect()
    }

    // ---- ambiguous_roots ----------------------------------------------------

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

    #[test]
    fn should_skip_settled_and_incoming_transactions() {
        let mut settled = send(1, &[outpoint(90, 0)]);
        settled.settled = true;
        let mut incoming = send(2, &[outpoint(91, 0)]);
        incoming.spends_own_coins = false;
        assert!(roots(&[settled, incoming]).is_empty());
    }

    /// Once the parent settles, the child has no unsettled parent and is a
    /// root in its own right.
    #[test]
    fn should_treat_the_child_of_a_settled_parent_as_a_root() {
        let mut parent = send(1, &[outpoint(90, 0)]);
        parent.settled = true;
        let child = send(2, &[outpoint(1, 1)]);
        assert_eq!(roots(&[parent, child]), BTreeSet::from([txid(2)]));
    }

    /// key-wallet records a transaction in every account it touches; one
    /// root, and a settled copy anywhere settles it.
    #[test]
    fn should_merge_a_transaction_recorded_in_two_accounts() {
        let a = send(1, &[outpoint(90, 0)]);
        let mut b = send(1, &[outpoint(90, 0)]);
        b.settled = true;
        assert!(roots(&[a.clone(), b]).is_empty());
        assert_eq!(roots(&[a.clone(), a]), BTreeSet::from([txid(1)]));
    }

    // ---- ProbeSchedule ------------------------------------------------------

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
            // Fixtures key transactions by lock_time == txid byte.
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

    #[tokio::test]
    async fn should_probe_only_roots_and_report_each_verdict() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(1), unresolved())]);
    }

    #[tokio::test]
    async fn should_stop_probing_a_root_proven_dead() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let again = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;

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

        let first = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let second = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;

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

        let first = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let same_block = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let next_block = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;

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

        resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        resolve_pass(
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

    /// A root that settled or left the wallet is forgotten, and the host is
    /// told to drop the verdict it was shown.
    #[tokio::test]
    async fn should_clear_a_published_root_that_is_no_longer_ambiguous() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let gone = resolve_pass(&state, &probe, wallet(), 101, &[], &HashSet::new()).await;

        assert_eq!(gone, vec![ResolverEvent::Cleared(txid(1))]);
        let guard = state.lock().expect("state");
        assert!(guard.schedules.is_empty());
        assert!(guard.finished.is_empty());
        assert!(guard.published.is_empty());
    }

    /// Nodes that have not seen an unconfirmed incoming payment refuse a send
    /// that spends it, so the send is not probed itself — the incoming parent
    /// is, since its fate decides the send's. Once the parent settles, the
    /// send is the root.
    #[test]
    fn should_probe_the_unconfirmed_incoming_parent_of_a_send_instead_of_the_send() {
        let mut incoming = send(5, &[outpoint(80, 0)]);
        incoming.spends_own_coins = false;
        let send_on_it = send(6, &[outpoint(5, 0)]);
        assert_eq!(
            roots(&[incoming.clone(), send_on_it.clone()]),
            BTreeSet::from([txid(5)])
        );

        incoming.settled = true;
        assert_eq!(roots(&[incoming, send_on_it]), BTreeSet::from([txid(6)]));
    }

    /// An unconfirmed incoming payment nobody built on is not this wallet's
    /// business to probe.
    #[test]
    fn should_not_probe_an_incoming_payment_the_wallet_did_not_spend() {
        let mut incoming = send(5, &[outpoint(80, 0)]);
        incoming.spends_own_coins = false;
        assert!(roots(&[incoming]).is_empty());
    }

    /// A transfer between the wallet's own accounts is recorded twice: as
    /// incoming in the receiving account (listed first) and as spending in the
    /// sending one. It is the wallet's own send either way.
    #[test]
    fn should_treat_a_transfer_between_own_accounts_as_the_wallets_send() {
        let mut as_received = send(1, &[outpoint(90, 0)]);
        as_received.spends_own_coins = false;
        let as_sent = send(1, &[outpoint(90, 0)]);
        assert_eq!(roots(&[as_received, as_sent]), BTreeSet::from([txid(1)]));
    }

    /// Two wallets' roots are tracked independently.
    #[tokio::test]
    async fn should_not_let_one_wallets_pass_forget_another_wallets_roots() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        resolve_pass(&state, &probe, [9u8; 32], 100, &[], &HashSet::new()).await;

        assert_eq!(state.lock().expect("state").schedules.len(), 1);
    }

    /// A mined transaction the wallet has not caught up with yet: nothing left
    /// to ask, so it is not probed again.
    #[tokio::test]
    async fn should_stop_probing_a_root_found_in_a_block() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Mined)]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let again = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;

        assert!(again.is_empty());
        assert_eq!(probe.probed(), vec![txid(1)]);
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

    #[tokio::test]
    async fn should_forget_one_wallet_and_return_its_published_roots() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved()), (txid(2), unresolved())]);
        resolve_pass(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &HashSet::new(),
        )
        .await;
        resolve_pass(
            &state,
            &probe,
            [9u8; 32],
            100,
            &[send(2, &[outpoint(91, 0)])],
            &HashSet::new(),
        )
        .await;

        let cleared = state.lock().expect("state").forget_wallet(&wallet());

        assert_eq!(cleared, vec![txid(1)]);
        let guard = state.lock().expect("state");
        assert_eq!(guard.published.len(), 1, "the other wallet is untouched");
        assert_eq!(guard.schedules.len(), 1);
    }

    #[tokio::test]
    async fn should_forget_everything_and_return_every_published_root() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved()), (txid(2), unresolved())]);
        resolve_pass(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &HashSet::new(),
        )
        .await;
        resolve_pass(
            &state,
            &probe,
            [9u8; 32],
            100,
            &[send(2, &[outpoint(91, 0)])],
            &HashSet::new(),
        )
        .await;

        let cleared = state.lock().expect("state").forget_all();

        assert_eq!(cleared.len(), 2);
        let guard = state.lock().expect("state");
        assert!(
            guard.published.is_empty() && guard.schedules.is_empty() && guard.finished.is_empty()
        );
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

        let events = resolve_pass(
            &state,
            &probe,
            wallet(),
            100,
            &[send(1, &[outpoint(90, 0)])],
            &HashSet::new(),
        )
        .await;

        assert!(events.is_empty());
        assert!(state.lock().expect("state").published.is_empty());
    }

    fn incoming(n: u8, inputs: &[OutPoint]) -> OutgoingView {
        OutgoingView {
            spends_own_coins: false,
            ..send(n, inputs)
        }
    }

    /// Spending a dead transaction is dead too: the whole chain of own sends
    /// built on a dead root is published dead, not just the root.
    #[tokio::test]
    async fn should_publish_a_dead_root_for_every_own_send_built_on_it() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;

        assert_eq!(
            events,
            vec![
                ResolverEvent::Verdict(txid(1), dead()),
                ResolverEvent::Verdict(txid(2), dead()),
            ]
        );
        let again = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;
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

        let events = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;

        assert_eq!(probe.probed(), vec![txid(5)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(6), dead())]);
    }

    #[tokio::test]
    async fn should_publish_nothing_while_an_incoming_parent_is_only_accepted() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(5), ProbeVerdict::Accepted)]);
        let views = [incoming(5, &[outpoint(80, 0)]), send(6, &[outpoint(5, 0)])];

        let events = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;

        assert!(events.is_empty());
    }

    /// A root found in a block that the wallet has not caught up with is
    /// settled for picking roots, so the send built on it is probed next.
    #[tokio::test]
    async fn should_probe_the_children_of_a_root_found_in_a_block() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Mined), (txid(2), dead())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let first = resolve_pass(&state, &probe, wallet(), 100, &views, &HashSet::new()).await;
        let second = resolve_pass(&state, &probe, wallet(), 101, &views, &HashSet::new()).await;

        assert_eq!(
            first,
            vec![ResolverEvent::Verdict(txid(1), ProbeVerdict::Mined)]
        );
        assert_eq!(second, vec![ResolverEvent::Verdict(txid(2), dead())]);
        assert_eq!(probe.probed(), vec![txid(1), txid(2)]);
    }

    /// One probe that met only transport errors says nothing new about a send
    /// already seen accepted; the UI must not flicker back to "unknown".
    #[test]
    fn should_not_let_unresolved_replace_an_accepted_verdict() {
        let mut state = ResolverState::default();
        let key = (wallet(), txid(1));

        assert!(state.publish(key, &ProbeVerdict::Accepted));
        assert!(!state.publish(key, &unresolved()));
        assert!(!state.publish(key, &ProbeVerdict::Accepted));
        assert!(state.publish(key, &dead()));
    }
}
