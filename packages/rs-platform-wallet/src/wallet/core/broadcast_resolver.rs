//! Automatically asking the network about sends whose broadcast outcome is
//! unknown.
//!
//! A send the SPV broadcaster could not confirm ends as `MaybeSent`
//! (`BroadcastResult::Uncertain`): the wallet keeps its record as an
//! unconfirmed spend and nothing ever tells the user whether the payment
//! landed. This module re-asks the network through
//! [`AcceptanceProbe`](crate::broadcast_probe::AcceptanceProbe) and publishes
//! the verdict; it **changes nothing** in the wallet. Cancelling a dead send is
//! a separate, user-driven decision (see `abandon_plan`).
//!
//! What is probed: every *root* of an unsettled chain the wallet itself
//! signed — a transaction that spends this wallet's coins, is not in a block,
//! not InstantSend-locked, and does not spend the output of another such
//! transaction. Probing a child is pointless and misleading: a node that has
//! not seen the parent answers `missingorspent` for a perfectly valid child.
//! A healthy send is IS-locked within seconds, so in practice only sends that
//! went quiet are ever probed.
//!
//! A send that spends an unconfirmed coin of *someone else's* — an incoming
//! payment not yet in a block or IS-locked — is not a root either, for the same
//! reason: nodes that have not seen that parent refuse the send.
//!
//! When: once as soon as dash-spv reports an `Uncertain` broadcast, then on
//! every advance of the wallet's synced height (a filter batch: once per block
//! at the tip, larger steps while catching up) while the root stays
//! unsettled; after [`EVERY_BLOCK_WINDOW`] blocks, every [`SLOW_INTERVAL`]
//! blocks. At most one height-driven pass per wallet runs at a time. Only
//! `Dead` ends the probing — acceptance by one node's mempool is not
//! settlement, so an `Accepted` root keeps being asked until it settles. A
//! verdict is published only when it differs from the last one published for
//! that root, and a root that settles or leaves the wallet after a verdict was
//! published is published as cleared. Probes run only while the SPV client
//! delivers blocks, i.e. while the app is in the foreground.

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

/// The roots of this wallet's unsettled chains, one entry per txid: unsettled
/// transactions it signed that spend no output of any unsettled transaction —
/// its own or an incoming one.
pub(crate) fn ambiguous_roots(views: &[OutgoingView]) -> Vec<&OutgoingView> {
    // A transaction recorded in several accounts is one transaction; if any
    // account holds it as settled, it is.
    let mut by_txid: BTreeMap<Txid, (&OutgoingView, bool)> = BTreeMap::new();
    for view in views {
        by_txid
            .entry(view.txid)
            .and_modify(|(_, settled)| *settled |= view.settled)
            .or_insert((view, view.settled));
    }
    let unsettled: BTreeSet<Txid> = by_txid
        .iter()
        .filter(|(_, (_, settled))| !*settled)
        .map(|(txid, _)| *txid)
        .collect();
    unsettled
        .iter()
        .map(|txid| by_txid[txid].0)
        .filter(|view| view.spends_own_coins)
        .filter(|view| {
            !view
                .inputs
                .iter()
                .any(|input| unsettled.contains(&input.txid))
        })
        .collect()
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

/// Per-root bookkeeping shared by every pass.
#[derive(Debug, Default)]
pub(crate) struct ResolverState {
    schedules: HashMap<(WalletId, Txid), ProbeSchedule>,
    /// Roots proven dead: never probed again while they stay roots.
    dead: HashSet<(WalletId, Txid)>,
    /// The last verdict published per root, so only changes are published.
    published: HashMap<(WalletId, Txid), ProbeVerdict>,
}

/// What a pass has to tell the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolverEvent {
    /// A root's verdict changed.
    Verdict(Txid, ProbeVerdict),
    /// A root that had a published verdict settled or left the wallet.
    Cleared(Txid),
}

/// One pass over one wallet at `height`: probe every due root and return what
/// to publish. `force` makes one root due regardless of its schedule (the SPV
/// broadcaster has just reported it `Uncertain`).
pub(crate) async fn resolve_pass(
    state: &Mutex<ResolverState>,
    probe: &dyn AcceptanceProbe,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    force: Option<Txid>,
) -> Vec<ResolverEvent> {
    let roots = ambiguous_roots(views);
    let root_ids: HashSet<Txid> = roots.iter().map(|view| view.txid).collect();

    // Decide what is due under the lock; probe without it.
    let mut events = Vec::new();
    let due: Vec<(Txid, Transaction)> = {
        let mut guard = state.lock().expect("resolver state poisoned");
        let ResolverState {
            schedules,
            dead,
            published,
        } = &mut *guard;
        // Forget this wallet's roots that settled or left the wallet; tell the
        // host about the ones it had a verdict for.
        let is_gone =
            |wallet: &WalletId, txid: &Txid| *wallet == wallet_id && !root_ids.contains(txid);
        schedules.retain(|(wallet, txid), _| !is_gone(wallet, txid));
        dead.retain(|(wallet, txid)| !is_gone(wallet, txid));
        let mut cleared: Vec<Txid> = published
            .keys()
            .filter(|(wallet, txid)| is_gone(wallet, txid))
            .map(|(_, txid)| *txid)
            .collect();
        cleared.sort();
        published.retain(|(wallet, txid), _| !is_gone(wallet, txid));
        events.extend(cleared.into_iter().map(ResolverEvent::Cleared));

        let mut due = Vec::new();
        for view in &roots {
            let key = (wallet_id, view.txid);
            if dead.contains(&key) {
                continue;
            }
            let schedule = schedules
                .entry(key)
                .or_insert_with(|| ProbeSchedule::new(height));
            if force == Some(view.txid) || schedule.is_due(height) {
                schedule.probed_at(height);
                due.push((view.txid, view.transaction.clone()));
            }
        }
        due
    };

    for (txid, transaction) in due {
        let verdict = probe.probe(&transaction).await;
        let key = (wallet_id, txid);
        let mut guard = state.lock().expect("resolver state poisoned");
        if matches!(verdict, ProbeVerdict::Dead { .. }) {
            guard.dead.insert(key);
        }
        // Publish changes only. Unresolved reasons vary between probes
        // (different nodes, different errors), so all Unresolved count as one.
        let changed = match guard.published.get(&key) {
            None => true,
            Some(ProbeVerdict::Unresolved { .. }) => {
                !matches!(verdict, ProbeVerdict::Unresolved { .. })
            }
            Some(previous) => *previous != verdict,
        };
        if changed {
            guard.published.insert(key, verdict.clone());
            events.push(ResolverEvent::Verdict(txid, verdict));
        }
    }
    events
}

/// This wallet's unsettled transactions, one view per account record: the ones
/// it signed (the candidate roots) and the incoming ones (whose outputs a send
/// may spend, which disqualifies that send as a root). Settled records are
/// skipped rather than cloned.
pub(crate) fn collect_views(info: &PlatformWalletInfo) -> Vec<OutgoingView> {
    let accounts = info.core_wallet.accounts.all_accounts();
    // A transaction settled in any account is settled.
    let mut settled_anywhere: HashSet<Txid> = HashSet::new();
    for account in accounts.iter() {
        for record in account.transactions().values() {
            if record.is_confirmed()
                || matches!(record.context, TransactionContext::InstantSend(_))
                || account.transaction_is_finalized(&record.txid)
            {
                settled_anywhere.insert(record.txid);
            }
        }
    }
    let mut views = Vec::new();
    for account in accounts.iter() {
        for record in account.transactions().values() {
            if settled_anywhere.contains(&record.txid) {
                continue;
            }
            views.push(OutgoingView {
                txid: record.txid,
                inputs: record
                    .transaction
                    .input
                    .iter()
                    .map(|input| input.previous_output)
                    .collect(),
                settled: false,
                spends_own_coins: !record.input_details.is_empty(),
                transaction: record.transaction.clone(),
            });
        }
    }
    views
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

/// Runs probe passes off the wallet-event fan-out and publishes each verdict
/// as [`PlatformEventHandler::on_outgoing_transaction_probed`].
///
/// Off until the host turns it on with [`set_enabled`](Self::set_enabled): a
/// probe sends the signed transaction to evonodes over DAPI, which is a
/// product decision, not something a wallet should start doing on upgrade.
pub(crate) struct BroadcastResolver {
    enabled: AtomicBool,
    probe: Arc<dyn AcceptanceProbe>,
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    state: Arc<Mutex<ResolverState>>,
    /// Last synced height per wallet, so an SPV-level `Uncertain` (which
    /// carries no wallet id) can run a pass on every wallet at once.
    heights: Mutex<HashMap<WalletId, u32>>,
    /// Wallets with a height-driven pass running: a burst of height advances
    /// (catch-up after time offline) must not stack passes for one wallet.
    in_flight: Arc<Mutex<HashSet<WalletId>>>,
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
            state: Arc::new(Mutex::new(ResolverState::default())),
            heights: Mutex::new(HashMap::new()),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            events: Mutex::new(Weak::new()),
            tasks: ProbeTasks::new(),
        }
    }

    /// Where verdicts are published. Weak: the event manager holds this
    /// resolver as one of its handlers.
    pub(crate) fn set_event_manager(&self, events: Weak<PlatformEventManager>) {
        *self.events.lock().expect("events mutex poisoned") = events;
    }

    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Stop admitting passes and stop the running ones. Idempotent.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        self.tasks.quiesce_within(budget).await
    }

    fn spawn_pass(&self, wallet_id: WalletId, height: u32, force: Option<Txid>) {
        // A height-driven pass is skipped while one for this wallet runs; a
        // forced pass (dash-spv just reported `Uncertain`) always runs — the
        // schedule marks a root probed before probing, so the two cannot probe
        // the same root twice unless forced.
        let in_flight = if force.is_none() {
            if !self
                .in_flight
                .lock()
                .expect("in-flight mutex poisoned")
                .insert(wallet_id)
            {
                return;
            }
            Some(InFlight {
                set: Arc::clone(&self.in_flight),
                wallet_id,
            })
        } else {
            None
        };
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let probe = Arc::clone(&self.probe);
        let state = Arc::clone(&self.state);
        let events = self.events.lock().expect("events mutex poisoned").clone();
        self.tasks.spawn(async move {
            let _in_flight = in_flight;
            // Read under the lock, probe without it: a probe is network I/O.
            let views = {
                let manager = wallet_manager.read().await;
                match manager.get_wallet_info(&wallet_id) {
                    Some(info) => collect_views(info),
                    None => return,
                }
            };
            let outcome =
                resolve_pass(&state, probe.as_ref(), wallet_id, height, &views, force).await;
            if outcome.is_empty() {
                return;
            }
            let events = events.upgrade();
            // What started this pass: dash-spv's Uncertain result for one
            // transaction, or an advance of the wallet's synced height.
            let trigger = if force.is_some() {
                "uncertain"
            } else {
                "height"
            };
            for event in &outcome {
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
        });
    }
}

/// Removes a wallet from the in-flight set when its pass ends, however it ends.
struct InFlight {
    set: Arc<Mutex<HashSet<WalletId>>>,
    wallet_id: WalletId,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        if let Ok(mut set) = self.set.lock() {
            set.remove(&self.wallet_id);
        }
    }
}

impl EventHandler for BroadcastResolver {
    fn on_wallet_event(&self, event: &WalletEvent) {
        let WalletEvent::SyncHeightAdvanced { wallet_id, height } = event else {
            return;
        };
        self.heights
            .lock()
            .expect("heights mutex poisoned")
            .insert(*wallet_id, *height);
        if self.is_enabled() {
            self.spawn_pass(*wallet_id, *height, None);
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
            self.spawn_pass(wallet_id, height, Some(*txid));
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
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)]), send(2, &[outpoint(1, 1)])];

        let events = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(events, vec![ResolverEvent::Verdict(txid(1), dead())]);
    }

    #[tokio::test]
    async fn should_stop_probing_a_root_proven_dead() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), dead())]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let again = resolve_pass(&state, &probe, wallet(), 101, &views, None).await;

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

        let first = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let second = resolve_pass(&state, &probe, wallet(), 101, &views, None).await;

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

        let first = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let same_block = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let next_block = resolve_pass(&state, &probe, wallet(), 101, &views, None).await;

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

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        resolve_pass(&state, &probe, wallet(), 100, &views, Some(txid(1))).await;

        assert_eq!(probe.probed(), vec![txid(1), txid(1)]);
    }

    /// A root that settled or left the wallet is forgotten, and the host is
    /// told to drop the verdict it was shown.
    #[tokio::test]
    async fn should_clear_a_published_root_that_is_no_longer_ambiguous() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let gone = resolve_pass(&state, &probe, wallet(), 101, &[], None).await;

        assert_eq!(gone, vec![ResolverEvent::Cleared(txid(1))]);
        let guard = state.lock().expect("state");
        assert!(guard.schedules.is_empty());
        assert!(guard.dead.is_empty());
        assert!(guard.published.is_empty());
    }

    /// Nodes that have not seen an unconfirmed incoming payment refuse a send
    /// that spends it; such a send is not a root until its parent settles.
    #[test]
    fn should_not_pick_a_send_that_spends_an_unconfirmed_incoming_coin() {
        let mut incoming = send(5, &[outpoint(80, 0)]);
        incoming.spends_own_coins = false;
        let send_on_it = send(6, &[outpoint(5, 0)]);
        assert!(roots(&[incoming.clone(), send_on_it.clone()]).is_empty());

        incoming.settled = true;
        assert_eq!(roots(&[incoming, send_on_it]), BTreeSet::from([txid(6)]));
    }

    /// Two wallets' roots are tracked independently.
    #[tokio::test]
    async fn should_not_let_one_wallets_pass_forget_another_wallets_roots() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        resolve_pass(&state, &probe, [9u8; 32], 100, &[], None).await;

        assert_eq!(state.lock().expect("state").schedules.len(), 1);
    }
}
