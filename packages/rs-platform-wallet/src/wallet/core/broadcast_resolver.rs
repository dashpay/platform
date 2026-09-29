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
//! When: once as soon as dash-spv reports an `Uncertain` broadcast, then once
//! per new block while unresolved; after [`EVERY_BLOCK_WINDOW`] blocks without
//! a verdict, every [`SLOW_INTERVAL`] blocks. A root stops being probed once a
//! probe says `Accepted` or `Dead`, or once it settles or leaves the wallet.
//! Probes run only while the SPV client delivers blocks, i.e. while the app is
//! in the foreground.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use dash_spv::EventHandler;
use dashcore::{OutPoint, Transaction, Txid};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::RwLock;

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

/// The roots of this wallet's unsettled chains, one entry per txid.
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
    let unsettled_own: BTreeSet<Txid> = by_txid
        .iter()
        .filter(|(_, (view, settled))| !*settled && view.spends_own_coins)
        .map(|(txid, _)| *txid)
        .collect();
    unsettled_own
        .iter()
        .map(|txid| by_txid[txid].0)
        .filter(|view| {
            !view
                .inputs
                .iter()
                .any(|input| unsettled_own.contains(&input.txid))
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
    resolved: HashSet<(WalletId, Txid)>,
}

/// One pass over one wallet at `height`: probe every due root and return the
/// verdicts to publish. `force` makes one root due regardless of its schedule
/// (the SPV broadcaster has just reported it `Uncertain`).
pub(crate) async fn resolve_pass(
    state: &Mutex<ResolverState>,
    probe: &dyn AcceptanceProbe,
    wallet_id: WalletId,
    height: u32,
    views: &[OutgoingView],
    force: Option<Txid>,
) -> Vec<(Txid, ProbeVerdict)> {
    let roots = ambiguous_roots(views);
    let root_ids: HashSet<Txid> = roots.iter().map(|view| view.txid).collect();

    // Decide what is due under the lock; probe without it.
    let due: Vec<(Txid, Transaction)> = {
        let mut guard = state.lock().expect("resolver state poisoned");
        let ResolverState {
            schedules,
            resolved,
        } = &mut *guard;
        // Forget this wallet's roots that settled or left the wallet.
        schedules.retain(|(wallet, txid), _| *wallet != wallet_id || root_ids.contains(txid));
        resolved.retain(|(wallet, txid)| *wallet != wallet_id || root_ids.contains(txid));

        let mut due = Vec::new();
        for view in &roots {
            let key = (wallet_id, view.txid);
            if resolved.contains(&key) {
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

    let mut verdicts = Vec::with_capacity(due.len());
    for (txid, transaction) in due {
        let verdict = probe.probe(&transaction).await;
        if !matches!(verdict, ProbeVerdict::Unresolved { .. }) {
            state
                .lock()
                .expect("resolver state poisoned")
                .resolved
                .insert((wallet_id, txid));
        }
        verdicts.push((txid, verdict));
    }
    verdicts
}

/// This wallet's unsettled transactions that spend its own coins, one view per
/// account record. Only those can be ambiguity roots, so settled and incoming
/// records are skipped here rather than cloned.
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
            if settled_anywhere.contains(&record.txid) || record.input_details.is_empty() {
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
                spends_own_coins: true,
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
    state: Mutex<(bool, Vec<tokio::task::JoinHandle<()>>)>,
}

impl ProbeTasks {
    fn new() -> Self {
        Self {
            state: Mutex::new((true, Vec::new())),
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
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
    async fn quiesce_within(&self, budget: std::time::Duration) -> bool {
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
    pub(crate) async fn quiesce_within(&self, budget: std::time::Duration) -> bool {
        self.tasks.quiesce_within(budget).await
    }

    fn spawn_pass(&self, wallet_id: WalletId, height: u32, force: Option<Txid>) {
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let probe = Arc::clone(&self.probe);
        let state = Arc::clone(&self.state);
        let events = self.events.lock().expect("events mutex poisoned").clone();
        self.tasks.spawn(async move {
            // Read under the lock, probe without it: a probe is network I/O.
            let views = {
                let manager = wallet_manager.read().await;
                match manager.get_wallet_info(&wallet_id) {
                    Some(info) => collect_views(info),
                    None => return,
                }
            };
            let verdicts =
                resolve_pass(&state, probe.as_ref(), wallet_id, height, &views, force).await;
            if verdicts.is_empty() {
                return;
            }
            let events = events.upgrade();
            for (txid, verdict) in &verdicts {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    txid = %txid,
                    ?verdict,
                    "broadcast probe: verdict for an unconfirmed send"
                );
                if let Some(events) = &events {
                    events.on_outgoing_transaction_probed(&wallet_id, txid, verdict);
                }
            }
        });
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

    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        let dash_spv::sync::SyncEvent::TransactionBroadcastResult {
            txid,
            result: dash_spv::BroadcastResult::Uncertain,
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

        let verdicts = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;

        assert_eq!(probe.probed(), vec![txid(1)]);
        assert_eq!(verdicts, vec![(txid(1), dead())]);
    }

    #[tokio::test]
    async fn should_stop_probing_a_root_once_it_is_resolved() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let again = resolve_pass(&state, &probe, wallet(), 101, &views, None).await;

        assert!(again.is_empty(), "an accepted root is not probed again");
        assert_eq!(probe.probed(), vec![txid(1)]);
    }

    #[tokio::test]
    async fn should_keep_probing_an_unresolved_root_on_each_new_block() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), unresolved())]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let same_block = resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        let next_block = resolve_pass(&state, &probe, wallet(), 101, &views, None).await;

        assert!(same_block.is_empty(), "one probe per block");
        assert_eq!(next_block, vec![(txid(1), unresolved())]);
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
        let forced = resolve_pass(&state, &probe, wallet(), 100, &views, Some(txid(1))).await;

        assert_eq!(forced.len(), 1);
    }

    /// A root that settled or left the wallet between passes is forgotten,
    /// so a later reappearance starts a fresh schedule.
    #[tokio::test]
    async fn should_forget_a_root_that_is_no_longer_ambiguous() {
        let state = Mutex::new(ResolverState::default());
        let probe = ScriptedProbe::new(&[(txid(1), ProbeVerdict::Accepted)]);
        let views = [send(1, &[outpoint(90, 0)])];

        resolve_pass(&state, &probe, wallet(), 100, &views, None).await;
        resolve_pass(&state, &probe, wallet(), 101, &[], None).await;

        let guard = state.lock().expect("state");
        assert!(guard.schedules.is_empty());
        assert!(guard.resolved.is_empty());
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
