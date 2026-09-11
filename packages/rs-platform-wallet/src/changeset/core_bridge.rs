//! Adapter that turns upstream `WalletEvent`s into `PlatformWalletChangeSet`s.
//!
//! Upstream `key_wallet_manager::WalletManager` exposes a dedicated,
//! **unbounded** `mpsc` persistence channel (drained via
//! `take_persistence_receiver()`) that carries every `WalletEvent` to this
//! single durable-persistence consumer losslessly and in order.
//! [`spawn_wallet_event_adapter`] is that consumer: a tokio task that pulls
//! events off the channel, projects each one into a
//! [`CoreChangeSet`](crate::changeset::CoreChangeSet), wraps it in a
//! [`PlatformWalletChangeSet`](crate::changeset::PlatformWalletChangeSet),
//! and forwards to the [`PlatformWalletPersistence`] sink.
//!
//! The manager keeps a separate, *bounded and lossy* `broadcast` bus for its
//! incidental subscribers (dash-spv's `EventHandler` fan-out, tests). This
//! consumer deliberately does NOT use that broadcast: under a heavy SPV
//! catch-up the broadcast ring overflows (`RecvError::Lagged`) and drops the
//! record/watermark events, which let the durable sync height outrun the
//! rows it implies and freeze forever. The unbounded
//! persistence channel can never `Lagged`, so that freeze cannot occur.
//!
//! # Why a single consumer, not per-wallet
//!
//! The persistence channel carries every event for every wallet. Each
//! event already carries a `wallet_id`, which the adapter forwards
//! verbatim to [`PlatformWalletPersistence::store`] — no need to fan
//! out a consumer per wallet.
//!
//! # Lifetime
//!
//! [`spawn_wallet_event_adapter`] returns a [`JoinHandle`]. The caller
//! (typically `PlatformWalletManager`) keeps the handle for the
//! manager's lifetime; on shutdown, fire the [`CancellationToken`] to
//! make the task exit cleanly.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, AddressState};
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::transaction_router::AccountTypeToCheck;
use key_wallet::transaction_checking::{DerivedAddressInfo, TransactionContext};
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{
    merge_payment_overlays, AssetLockChangeSet, CoreChangeSet, HighestUsedIndexes,
    IdentityChangeSet, IdentityEntry, PaymentOverlay, PlatformWalletChangeSet, SweepBatch,
    UtxoCreditVerdict,
};
use crate::changeset::merge::Merge;
use crate::changeset::persistence_capabilities::PersistenceCapabilities;
use crate::changeset::traits::PlatformWalletPersistence;
use crate::wallet::asset_lock::sync::reconstruction;
use crate::wallet::identity::network::sent_payment_status_for_record;
use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentStatus};
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Maximum number of `WalletEvent`s folded into a single
/// `persister.store(..)` round-trip by [`run_wallet_event_adapter`].
///
/// # Why batch at all
///
/// This adapter's per-event cost is overwhelmingly the persister call: on
/// Android that is a JNI hop into a Room transaction (milliseconds), while
/// projecting a `WalletEvent` into a [`CoreChangeSet`] is microseconds.
/// Storing one event per `store()` would therefore pin the drain rate at
/// roughly the *store* rate — low hundreds per second — which is far below
/// what a historical SPV catch-up emits. Draining the lossless persistence
/// channel one store at a time would let its backlog grow without bound
/// during a catch-up.
///
/// Folding every event *already buffered* in the channel into one changeset
/// per wallet collapses a burst of N events into a single store, so the
/// drain keeps pace with the producer at projection speed. This is
/// exactly the fold [`Merge`] was specified for — an ORDERED left fold in
/// channel-arrival order. `CoreChangeSet` merging is associative but NOT
/// commutative, so regrouping the fold is safe but reordering or
/// parallelizing it is not: sweep-aware merging deliberately depends on
/// operand order in two ways. A record arriving after a sweep of the same
/// txid retracts that sweep (reinstatement), while a sweep arriving after
/// the record survives the merge and deletes the row at apply time —
/// swapping the operands swaps which of those happens. And sweep batches
/// append in emission order because each release set is only true of the
/// wallet as that sweep saw it, so a later batch keeping a coin spent must
/// replay after the earlier batch that freed it. (The IS-lock map's
/// last-write-wins and the chain-lock equal-height tie-break also take the
/// later operand.) A reordered fold can therefore persist a different
/// spend decision, not just a differently-arranged changeset. The doc
/// comment on [`Merge`] states the same contract and already anticipates
/// this fold: "a flush can fold multiple events together
/// (TransactionDetected + BlockProcessed for the same wallet over a sync
/// round)".
///
/// The cap bounds the worst-case size of a single merged changeset (and
/// hence one Room transaction), and keeps a saturated producer from
/// starving the cancellation branch of the select below.
const ADAPTER_STORE_BATCH_LIMIT: usize = 512;

/// Session fault state for the durable-watermark guard.
///
/// Because the persistence channel is a lossless unbounded `mpsc`, two things
/// fault a wallet, and both name the wallets they hit, so a sibling whose rows
/// are still landing atomically keeps advancing — freezing it too would force a
/// redundant rescan of a wallet that never lost a row.
///
/// - A **`store()` rejection**, which carries a `wallet_id`: that wallet's rows
///   are known not to be on disk.
/// - A **panic in the blocking commit thread**, which faults every wallet with
///   something to persist that is absent from `settled` — the one that panicked
///   plus every wallet the loop never reached. Their outcome is unknown rather
///   than known-bad, and unknown must fail closed the same way.
///
/// The old global (`broadcast::Lagged`) latch is gone: the unbounded channel
/// can never `Lagged`, so there is no more "dropped events of unknown wallet"
/// signal to freeze everything for. This per-wallet freeze remains as a
/// fail-closed backstop — in a healthy run it never fires.
#[derive(Default)]
struct AdapterFaultState {
    /// Set by a `store()` rejection, or by the panic-recovery branch for a
    /// wallet whose commit outcome is unknown: freezes only the named wallets.
    per_wallet: HashMap<WalletId, bool>,
}

impl AdapterFaultState {
    /// Whether `wallet_id`'s durable watermark must be held frozen.
    fn is_faulted(&self, wallet_id: &WalletId) -> bool {
        self.per_wallet.get(wallet_id).copied().unwrap_or(false)
    }

    /// Fault a single wallet — after its `store()` was rejected, or after a
    /// commit panic left its outcome unknown — and raise the host-visible
    /// hard-fault signal.
    fn fault_wallet(&mut self, wallet_id: WalletId, hard_signal: &AtomicBool) {
        self.per_wallet.insert(wallet_id, true);
        hard_signal.store(true, Ordering::Relaxed);
    }
}

/// Per-drain accounting behind the one-line batch diagnostic.
///
/// This line is read off a tester's logcat to answer one question — *did the
/// durable sync watermark actually advance?* — so it must only ever report
/// what the persister truly accepted. A height can meet three different fates
/// in a single drain, and they are tracked separately because conflating them
/// sends a diagnosis down the wrong path:
///
/// * `persisted` — a changeset carrying this height was handed to
///   [`PlatformWalletPersistence::store`] and it returned `Ok`. **This is the
///   only field that means the durable watermark advanced.**
/// * `frozen` — the batch proposed this height, but the fail-closed guard
///   ([`freeze_synced_height_if_faulted`]) stripped it because that wallet had
///   already faulted this session, so it was never offered to the store.
///   Without this field a held-back watermark is indistinguishable from a
///   batch that simply carried no watermark at all.
/// * `rejected` — this height *was* offered to the store and the store
///   returned an error, so the rows and the watermark are not on disk.
///
/// Each is the monotonic max over the wallets in the drain, so a batch
/// spanning a healthy wallet and a faulted one reports both.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BatchDiagnostics {
    /// Events folded into this drain.
    folded: usize,
    /// Distinct wallets the drain produced a changeset for.
    wallets: usize,
    /// Highest watermark the store accepted and committed.
    persisted: Option<u32>,
    /// Highest watermark withheld by the fail-closed guard.
    frozen: Option<u32>,
    /// Highest watermark the store rejected.
    rejected: Option<u32>,
    /// Wallets in this drain that are faulted — whether they entered faulted
    /// or were faulted by it. Each wallet counts at most once per drain.
    faulted: usize,
}

impl BatchDiagnostics {
    fn new(folded: usize, wallets: usize) -> Self {
        Self {
            folded,
            wallets,
            ..Self::default()
        }
    }

    /// Raise `slot` to `height` if it is higher (or set it if unset).
    fn raise(slot: &mut Option<u32>, height: u32) {
        *slot = Some(slot.map_or(height, |cur| cur.max(height)));
    }

    /// The store returned `Ok` for a changeset carrying `height`.
    fn record_persisted(&mut self, height: u32) {
        Self::raise(&mut self.persisted, height);
    }

    /// The fail-closed guard stripped `height` before it reached the store.
    fn record_frozen(&mut self, height: u32) {
        Self::raise(&mut self.frozen, height);
    }

    /// The store returned an error for a changeset carrying `height`.
    fn record_rejected(&mut self, height: u32) {
        Self::raise(&mut self.rejected, height);
    }
}

impl std::fmt::Display for BatchDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "wallet-event batch: folded={} wallets={} synced_height_persisted={:?} \
             synced_height_frozen={:?} synced_height_rejected={:?} faulted={}",
            self.folded, self.wallets, self.persisted, self.frozen, self.rejected, self.faulted,
        )
    }
}

/// Spawn the wallet-event persistence task.
///
/// The `receiver` is the manager's lossless persistence receiver, taken once
/// via `take_persistence_receiver()` before the manager is published to
/// producers, and handed to this function. Exits when `cancel` fires or the
/// persistence channel's sender (the manager) is dropped, in both cases after
/// committing what the exiting drain had consumed — never mid-batch.
///
/// A drain commits the whole backlog only while the persister outlives it,
/// which is what [`PlatformWalletManager::shutdown`](crate::PlatformWalletManager::shutdown)
/// guarantees and a dirty drop does not: the task claims the persister when it
/// wakes, so a claim that finds it already released exits with the backlog
/// uncommitted (re-derived by the next SPV pass — the watermark rides the same
/// `store()` as the rows it implies).
///
/// `sync_fault` is the host-visible hard-fault latch: the task sets it
/// (and never clears it) the first time it freezes a durable watermark, so
/// an integrator can surface "verification failed / rescan pending" rather
/// than silently re-freezing on the next launch.
///
/// Generic over `P` so the spawned task gets static-dispatch on
/// every `persister.store(...)` call. Pass a `Weak` to the manager's own
/// `Arc<P>` (not to the `Arc<dyn PlatformWalletPersistence>` coercion) to
/// actually realize the static-dispatch win.
///
/// The reference is **weak**: the task holds nothing while parked for the next
/// event, so the persister is released when its owner drops rather than when
/// this task next polls. It upgrades once per drain — before consuming
/// anything — and keeps that claim until the drain's backlog is committed.
pub fn spawn_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Weak<P>,
    receiver: mpsc::UnboundedReceiver<WalletEvent>,
    sync_fault: Arc<AtomicBool>,
    cancel: CancellationToken,
) -> JoinHandle<()>
where
    P: PlatformWalletPersistence + 'static,
{
    tokio::spawn(run_wallet_event_adapter(
        wallet_manager,
        persister,
        receiver,
        sync_fault,
        cancel,
    ))
}

/// Drain `receiver`, project each [`WalletEvent`] into a
/// [`CoreChangeSet`], and forward to the persister. Split out of
/// [`spawn_wallet_event_adapter`] so the loop — not just the
/// [`freeze_synced_height_if_faulted`] helper — is directly testable
/// (drive a real `mpsc::UnboundedSender`, inject a probe persister).
///
/// # Lossless persistence channel
///
/// The upstream `WalletManager` publishes `WalletEvent`s to this consumer
/// over a dedicated, **unbounded** `mpsc` persistence channel (taken once
/// via `take_persistence_receiver()`). Because it is unbounded, a burst
/// larger than any ring cannot overflow it: the consumer never observes a
/// `Lagged`, and every `TransactionDetected` / `BlockProcessed` row event
/// reaches the persister before the `SyncHeightAdvanced` watermark that
/// implies it — in the same order the manager emitted them. There is also no
/// subscribe-before-publish race: an `mpsc::UnboundedReceiver` buffers events
/// sent before the task's first poll rather than dropping them.
///
/// The manager's *bounded* broadcast ring is the wrong transport here: during
/// a historical SPV catch-up the manager processes blocks far faster than
/// this single-threaded adapter can drain them through the (slow, JNI + Room)
/// persister, so the ring overflows and `recv()` returns `Lagged` — the
/// dropped events being exactly the record/UTXO/spent-marker events, while
/// the bare `SyncHeightAdvanced` watermark keeps flowing and advances the
/// persisted `syncedHeight` past blocks whose rows never reach disk. The
/// durable watermark then outruns its rows and the guard below latches it
/// frozen forever. With the lossless channel that path cannot occur.
///
/// # Durable-watermark guard (fail-closed backstop)
///
/// Two things fault a wallet: a rejected `store()` (the rows for that batch
/// are not on disk) and a panic in the blocking commit thread (the rows for
/// every persistable wallet the commit did not settle have an unknown fate,
/// which fails closed the same way). When a wallet faults either way, we never
/// advance ITS persisted sync watermark again this session — [`freeze_synced_height_if_faulted`]
/// strips `synced_height` from every subsequent changeset, holding the
/// durable watermark at the last height whose rows were fully committed.
/// Records/UTXO deltas in the same changeset still persist; only the height
/// advance is suppressed. On the next launch the SPV scan resumes from that
/// (lower) watermark and the persister's idempotent upserts re-apply the
/// missing rows. This is a fail-closed safety property — the durable
/// watermark never outruns the rows it implies — and in a healthy run it
/// never fires, since the channel is lossless, and both triggers mean a
/// genuine backend error rather than overload — a rejection is one the store
/// reported, a panic one it could not.
///
/// When a wallet faults, the task raises `sync_fault` (an `AtomicBool` the
/// host polls via `PlatformWalletManager::sync_fault_detected`) and logs a
/// one-shot `SYNC WATERMARK FROZEN` line via the `log` facade (which
/// android_logger forwards to logcat; `tracing` may not), so integrators can
/// show a hard "verification failed / rescan pending" state.
async fn run_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Weak<P>,
    mut receiver: mpsc::UnboundedReceiver<WalletEvent>,
    sync_fault: Arc<AtomicBool>,
    cancel: CancellationToken,
) where
    P: PlatformWalletPersistence + 'static,
{
    tracing::debug!("wallet-event adapter task started");
    // Both live behind handles rather than as locals because the commit runs
    // on a blocking thread (see the `spawn_blocking` below) and has to be able
    // to carry its state across drains. Moving them into the closure by value
    // would lose a wallet's frozen watermark if that thread ever panicked —
    // un-freezing a wallet that failed verification is the one outcome the
    // fail-closed guard exists to prevent.
    let fault = Arc::new(Mutex::new(AdapterFaultState::default()));
    // One-shot latch so the hard "watermark frozen" line hits logcat exactly
    // once per session rather than once per faulted batch.
    let freeze_logged = Arc::new(AtomicBool::new(false));
    // The claim carried between the chunks of one cancellation drain. Empty
    // while a commit is in flight — the claim rides into the blocking task and
    // back out — and released before the task parks for the next event, since
    // an idle adapter must hold nothing (issue #4133).
    let mut drain_persister: Option<Arc<P>> = None;

    loop {
        // Block for the first event of a batch. Everything already sitting in
        // the channel behind it is folded in below without another await, so a
        // burst costs one `store()` per wallet instead of one per event (see
        // [`ADAPTER_STORE_BATCH_LIMIT`]).
        let first = if cancel.is_cancelled() {
            // Shutting down: commit the backlog, never wait for more. The
            // `select!` below would race the fired token against `recv` and
            // drop it. The claim carries across these chunks, so a backlog
            // larger than one batch cannot lose its tail to a chunk boundary.
            match receiver.try_recv() {
                Ok(event) => Some(event),
                Err(_) => break,
            }
        } else {
            // About to park with nothing consumed: hold no strong reference,
            // or a dropped manager's store stays open until this task next
            // polls (issue #4133).
            drain_persister = None;
            tokio::select! {
                recv = receiver.recv() => recv,
                // Re-enter above to drain the backlog before exiting.
                _ = cancel.cancelled() => continue,
            }
        };

        // `recv()` on an mpsc returns `None` only when every sender (the
        // manager) has been dropped — the lossless channel has no `Lagged`.
        let Some(event) = first else {
            if !cancel.is_cancelled() {
                tracing::error!("WalletEvent persistence channel closed unexpectedly");
            }
            break;
        };

        // Claim the persister before folding anything else off the channel, so
        // everything this drain consumes is guaranteed a commit: an owner
        // releasing its `Arc` mid-drain can no longer strand events this task
        // has already taken. Claiming after the fold left a window as wide as
        // the fold itself in which a whole batch became uncommittable.
        //
        // The one event already in hand is the irreducible remainder: an
        // adapter that holds nothing while parked cannot claim before it wakes,
        // and by then the persister may be gone. Nothing durable breaks — the
        // watermark rides the same `store()` as the rows it implies, so the
        // next SPV pass re-derives both.
        //
        // Taken, never cloned: the claim MOVES into the commit below and comes
        // back out with the diagnostics, so a commit in flight is still the one
        // and only strong reference a dropped manager has to wait on (#4133).
        let persister_for_commit = match drain_persister.take() {
            Some(claimed) => claimed,
            None => match persister.upgrade() {
                Some(claimed) => claimed,
                None => {
                    tracing::warn!(
                        "persister already released when the wallet-event adapter woke; \
                         exiting with the backlog uncommitted — the next scan re-derives it"
                    );
                    break;
                }
            },
        };

        let mut batch: BTreeMap<WalletId, WalletBatch> = BTreeMap::new();
        let mut closed = false;
        {
            let wallet_id = event.wallet_id();
            // For events that need to consult per-wallet state (today only
            // `TransactionInstantLocked`, which checks finality before
            // recording the IS lock), `build_core_changeset` takes a brief
            // read lock on the manager.
            let core = build_core_changeset(&wallet_manager, &event).await;
            let asset_locks = reconstruct_asset_locks_for_event(&wallet_manager, &event).await;
            let payments = sent_payment_verdicts(&wallet_manager, &event).await;
            let entry = batch.entry(wallet_id).or_default();
            entry.core.merge(core);
            entry.asset_locks.merge(asset_locks);
            entry.payments.merge(payments);
        }

        // Fold in whatever else is already buffered. `try_recv` never waits,
        // so this drains the backlog at projection speed and stops as soon as
        // the channel is empty.
        let mut folded = 1usize;
        while folded < ADAPTER_STORE_BATCH_LIMIT {
            match receiver.try_recv() {
                Ok(event) => {
                    let wallet_id = event.wallet_id();
                    let core = build_core_changeset(&wallet_manager, &event).await;
                    let asset_locks =
                        reconstruct_asset_locks_for_event(&wallet_manager, &event).await;
                    let payments = sent_payment_verdicts(&wallet_manager, &event).await;
                    let entry = batch.entry(wallet_id).or_default();
                    entry.core.merge(core);
                    entry.asset_locks.merge(asset_locks);
                    // Last-write-wins per `(owner, txid)`: a transaction swept
                    // and then reinstated inside one drain reaches the store as
                    // the verdict the drain ended on, never as two rows.
                    entry.payments.merge(payments);
                    folded += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }

        // Commit the folded batch. The channel is lossless, so a watermark is
        // held back only by the fail-closed backstops around this call: a
        // rejected `store()` inside `commit_batch`, or a panic in the commit
        // thread, handled in the `Err` arm below.
        // Commit on a blocking thread, never on the async worker.
        //
        // `store()` is synchronous and, for the SQLite backend, commits a real
        // transaction per call — its own docs warn that a slow write blocks
        // every other wallet accessor for its duration. Called inline here it
        // blocked a tokio worker instead: a field restore showed one drain of
        // 512 folded events park the runtime long enough for the metrics tick
        // covering it to report a 1.4s mean poll, with the whole sync stalled
        // for minutes at a time and the durable watermark left hundreds of
        // thousands of blocks behind the chain tip.
        //
        // The handle is awaited rather than raced against `cancel`: a store
        // that has started must be allowed to finish, and dropping the handle
        // would not stop the thread anyway. Shutdown is observed at the next
        // `recv` instead.
        // Captured before the batch moves into the closure: if the commit
        // thread panics, these are the wallets whose rows have an unknown fate
        // and whose watermark must therefore be frozen.
        //
        // Only wallets `commit_batch` can actually call `store()` for. A wallet
        // contributes to the batch whenever it produced an event, including
        // events that project to nothing — a `TransactionInstantLocked` that is
        // ignored because the transaction is already chain-locked, a
        // `SyncHeightAdvanced` for an unknown wallet. Those hit the
        // `is_empty_no_records()` skip and never reach a store, so a sibling's
        // panic says nothing about them; freezing them would strip a healthy
        // wallet's watermark for the rest of the session over someone else's
        // bad batch.
        //
        // Deliberately conservative in one direction: `commit_batch` re-tests
        // emptiness AFTER `freeze_synced_height_if_faulted` has stripped a
        // faulted wallet's watermark, so a batch that looks non-empty here can
        // still be skipped there. That wallet is already faulted, so freezing
        // it again costs nothing — whereas the reverse error, omitting a wallet
        // whose store did run, would leave a watermark free to advance past
        // rows nobody can account for.
        let batch_wallet_ids: Vec<WalletId> = batch
            .iter()
            .filter(|(_, wallet_batch)| {
                !wallet_batch.core.is_empty_no_records()
                    || !Merge::is_empty(&wallet_batch.asset_locks)
                    || !wallet_batch.payments.is_empty()
            })
            .map(|(wallet_id, _)| *wallet_id)
            .collect();
        // Filled by `commit_batch` as each wallet's `store()` returns. Lives
        // out here so a panicking commit thread cannot take it down with it:
        // what it holds is the difference between "this wallet's rows are
        // accounted for" and "nobody knows".
        let settled: Arc<Mutex<Vec<WalletId>>> = Arc::new(Mutex::new(Vec::new()));
        let settled_for_commit = Arc::clone(&settled);
        let sync_fault_for_commit = Arc::clone(&sync_fault);
        let fault_for_commit = Arc::clone(&fault);
        let freeze_for_commit = Arc::clone(&freeze_logged);
        let committed = tokio::task::spawn_blocking(move || {
            // The lock is uncontended by construction — this task is the only
            // writer, and one drain commits at a time — so it never blocks;
            // it exists to carry the state, not to arbitrate.
            let mut fault = fault_for_commit
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut settled = settled_for_commit
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // The flag itself, not a copy: a local `bool` written back after
            // `commit_batch` returns is lost when a later store in the same
            // batch panics, and the panic branch would then emit the one-shot
            // marker a second time for a freeze already announced.
            let diag = commit_batch(
                &*persister_for_commit,
                batch,
                folded,
                &mut fault,
                &sync_fault_for_commit,
                &freeze_for_commit,
                &mut settled,
            );
            // Hand the claim back out: the next chunk of a cancellation drain
            // inherits it instead of racing a fresh upgrade against the owner's
            // release. A panicking `commit_batch` drops it instead, and the
            // next chunk re-claims.
            (persister_for_commit, diag)
        })
        .await;

        let diag = match committed {
            Ok((claimed, diag)) => {
                drain_persister = Some(claimed);
                diag
            }
            // The commit thread panicked, so `commit_batch` never reached the
            // `store()` rejection arm that would have frozen the affected
            // wallets. Freeze them here instead.
            //
            // Before this call moved off the runtime a panic unwound the whole
            // adapter task, which stopped every later watermark advance by
            // killing the writer. `spawn_blocking` turns that into a recoverable
            // `JoinError`, and simply continuing would let the NEXT batch
            // persist a higher `synced_height` for a wallet whose rows from this
            // batch may never have landed — the exact hole the fail-closed rule
            // exists to prevent. Faulting per wallet rather than stopping the
            // adapter keeps the existing design: a wallet whose commit is in
            // doubt freezes, its siblings keep syncing.
            Err(join_error) => {
                // `commit_batch` walks the batch serially, so a panic partitions
                // it: wallets whose `store()` already returned are settled — their
                // rows were accepted or rejected, and a rejection already faulted
                // them from inside. Freezing those too would strip a healthy
                // wallet's watermark for the rest of the session over a sibling's
                // bad batch.
                //
                // What is left — the wallet that panicked, plus every wallet the
                // loop never reached — has no known outcome, and its events are
                // gone from the lossless channel. Those must freeze, or a later
                // batch advances their watermark past rows that may never have
                // landed.
                let settled_ids = settled
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>();
                let unsettled: Vec<WalletId> = batch_wallet_ids
                    .iter()
                    .copied()
                    .filter(|id| !settled_ids.contains(id))
                    .collect();
                {
                    let mut fault = fault
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    for wallet_id in &unsettled {
                        fault.fault_wallet(*wallet_id, &sync_fault);
                    }
                }
                // Same one-shot marker the rejection arm emits, and via the same
                // `log` facade, because this freeze is indistinguishable from
                // that one as far as the host is concerned: the latch is up and
                // a rescan is pending. Leaving it to `tracing` alone would hide
                // a panic-induced freeze from logcat entirely, and — worse —
                // leave the flag clear, so a later rejected store would
                // emit the supposedly one-shot line as though it were the first
                // fault of the session.
                //
                // `swap` rather than load-then-store: a store panicking inside
                // the same blocking task can race the flag the task wrote back
                // on its way out, and emitting this line twice is a worse
                // outcome than the branch reading its own write.
                if !unsettled.is_empty() && !freeze_logged.swap(true, Ordering::Relaxed) {
                    log::error!(
                        "SYNC WATERMARK FROZEN: the wallet-event commit thread panicked ({}); \
                         {} wallet(s) whose rows have an unknown outcome are now held so the \
                         next scan re-persists them (dashpay/platform#4370). \
                         syncFaultDetected() is latched.",
                        join_error,
                        unsettled.len()
                    );
                }
                tracing::error!(
                    error = %join_error,
                    folded,
                    settled = settled_ids.len(),
                    frozen = unsettled.len(),
                    "wallet-event commit thread failed; freezing the wallets whose \
                     rows have an unknown outcome"
                );
                continue;
            }
        };

        // One structured line per drain via the `log` facade so a tester
        // logcat is unambiguous about whether the watermark is advancing.
        // Every field reports an observed outcome — see [`BatchDiagnostics`].
        log::info!("{}", diag);

        if closed {
            if !cancel.is_cancelled() {
                tracing::error!("WalletEvent persistence channel closed unexpectedly");
            }
            break;
        }
    }
    tracing::debug!("wallet-event adapter task exiting");
}

/// Commit one folded drain to the persister and report what actually happened.
///
/// Split out of [`run_wallet_event_adapter`] so the batch diagnostic — the line
/// we read off a tester's logcat to decide whether the durable watermark is
/// advancing — is directly unit-testable against a real `store()` rejection,
/// without the async channel plumbing. The returned [`BatchDiagnostics`] is
/// what the caller logs.
///
/// Ordering matters and is load-bearing:
///
/// 1. Apply [`freeze_synced_height_if_faulted`] *after* the fold, so a
///    `synced_height` that entered via `Merge` is stripped just like a
///    standalone one (otherwise folding would smuggle a watermark past the
///    guard and let the durable watermark outrun its rows).
/// 2. Record `frozen` from the height the batch *proposed*, captured before the
///    guard strips it.
/// 3. Record `persisted` only from the `Ok` arm of `store()`. A rejected store
///    means the rows never reached disk — the same condition that makes us
///    fault the wallet — so counting it as persisted would make the trace
///    contradict itself.
fn commit_batch<P>(
    persister: &P,
    batch: BTreeMap<WalletId, WalletBatch>,
    folded: usize,
    fault: &mut AdapterFaultState,
    sync_fault: &AtomicBool,
    freeze_logged: &AtomicBool,
    settled: &mut Vec<WalletId>,
) -> BatchDiagnostics
where
    P: PlatformWalletPersistence + ?Sized,
{
    let mut diag = BatchDiagnostics::new(folded, batch.len());
    for (wallet_id, wallet_batch) in batch {
        commit_wallet(
            persister,
            wallet_id,
            wallet_batch,
            &mut diag,
            fault,
            sync_fault,
            freeze_logged,
            settled,
        );
    }
    diag
}

/// Commit one wallet's folded changeset — the per-wallet unit of
/// [`commit_batch`].
///
/// Eight parameters, one over clippy's threshold: every one is a distinct
/// piece of the drain's state that this function must both read and write —
/// the fault map, the sync flag, the one-shot freeze log, the diagnostics
/// and the settled set that the panic arm in `run_wallet_event_adapter`
/// reads back. Bundling them into a struct would only rename the same
/// eight, and the borrow split is what keeps them separately mutable here.
#[allow(clippy::too_many_arguments)]
fn commit_wallet<P>(
    persister: &P,
    wallet_id: WalletId,
    wallet_batch: WalletBatch,
    diag: &mut BatchDiagnostics,
    fault: &mut AdapterFaultState,
    sync_fault: &AtomicBool,
    freeze_logged: &AtomicBool,
    settled: &mut Vec<WalletId>,
) where
    P: PlatformWalletPersistence + ?Sized,
{
    let WalletBatch {
        mut core,
        asset_locks,
        payments,
    } = wallet_batch;
    {
        // Sent-payment verdicts reach a host only through the payment-overlay
        // slot, which a persister advertises with `DASHPAY_PAYMENTS`. A host
        // without it would take the round, return `Ok`, and drop the verdict
        // on the floor — so withhold it and say so, once per round that had
        // one. Unlike a withheld sweep this does not freeze the watermark:
        // the verdict is derived state, so a host that later ships the slot
        // re-derives it from the records and the reconcile pass, whereas a
        // dropped removal has no such recovery.
        let (payments, verdict_identities) = if payments.is_empty() {
            (None, None)
        } else if persister
            .persistence_capabilities()
            .contains(PersistenceCapabilities::DASHPAY_PAYMENTS)
        {
            // Both carriers or neither. The identity snapshots are what make
            // the verdict survive a restart (`load()` rehydrates payments from
            // the identity blob, never from the overlay table); the overlay is
            // the bounded row a delta-style persister projects. Splitting them
            // across rounds is exactly the durability hole this pair closes,
            // and `DASHPAY_PAYMENTS` is the one bit that says a host stores
            // sent-payment state at all.
            (Some(payments.overlay), Some(payments.identities))
        } else {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                identities = payments.overlay.len(),
                rows = payments.overlay.values().map(BTreeMap::len).sum::<usize>(),
                "Persister does not advertise DASHPAY_PAYMENTS; withholding this round's \
                 sent-payment verdicts. Swept sent payments stay as stored until the host \
                 adopts the payment-overlay slot."
            );
            (None, None)
        };
        // Hold this wallet's durable watermark at the last fully persisted
        // height once it has faulted. Records/UTXOs still persist — only the
        // height advance is suppressed.
        let is_faulted = fault.is_faulted(&wallet_id);
        if is_faulted {
            diag.faulted += 1;
        }
        // Capture what this batch PROPOSED before the guard can strip it, so a
        // withheld watermark is reported as frozen instead of silently reading
        // as "this batch carried no watermark".
        let proposed_height = core.synced_height;
        freeze_synced_height_if_faulted(&mut core, is_faulted);
        if is_faulted {
            if let Some(h) = proposed_height {
                diag.record_frozen(h);
            }
        }
        if core.is_empty_no_records()
            && Merge::is_empty(&asset_locks)
            && payments.is_none()
            && verdict_identities.is_none()
        {
            // SyncHeightAdvanced for an unknown wallet, empty BlockProcessed, a
            // watermark-only batch stripped by the fault guard above, a verdict
            // withheld from a payments-blind persister, etc. — nothing to
            // persist. Skip the round-trip.
            return;
        }
        // The height this changeset OFFERS to the store. It is counted as
        // persisted only in the `Ok` arm below.
        let offered_height = core.synced_height;

        // Sweeps reach an FFI host only through the persistence extension's
        // size-negotiated sweep callback, and Rust never calls a slot the
        // host's declared `struct_size` did not prove — so a persister
        // predating that slot (an old C host, or a Kotlin subclass that
        // never overrode `onWalletChangesetTransactionsSwept`) processes the
        // rest of the round normally and returns success without ever
        // seeing `core.sweeps` at all. `store()` coming back `Ok` in that
        // case proves nothing about whether the removal actually happened,
        // so it is checked separately from the result below rather than
        // folded into it.
        let sweep_removal_unsupported = !core.sweeps.is_empty()
            && !persister
                .persistence_capabilities()
                .contains(PersistenceCapabilities::CORE_SWEEP_REMOVAL);
        if sweep_removal_unsupported {
            // Strip the watermark from THIS round, not just later ones. The
            // adapter folds whatever is buffered, so a `TransactionsSwept`
            // and a following `SyncHeightAdvanced` land in one changeset —
            // and `synced_height` lives in the unchanged prefix such a
            // persister does read. Letting it through would commit a height
            // that claims blocks are scanned while the removal those blocks
            // implied never landed, and the fault below cannot retract a
            // watermark the backend has already made durable. `offered_height`
            // keeps the original so the rejection is still diagnosed as a
            // withheld advance rather than as a round that carried none.
            //
            // `last_processed_height` is deliberately NOT stripped, matching
            // the existing fault guard (`freeze_synced_height_if_faulted`,
            // dashpay/platform#4069). The two watermarks answer different
            // questions: `synced_height` is the durable claim "everything up
            // to here is scanned AND persisted", which is what must not
            // outrun an unapplied removal, while `last_processed_height` is
            // the adapter's own progress marker and holding it back would
            // re-drive work without making anything safer.
            core.synced_height = None;
        }

        let cs = PlatformWalletChangeSet {
            core: Some(core),
            // Tracked-asset-lock rows reconstructed from this drain's
            // records (see `reconstruct_asset_locks_for_event`) ride the
            // same store round-trip so the row and the record that
            // implies it land atomically.
            asset_locks: (!Merge::is_empty(&asset_locks)).then_some(asset_locks),
            // The authoritative half of the same verdicts: a post-flip
            // `IdentityEntry` per identity whose payments moved. `load()`
            // rebuilds `dashpay_payments` from this snapshot and never reads
            // the overlay table back, so without it the round's verdict is
            // undone by the next restart.
            identities: verdict_identities,
            // The sent-payment verdicts this drain resolved, on the same round
            // as the sweep removal or confirming record that justifies them.
            dashpay_payments_overlay: payments,
            ..PlatformWalletChangeSet::default()
        };
        let store_result = persister.store(wallet_id, cs);
        // Recorded only once the store has RETURNED, and whether it accepted
        // or rejected — both are answers. A wallet whose store panicked never
        // reaches this line, and one the loop never got to is never pushed at
        // all, so what is missing from `settled` is exactly the set whose
        // outcome the caller cannot reason about. See the `JoinError` arm in
        // `run_wallet_event_adapter`.
        settled.push(wallet_id);
        match store_result {
            Ok(()) if sweep_removal_unsupported => {
                // The write nominally succeeded, but a backend that never
                // attested `CORE_SWEEP_REMOVAL` is not known to have applied
                // the one subtractive part of this round — reporting it
                // durable would let the swept loser return at the next
                // `load()`. Fault exactly like a rejection: the watermark is
                // held, so the next scan re-emits the sweep and the
                // idempotent removal is retried.
                //
                // Recovery is not in-session: the persister does not change
                // under a running adapter, so a host that lacks the slot
                // stays frozen until it ships one and relaunches. Freezing
                // is the point — it is what keeps a height that outran an
                // unapplied removal from becoming durable.
                // The guard above stripped the height before the store saw
                // it, so it is reported FROZEN — a `rejected` here would send
                // the operator to a persister that returned `Ok`.
                if fault_and_freeze(
                    diag,
                    WithheldHeight::Frozen(offered_height),
                    fault,
                    sync_fault,
                    wallet_id,
                    is_faulted,
                    freeze_logged,
                ) {
                    log::error!(
                        "SYNC WATERMARK FROZEN: persister for wallet {} does not advertise \
                         CORE_SWEEP_REMOVAL but this round swept one or more transactions; a \
                         removal must never be reported durable to a backend that cannot apply \
                         it, so the sync watermark is held back (dashpay/platform#4406).",
                        hex::encode(wallet_id)
                    );
                }
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    "Persister lacks CORE_SWEEP_REMOVAL for a changeset carrying sweeps; \
                     freezing this wallet's sync watermark rather than trusting an unversioned \
                     store() success"
                );
            }
            Ok(()) => {
                if let Some(h) = offered_height {
                    diag.record_persisted(h);
                }
            }
            Err(e) => {
                // A rejected changeset means these rows are not on disk. Fault
                // THIS wallet's watermark so it can't outrun them; the next
                // scan re-emits and the idempotent upserts recover the state.
                // Rejected — unless the sweep guard had already stripped the
                // height, in which case the store never saw it and it stays
                // a frozen one whatever the store then said.
                let withheld = if sweep_removal_unsupported {
                    WithheldHeight::Frozen(offered_height)
                } else {
                    WithheldHeight::Rejected(offered_height)
                };
                if fault_and_freeze(
                    diag,
                    withheld,
                    fault,
                    sync_fault,
                    wallet_id,
                    is_faulted,
                    freeze_logged,
                ) {
                    log::error!(
                        "SYNC WATERMARK FROZEN: persister rejected a changeset for wallet {} ({}); \
                         its durable sync height is now held so the next scan re-persists the \
                         missing rows (dashpay/platform#4069). syncFaultDetected() is latched.",
                        hex::encode(wallet_id),
                        e
                    );
                }
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "Persister rejected core changeset; freezing this wallet's sync watermark so the next scan re-persists the missing rows (dashpay/platform#4069)"
                );
            }
        }
    }
}

/// How a round's proposed `synced_height` was withheld, for
/// [`BatchDiagnostics`]. The two are different answers to "where is the
/// watermark?": a REJECTED height was offered to the store and the store
/// said no, so the operator looks at the persister; a FROZEN height was
/// stripped by the adapter before the store ever saw it — the sweep guard
/// does this when the backend never attested `CORE_SWEEP_REMOVAL` — so the
/// operator looks at the host's missing capability, not at a store that
/// in fact returned `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WithheldHeight {
    /// Stripped by the adapter; never offered to the store.
    Frozen(Option<u32>),
    /// Offered to the store, which returned an error.
    Rejected(Option<u32>),
}

/// The bookkeeping shared by the two ways a round fails to be durably
/// applied — a rejected `store()`, and a nominal success from a backend
/// that cannot have applied the round's sweeps. Records the withheld
/// advance under the field that says which of the two it was, faults the wallet (counting it once per drain: a wallet that
/// entered already faulted was counted at the top of the loop, and a
/// repeat failure must not count it again), and returns whether this is
/// the drain's first freeze — the caller owns the one-shot `log`-facade
/// line, whose wording differs per cause (android_logger forwards `log`
/// to logcat; `tracing` may not).
fn fault_and_freeze(
    diag: &mut BatchDiagnostics,
    withheld: WithheldHeight,
    fault: &mut AdapterFaultState,
    sync_fault: &AtomicBool,
    wallet_id: WalletId,
    entered_faulted: bool,
    freeze_logged: &AtomicBool,
) -> bool {
    match withheld {
        WithheldHeight::Frozen(Some(h)) => diag.record_frozen(h),
        WithheldHeight::Rejected(Some(h)) => diag.record_rejected(h),
        WithheldHeight::Frozen(None) | WithheldHeight::Rejected(None) => {}
    }
    fault.fault_wallet(wallet_id, sync_fault);
    if !entered_faulted {
        diag.faulted += 1;
    }
    // One-shot: only the first freeze of the session logs.
    !freeze_logged.swap(true, Ordering::Relaxed)
}

/// Durable-watermark guard.
///
/// When a wallet has faulted this session — its `store()` was rejected, or a
/// commit panic left the batch's outcome unknown — its persisted
/// `synced_height` watermark must not advance past the last height whose rows
/// were fully committed; otherwise the wallet believes it is scanned and never
/// re-matches the blocks whose rows were lost. This
/// strips ONLY `synced_height`; every other field (records, UTXO
/// deltas, `last_processed_height`, chain-lock) is left intact so
/// in-flight rows still persist. Factored out as a pure function so the
/// invariant is unit-testable without the async channel plumbing.
fn freeze_synced_height_if_faulted(core: &mut CoreChangeSet, persistence_faulted: bool) {
    if persistence_faulted {
        core.synced_height = None;
    }
}

/// Per-wallet fold of one drain: the projected core rows plus any
/// tracked-asset-lock entries reconstructed from the same events. Both
/// sub-changesets are committed in a single `store()` per wallet.
#[derive(Default)]
struct WalletBatch {
    core: CoreChangeSet,
    asset_locks: AssetLockChangeSet,
    /// Sent-payment verdicts this drain resolved (see
    /// [`sent_payment_verdicts`]), in both carriers: the bounded overlay rows
    /// and the authoritative post-flip identity snapshots. Rides the same
    /// `store()` as the rows that justify it — a sweep's removal, or the
    /// record that confirmed it — because neither event re-emits once its
    /// round is durable.
    payments: SentPaymentVerdicts,
}

/// Rebuild missing tracked asset locks from the records an event
/// carries (see [`crate::wallet::asset_lock::sync::reconstruction`]).
///
/// This is what repopulates the tracked-asset-lock set — and through it
/// the host's persisted mirror (e.g. the swift-sdk `PersistentAssetLock`
/// store) — after a wallet restore: the restore scan re-emits every
/// historical asset-lock funding tx as a `BlockProcessed` record filed
/// under the funding account whose pool its credit output pays.
/// `TransactionDetected` is included for live off-chain detections (a
/// same-seed wallet on another device broadcasting an asset lock).
///
/// The lock-free `is_reconstruction_candidate` pre-filter keeps the
/// wallet-manager write lock off the hot path: plain payment records
/// (the overwhelming majority of scan traffic) never qualify. Locks
/// tracked live by the build pipeline are never overwritten
/// (insert-if-absent inside `reconstruct_tracked_asset_locks`).
async fn reconstruct_asset_locks_for_event(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    event: &WalletEvent,
) -> AssetLockChangeSet {
    let (wallet_id, candidates): (WalletId, Vec<&TransactionRecord>) = match event {
        WalletEvent::TransactionDetected {
            wallet_id, record, ..
        } => (
            *wallet_id,
            std::iter::once(&**record)
                .filter(|r| reconstruction::is_reconstruction_candidate(r))
                .collect(),
        ),
        WalletEvent::BlockProcessed {
            wallet_id,
            inserted,
            updated,
            ..
        } => (
            *wallet_id,
            inserted
                .iter()
                .chain(updated.iter())
                .filter(|r| reconstruction::is_reconstruction_candidate(r))
                .collect(),
        ),
        // A chainlock's bulk promotion (`InBlock` →
        // `InChainLockedBlock`) surfaces ONLY here — the promoted
        // records never re-flow as `TransactionDetected` /
        // `BlockProcessed`. Without this arm, entries a restore scan
        // inserted at pre-finality statuses stayed there for the whole
        // session (the scan detects historical funding txs before any
        // chainlock is applied); see
        // `enrich_tracked_asset_locks_from_chain_lock`.
        WalletEvent::ChainLockProcessed {
            wallet_id,
            chain_lock,
            locked_transactions,
        } => {
            return reconstruction::enrich_tracked_asset_locks_from_chain_lock(
                wallet_manager,
                wallet_id,
                chain_lock.block_height,
                locked_transactions,
            )
            .await;
        }
        // The subtractive arm: a swept funding tx can never confirm, so
        // every tracked lock it funds is dead. Nothing else cascades the
        // sweep into this table — without this arm the entry is a zombie
        // `resume_asset_lock` re-broadcasts and waits on without bound,
        // mirrored forever by every store. A chainlocked return re-emits
        // the funding record through the arms above, which re-insert the
        // entry, so removal here is not a one-way door.
        WalletEvent::TransactionsSwept {
            wallet_id, txids, ..
        } => {
            return reconstruction::remove_tracked_asset_locks_for_swept(
                wallet_manager,
                wallet_id,
                txids,
            )
            .await;
        }
        _ => return AssetLockChangeSet::default(),
    };
    if candidates.is_empty() {
        return AssetLockChangeSet::default();
    }
    reconstruction::reconstruct_tracked_asset_locks(wallet_manager, &wallet_id, &candidates).await
}

/// Project an upstream [`WalletEvent`] into a [`CoreChangeSet`] suitable
/// for atomic persistence.
async fn build_core_changeset(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    event: &WalletEvent,
) -> CoreChangeSet {
    match event {
        WalletEvent::TransactionDetected {
            wallet_id,
            record,
            addresses_derived,
            ..
        } => {
            // Live mempool matching emits ONE event per matched
            // account, each carrying only that account's slice — and
            // nothing marks a transaction's last slice. Folding
            // whatever slices happen to share an adapter drain would
            // make the persisted row depend on scheduling: a drain
            // that catches one slice would store that slice as the
            // wallet's row, nondeterministically. The MANAGER, not
            // the drain, is
            // the boundary where a transaction's slices are complete:
            // by the time this event is projected the manager already
            // holds every so-far-matched account's record for the
            // txid (mempool records are never pruned — only
            // chain-locked ones are). Rebuild the wallet-level row
            // from that snapshot; a sibling account matching later
            // re-runs this rebuild through its own event, so the
            // persisted row converges on the full fold no matter how
            // events land in drains.
            //
            // `None` = the manager doesn't know the wallet (removed
            // mid-flight, or a bare test manager): fall back to the
            // event's own record. `Some([])` = the wallet exists but
            // the record is gone (chain-locked and pruned between
            // emit and drain): emit NO row rather than let a lone
            // stale slice supersede a complete fold earlier in this
            // drain's batch — the chainlock's own events carry the
            // row's finality forward.
            //
            // The credit verdicts are read under the SAME guard as the
            // slices: a verdict is the engine's opinion on a record's
            // outputs, and the record's own context is one of its inputs
            // (an unconfirmed record whose input a block spent reads
            // `Doomed`). Read from two snapshots, a funding that confirmed
            // and lost its coin to a mempool child between them would be
            // judged on a stale mempool context and marked spent for good.
            let (slices, utxo_credit_verdicts): (
                Vec<TransactionRecord>,
                BTreeMap<OutPoint, UtxoCreditVerdict>,
            ) = match wallet_slices_and_verdicts_for_txid(wallet_manager, wallet_id, &record.txid)
                .await
            {
                Some(read) => read,
                None => (vec![(**record).clone()], BTreeMap::new()),
            };
            // A contact's watch-only chain never defines the wallet's
            // transaction row or its TXOs (see `is_contact_watch_only`);
            // the usage deltas below are still emitted, so the event
            // is not dropped and the contact's address pool still
            // advances.
            let owned: Vec<TransactionRecord> = slices
                .iter()
                .filter(|r| !is_contact_watch_only(r))
                .cloned()
                .collect();
            let (addresses_marked_used, account_highest_used) =
                collect_usage_deltas(wallet_manager, wallet_id, vec![&**record]).await;
            let mut folded = owned.clone();
            crate::changeset::changeset::fold_same_txid_records(&mut folded);
            CoreChangeSet {
                // New UTXOs from the owned slices only (a watch-only
                // chain's outputs are the contact's coins); spends
                // from ALL slices, so a contact spending an output a
                // pre-fix build persisted still clears the stale row.
                new_utxos: owned.iter().flat_map(derive_new_utxos).collect(),
                spent_utxos: slices.iter().flat_map(derive_spent_utxos).collect(),
                records: folded,
                account_records: owned,
                // Mirror the upstream-emitted derived addresses
                // through to the persister so newly-extended pool
                // rows are written transactionally with the tx that
                // triggered the extension. See
                // `CoreChangeSet.addresses_derived` for the cascade-
                // link rationale.
                addresses_derived: addresses_derived.clone(),
                addresses_marked_used,
                account_highest_used,
                utxo_credit_verdicts,
                ..CoreChangeSet::default()
            }
        }
        WalletEvent::TransactionInstantLocked {
            wallet_id,
            txid,
            instant_lock,
            ..
        } => {
            // IS-lock is informative only for non-final records. If the
            // wallet has already chain-locked this txid, drop the lock —
            // chain-lock supersedes IS finality.
            if is_chain_locked(wallet_manager, wallet_id, txid).await {
                return CoreChangeSet::default();
            }
            let mut cs = CoreChangeSet::default();
            cs.instant_locks_for_non_final_records
                .insert(*txid, instant_lock.clone());
            cs
        }
        WalletEvent::BlockProcessed {
            wallet_id,
            height,
            inserted,
            updated,
            matured,
            addresses_derived,
            ..
        } => {
            let mut cs = CoreChangeSet::default();
            // Inserted records bring fresh UTXOs and may consume previous ones.
            for r in inserted {
                cs.new_utxos.extend(derive_new_utxos(r));
                cs.spent_utxos.extend(derive_spent_utxos(r));
            }
            // Updated records (re-confirmation, IS-lock applied to a known
            // mempool tx, etc.) don't usually change UTXO topology — the
            // record's content does change though, so re-emit it.
            // Matured coinbase records likewise: no UTXO topology change,
            // just a status update for the persister.
            //
            // Contact watch-only records are filtered out of all three
            // lists: re-emitting one on confirmation would re-clobber the
            // funding account's row with an incoming/positive
            // classification just as the first sighting did (see
            // `is_contact_watch_only`).
            // Keep the raw per-account slices for persisters that
            // route per-account state (the FFI projection buckets
            // TXOs by owning account from these), then fold the
            // wallet-level `records` copy: one block can insert
            // SEVERAL per-account records for one transaction (a
            // multi-account spend), and the txid-keyed row needs the
            // one wallet-level record (see fold_same_txid_records).
            cs.account_records.extend(
                inserted
                    .iter()
                    .chain(updated.iter())
                    .chain(matured.iter())
                    .filter(|r| !is_contact_watch_only(r))
                    .cloned(),
            );
            cs.records = cs.account_records.clone();
            crate::changeset::changeset::fold_same_txid_records(&mut cs.records);
            cs.last_processed_height = Some(*height);
            // Pool extensions triggered by any record in this block.
            // Already deduped upstream by `project_derived_addresses`;
            // `Merge` re-dedupes if multiple events fold together.
            cs.addresses_derived = addresses_derived.clone();
            // Used-flag flips + highest-used watermarks for every
            // record in the block. `updated` / `matured` records
            // re-emit their involved addresses — idempotent at the
            // persister, and it lets a rescan converge stores that
            // missed the original insert-time flip.
            let records: Vec<&TransactionRecord> = inserted
                .iter()
                .chain(updated.iter())
                .chain(matured.iter())
                .collect();
            let (addresses_marked_used, account_highest_used) =
                collect_usage_deltas(wallet_manager, wallet_id, records).await;
            cs.addresses_marked_used = addresses_marked_used;
            cs.account_highest_used = account_highest_used;
            // The engine's verdict on the outputs the persister is about to
            // materialise from these records — see
            // `CoreChangeSet::utxo_credit_verdicts`. Over the owned slices
            // only (`account_records` is already filtered): a contact's
            // watch-only chain never defines the wallet's TXOs.
            cs.utxo_credit_verdicts = utxo_credit_verdicts(
                wallet_manager,
                wallet_id,
                &cs.account_records.iter().collect::<Vec<_>>(),
            )
            .await;
            cs
        }
        WalletEvent::TransactionsSwept {
            txids,
            superseded_by,
            winner_mined_height,
            released_outpoints,
            ..
        } => {
            // The only subtractive event upstream emits. Each txid was a
            // recorded spend that `superseded_by` beat to an input, so it can
            // never confirm and the wallet has already dropped it. Mirroring
            // the removal is not optional: every other arm here appends, so a
            // persister that skipped this would keep the dead rows, hand them
            // back on the next load, and re-create the balance the wallet
            // just corrected — the exact bug the upstream sweep fixes.
            //
            // No `spent_utxos` entry for the inputs: a wallet-relevant winner
            // claims them through its own record. This arm names the dead and
            // the coins their removal freed — the persister holds every input
            // of what it deletes, so `released_outpoints` is the only thing
            // that tells it which of those to hand back. It cannot work that
            // out from the txids: the transaction that took the rest may
            // never appear in this wallet's stream at all.
            tracing::debug!(
                swept = txids.len(),
                released = released_outpoints.len(),
                superseded_by = %superseded_by,
                winner_mined_height = ?winner_mined_height,
                "Mirroring swept transactions to the persister"
            );
            CoreChangeSet {
                sweeps: vec![SweepBatch {
                    txids: txids.clone(),
                    superseded_by: *superseded_by,
                    // The winner's finality context rides with the batch:
                    // only the event has it (the winner may never appear in
                    // this wallet's records), and every persister keys the
                    // lifetime of a held-but-unfunded placeholder on it —
                    // `Some` anchors the hold at a height that chainlocks,
                    // `None` (IS-locked, unmined) leaves the hold unstamped
                    // and uncollectible, the durable stand-in for the
                    // `spent_outpoints` retention upstream cannot rebuild
                    // once the loser's record is gone.
                    winner_mined_height: *winner_mined_height,
                    released_outpoints: released_outpoints.clone(),
                }],
                ..CoreChangeSet::default()
            }
        }
        WalletEvent::SyncHeightAdvanced { height, .. } => CoreChangeSet {
            synced_height: Some(*height),
            ..CoreChangeSet::default()
        },
        WalletEvent::ChainLockProcessed { chain_lock, .. } => {
            // The wallet has already promoted the matching records from
            // `InBlock` to `InChainLockedBlock` by the time this event
            // fires (upstream `WalletManager::process_chain_lock` mutates
            // the in-memory map before emitting); our poll loop reads
            // `record.context.is_chain_locked()` directly so we don't
            // mirror per-record promotions here.
            //
            // What we DO persist is the wallet's global
            // `metadata.last_applied_chain_lock` advance. Without this
            // roundtrip, the metadata starts as `None` on every restart
            // and the asset-lock-resume CL-from-metadata fallback in
            // `proof.rs` can't fire until SPV re-applies a fresh
            // ChainLock — wasted latency that the persister-roundtrip
            // collapses to ~zero. SPV persists its own `best_chainlock`
            // independently; this is the symmetric wallet-side
            // persistence, not a re-application.
            //
            // `ChainLockProcessed` fires every time the wallet's
            // `last_applied_chain_lock` advances,
            // even when no record was promoted — so a quiescent wallet's
            // boundary advance is never invisible to this bridge.
            // A `TransactionsChainlocked`-only signal would leave a
            // gap on the "metadata advanced but per-account empty"
            // path; this event closes it deterministically.
            CoreChangeSet {
                last_applied_chain_lock: Some(chain_lock.clone()),
                ..CoreChangeSet::default()
            }
        }
    }
}

/// What one drained `WalletEvent` proved about the Core transaction behind a
/// `Sent` DashPay payment.
///
/// Only two things are ever proven about a broadcast payment, and they are
/// exactly the two terminals a sent entry can reach — which is why the
/// evidence class, not the event variant, is what the transition table below
/// matches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SentPaymentEvidence {
    /// The transaction lost a double-spend: a final rival claimed one of its
    /// inputs, so it can never confirm and the wallet has already dropped its
    /// record.
    Swept,
    /// The transaction reached a context that is final for DashPay display —
    /// mined, or InstantSend-locked.
    Final,
}

/// The complete set of legal `Sent` payment transitions, and the only place a
/// status is chosen for a sent entry.
///
/// | from        | evidence | to          | why this edge exists                                                                 |
/// |-------------|----------|-------------|--------------------------------------------------------------------------------------|
/// | `Pending`   | `Swept`  | `Failed`    | The broadcast lost; nothing else ever writes `Failed`, so without it the entry is stuck `Pending` for good. |
/// | `Confirmed` | `Swept`  | `Failed`    | An IS-locked payment evicted by a chainlocked winner is a dead payment reported as good — the worst of the two. |
/// | `Pending`   | `Final`  | `Confirmed` | The ordinary confirm: mempool → mined / IS-locked.                                    |
/// | `Failed`    | `Final`  | `Confirmed` | A chainlock can reinstate a transaction a sweep removed; without this edge the repair is unreachable. |
///
/// The two remaining pairs — `Failed` + `Swept` and `Confirmed` + `Final` —
/// return `None`: the entry already carries that verdict, so re-emitting it
/// would put an unchanged row on the store round for every re-detection.
///
/// Exhaustive on purpose (no `_` arm): a future [`PaymentStatus`] variant must
/// fail to compile here and be given an explicit edge, rather than be silently
/// swept into — or excluded from — a verdict.
fn next_sent_payment_status(
    current: PaymentStatus,
    evidence: SentPaymentEvidence,
) -> Option<PaymentStatus> {
    use SentPaymentEvidence::{Final, Swept};
    match (current, evidence) {
        (PaymentStatus::Pending, Swept) => Some(PaymentStatus::Failed),
        (PaymentStatus::Confirmed, Swept) => Some(PaymentStatus::Failed),
        (PaymentStatus::Failed, Swept) => None,
        (PaymentStatus::Pending, Final) => Some(PaymentStatus::Confirmed),
        (PaymentStatus::Failed, Final) => Some(PaymentStatus::Confirmed),
        (PaymentStatus::Confirmed, Final) => None,
    }
}

/// The sent-payment evidence `event` carries, as `(txid, evidence)` pairs
/// keyed the way a [`PaymentEntry`](crate::wallet::identity::PaymentEntry) is
/// — by the transaction id's display string.
///
/// Exhaustive on purpose: a new upstream `WalletEvent` variant that says
/// something about a broadcast transaction's fate must fail to compile here
/// rather than be silently dropped.
///
/// `matured` is excluded from `BlockProcessed`: coinbase maturity is never a
/// DashPay payment, and a confirmed record in that bucket says nothing about
/// a sent one.
fn sent_payment_evidence(event: &WalletEvent) -> Vec<(String, SentPaymentEvidence)> {
    /// A record is evidence only once its context is final for DashPay —
    /// the same definition the reconcile sweep uses, so the live path and
    /// the recovery path can never disagree about what "final" means.
    fn finality<'a>(
        records: impl Iterator<Item = &'a TransactionRecord>,
    ) -> Vec<(String, SentPaymentEvidence)> {
        records
            .filter(|record| sent_payment_status_for_record(record) == PaymentStatus::Confirmed)
            .map(|record| (record.txid.to_string(), SentPaymentEvidence::Final))
            .collect()
    }

    match event {
        WalletEvent::TransactionsSwept { txids, .. } => txids
            .iter()
            .map(|txid| (txid.to_string(), SentPaymentEvidence::Swept))
            .collect(),
        // Carries no record, only a txid — and an InstantSend lock is final
        // for DashPay display, so the txid alone is the evidence.
        WalletEvent::TransactionInstantLocked { txid, .. } => {
            vec![(txid.to_string(), SentPaymentEvidence::Final)]
        }
        WalletEvent::TransactionDetected { record, .. } => {
            finality(std::iter::once(record.as_ref()))
        }
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => finality(inserted.iter().chain(updated.iter())),
        WalletEvent::SyncHeightAdvanced { .. } | WalletEvent::ChainLockProcessed { .. } => {
            Vec::new()
        }
    }
}

/// One event's sent-payment verdicts, in the two carriers a round needs.
///
/// Both describe the same flips; neither is redundant.
///
/// * `identities` is the **authoritative** one. `load()` rebuilds a managed
///   identity's `dashpay_payments` from the identity snapshot
///   (`identities.entry_blob` in the SQLite backend), so a verdict that does
///   not ride an [`IdentityEntry`] is silently replaced by the pre-verdict
///   status at the next launch — with the swept transaction gone and nothing
///   able to re-derive it. This is the same pair
///   `ManagedIdentity::record_dashpay_payment` writes on the path this
///   adapter replaced.
/// * `overlay` is the **bounded** one. A full snapshot replays the identity's
///   whole payment history on every flip, which delta-style persisters (the
///   FFI vtable) must not be handed per round; the single-row overlay is what
///   they project instead.
#[derive(Default)]
pub(crate) struct SentPaymentVerdicts {
    /// The changed rows, keyed `(owner, txid)`.
    overlay: PaymentOverlay,
    /// A post-flip [`IdentityEntry`] snapshot per identity whose payments
    /// moved. Merged with [`Merge`], which is keyed by identity id and folds
    /// `dashpay_payments` last-write-wins per txid — so a drain touching one
    /// identity twice still reaches the store as one entry.
    identities: IdentityChangeSet,
}

impl SentPaymentVerdicts {
    /// True when nothing moved, so the round carries no verdict at all. The
    /// two halves are populated together — an identity is snapshotted exactly
    /// when at least one of its rows entered the overlay — so either one
    /// answers, and both are asserted here to keep that coupling honest.
    fn is_empty(&self) -> bool {
        debug_assert_eq!(
            self.overlay.is_empty(),
            Merge::is_empty(&self.identities),
            "a verdict's overlay row and its identity snapshot are written together"
        );
        self.overlay.is_empty() && Merge::is_empty(&self.identities)
    }

    /// Fold `other` in. The overlay takes last-write-wins per `(owner, txid)`
    /// via [`merge_payment_overlays`]; the snapshots take
    /// [`IdentityChangeSet`]'s own merge, whose `dashpay_payments` fold is
    /// last-write-wins per txid as well — so a transaction swept and then
    /// reinstated inside one drain reaches the store once, as the verdict the
    /// drain ended on, in both carriers.
    fn merge(&mut self, other: Self) {
        merge_payment_overlays(&mut self.overlay, other.overlay);
        self.identities.merge(other.identities);
    }
}

/// Resolve `event`'s sent-payment evidence against the wallet's live payment
/// entries, flip the ones the transition table moves, and return them as a
/// ready overlay for this drain's `store()` round.
///
/// # Why the adapter owns this and the payment handler does not
///
/// The handler runs off dash-spv's *lossy* broadcast bus, which drops events
/// under `RecvError::Lagged` during catch-up. A sweep dropped there is
/// unrecoverable: `drop_conflicted_transactions` selects its losers from the
/// live in-memory records and deletes them in the same call, so the sweep
/// never re-emits and the reconcile pass — which resolves against a record
/// that no longer exists — gives up. This adapter drains the *lossless*
/// persistence channel, so the verdict rides the same `store()` as the row
/// removal that implies it.
///
/// # Failure posture
///
/// The flip lands in memory here and in the store when the round commits. A
/// rejected round leaves the two disagreeing until the next launch, and that
/// is deliberate: the wallet is faulted and its watermark frozen by the same
/// rejection, so the next launch reloads memory from the store and re-emits
/// the sweep from the frozen watermark, which re-derives the verdict. A
/// rollback ledger would only defend a divergence that cannot outlive the
/// session that caused it.
pub(crate) async fn sent_payment_verdicts(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    event: &WalletEvent,
) -> SentPaymentVerdicts {
    let mut overlay = SentPaymentVerdicts::default();
    let evidence = sent_payment_evidence(event);
    if evidence.is_empty() {
        return overlay;
    }
    let wallet_id = event.wallet_id();

    // Probe under a read lock first. During a catch-up nearly every block
    // carries final records and almost none of them are DashPay payments, so
    // this keeps the write lock — which contends with SPV's own wallet
    // mutations on the hot path — for rounds that genuinely have a verdict to
    // write.
    {
        let wm = wallet_manager.read().await;
        let Some(info) = wm.get_wallet_info(&wallet_id) else {
            return overlay;
        };
        let any_verdict = info
            .identity_manager
            .identity_ids()
            .into_iter()
            .any(|owner| {
                info.identity_manager
                    .managed_identity(&owner)
                    .is_some_and(|managed| {
                        evidence.iter().any(|(txid, evidence)| {
                            managed
                                .dashpay()
                                .payments
                                .get(txid)
                                .is_some_and(|entry| verdict_for(entry, *evidence).is_some())
                        })
                    })
            });
        if !any_verdict {
            return overlay;
        }
    }

    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(&wallet_id) else {
        return overlay;
    };
    for owner in info.identity_manager.identity_ids() {
        let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
            continue;
        };
        // The replay/restore accessor, deliberately: the live
        // `record_dashpay_payment` writer persists on its own round, which is
        // the one thing this fix exists to avoid. The overlay returned here
        // carries the same row onto the adapter's round instead.
        let payments = managed.dashpay_payments_mut();
        let mut flipped_any = false;
        for (txid, evidence) in &evidence {
            let Some(entry) = payments.get_mut(txid) else {
                continue;
            };
            let Some(next) = verdict_for(entry, *evidence) else {
                continue;
            };
            tracing::info!(
                %owner,
                %txid,
                from = ?entry.status,
                to = ?next,
                "Sent DashPay payment verdict"
            );
            entry.status = next;
            flipped_any = true;
            overlay
                .overlay
                .entry(owner)
                .or_default()
                .insert(txid.clone(), entry.clone());
        }
        if flipped_any {
            // Taken AFTER the flips above, so the snapshot carries the verdict
            // rather than the status it replaced. This is the authoritative
            // half: `load()` rehydrates a managed identity's payments from the
            // identity blob this entry encodes and never reads the overlay
            // table back (see the sqlite `dashpay` module's own header), so a
            // round carrying only the overlay is a verdict that does not
            // survive a restart.
            overlay
                .identities
                .identities
                .insert(owner, IdentityEntry::from_managed(managed));
        }
    }
    overlay
}

/// The status `entry` moves to under `evidence`, or `None` if it does not
/// move. Received entries never move: their status is settled at the moment
/// they are recorded from an on-chain sighting, and a sweep of an unrelated
/// spend must not touch one.
fn verdict_for(
    entry: &crate::wallet::identity::PaymentEntry,
    evidence: SentPaymentEvidence,
) -> Option<PaymentStatus> {
    if entry.direction != PaymentDirection::Sent {
        return None;
    }
    next_sent_payment_status(entry.status, evidence)
}

/// Rebuild the "addresses marked used" delta plus the post-batch
/// highest-used watermarks for the accounts touched by `records`.
///
/// Upstream `wallet_checker` marks matched addresses used (and bumps
/// each pool's `highest_used`) **in memory only** — `WalletEvent`
/// carries neither. Without re-deriving the flips here, a match found
/// during SPV block processing (a TXO on a BIP44 address, or a
/// special-tx payload hitting a provider owner / voting key) never
/// reaches the persister and every mirrored store keeps
/// `is_used = false` / `highest_used = None` forever.
///
/// Returns empty deltas when the wallet is unknown (raced a removal)
/// — the next sync round re-emits. See
/// [`collect_usage_deltas_from_accounts`] for the derivation itself.
async fn collect_usage_deltas(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    records: Vec<&TransactionRecord>,
) -> (
    Vec<DerivedAddressInfo>,
    BTreeMap<AccountType, HighestUsedIndexes>,
) {
    if records.is_empty() {
        return (Vec::new(), BTreeMap::new());
    }
    let guard = wallet_manager.read().await;
    let Some(info) = guard.get_wallet_info(wallet_id) else {
        return (Vec::new(), BTreeMap::new());
    };
    collect_usage_deltas_from_accounts(&info.core_wallet.accounts, &records)
}

/// The engine's credit verdict for every `Received` / `Change` output of
/// `records` that the owning account does NOT hold — see
/// [`CoreChangeSet::utxo_credit_verdicts`] for what a persister does with
/// it. Empty when every output is credited, when the wallet is unknown
/// (raced a removal — the next round re-emits), or when `records` is empty.
///
/// One read of the wallet lock per event, like [`collect_usage_deltas`];
/// the walk itself is [`utxo_credit_verdicts_from_wallet`], factored so
/// tests can drive it against a bare `ManagedWalletInfo`. `BlockProcessed`
/// uses this over the event's own records (block records carry their
/// block context and cannot read `Doomed`); `TransactionDetected` reads
/// its slices and their verdicts under one guard through
/// [`wallet_slices_and_verdicts_for_txid`].
async fn utxo_credit_verdicts(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    records: &[&TransactionRecord],
) -> BTreeMap<OutPoint, UtxoCreditVerdict> {
    if records.is_empty() {
        return BTreeMap::new();
    }
    let guard = wallet_manager.read().await;
    let Some(info) = guard.get_wallet_info(wallet_id) else {
        return BTreeMap::new();
    };
    utxo_credit_verdicts_from_wallet(&info.core_wallet, records)
}

/// Synchronous core of [`utxo_credit_verdicts`].
///
/// For each record (a contact's watch-only slice excluded — its outputs
/// are the contact's coins and never become this wallet's TXOs) and each
/// output the record classifies `Received` / `Change`, the owning account
/// is resolved by the record's `account_type` and the outpoint looked up
/// in its live `utxos`:
///
/// - present → credited, no verdict (the ordinary case);
/// - absent and the wallet's `observed_spent_outpoints` (#649) names the
///   outpoint → [`UtxoCreditVerdict::ObservedSpent`] with that height:
///   `update_utxos` skipped the insert because a block already spent it,
///   and the spender may be a transaction the wallet never recorded
///   (rust-dashcore#992);
/// - absent, the record unconfirmed, and one of its own inputs observed
///   spent → [`UtxoCreditVerdict::Doomed`]: `doomed_by_a_settled_spend`
///   credited nothing, and the conflict sweep that would delete the row
///   fired on the winner's arrival and will not fire again;
/// - absent otherwise → [`UtxoCreditVerdict::Uncredited`]: the coin was
///   taken between emit and drain (a spend, an abandon, a sweep) or holds
///   an account-level spent mark; the store learns the rest from the
///   spender's own record or the sweep callback.
///
/// `utxos` membership is the gate, not the reason: a coin the engine holds
/// is credited whatever the observed-spent map says (an IS-locked loser
/// under DIP-10 precedence keeps its outputs credited, and must not be
/// flagged). The verdict is evaluated against the wallet as it is at drain
/// time, which is later than the record — that is the same lag every
/// other delta this bridge derives already has, and a coin spent in a
/// block since the record was built reads `ObservedSpent` by the same
/// evidence the engine used to drop it. The one thing that must NOT lag
/// is the record's context relative to the wallet it is judged against:
/// callers hand in records read from the same wallet snapshot (the
/// manager's own slices under one guard), never a clone from an earlier
/// one.
fn utxo_credit_verdicts_from_wallet(
    core_wallet: &key_wallet::wallet::ManagedWalletInfo,
    records: &[&TransactionRecord],
) -> BTreeMap<OutPoint, UtxoCreditVerdict> {
    let mut verdicts = BTreeMap::new();
    let observed = core_wallet.observed_spent_outpoints();
    let accounts = core_wallet.accounts.all_accounts();
    for record in records {
        if is_contact_watch_only(record) {
            continue;
        }
        let Some(funds) = accounts
            .iter()
            .find(|a| a.managed_account_type().to_account_type() == record.account_type)
            .and_then(|a| a.as_funds())
        else {
            continue;
        };
        let doomed = matches!(record.context, TransactionContext::Mempool)
            && record
                .transaction
                .input
                .iter()
                .any(|input| observed.contains_key(&input.previous_output));
        for detail in &record.output_details {
            if !matches!(detail.role, OutputRole::Received | OutputRole::Change) {
                continue;
            }
            let outpoint = OutPoint {
                txid: record.txid,
                vout: detail.index,
            };
            if funds.utxos.contains_key(&outpoint) {
                continue;
            }
            let verdict = if let Some(height) = observed.get(&outpoint) {
                UtxoCreditVerdict::ObservedSpent { height: *height }
            } else if doomed {
                UtxoCreditVerdict::Doomed
            } else {
                UtxoCreditVerdict::Uncredited
            };
            verdicts.insert(outpoint, verdict);
        }
    }
    verdicts
}

/// Synchronous core of [`collect_usage_deltas`], factored over the
/// account collection so tests can drive it without a `WalletManager`.
///
/// Usage-site candidates for each record come from two complementary
/// sources:
///
/// 1. **The record's own `input_details` / `output_details`.**
///    `input_details` entries are wallet-owned by construction and are
///    the ONLY reliable source for spent-input addresses: by the time
///    this bridge runs, `record_transaction`'s UTXO update has already
///    removed the spent outpoints from the live account, so a replayed
///    match can no longer see them. Output details are filtered to the
///    `Received` / `Change` roles (a `Sent` detail carries the
///    counterparty's address).
/// 2. **A replayed read-only match**
///    ([`ManagedAccountCollection::check_transaction`], scoped to the
///    record's account type), which covers involvement the details
///    don't carry — most importantly special-tx payload matches: a
///    ProRegTx hitting a provider owner / voting key produces a record
///    with no input/output detail for the matched key.
///
/// Every candidate is then resolved against the live pools for the
/// authoritative post-mark
/// [`AddressInfo`](key_wallet::managed_account::address_pool::AddressInfo)
/// and its `(account_type, pool_type)`; candidates no pool monitors
/// (counterparty addresses) simply drop out. `used` is forced `true`
/// on the emitted entry — involvement in a recorded transaction is the
/// definition of "used", independent of snapshot timing. Highest-used
/// watermarks are snapshotted once per touched account at the end.
fn collect_usage_deltas_from_accounts(
    accounts: &key_wallet::managed_account::managed_account_collection::ManagedAccountCollection,
    records: &[&TransactionRecord],
) -> (
    Vec<DerivedAddressInfo>,
    BTreeMap<AccountType, HighestUsedIndexes>,
) {
    let account_refs = accounts.all_accounts();
    // Hoisted `(account_type, pools)` snapshot — `address_pools()`
    // allocates a fresh Vec per call, so build it once per batch
    // instead of once per (candidate address × account).
    let pools_by_account: Vec<(AccountType, Vec<&AddressPool>)> = account_refs
        .iter()
        .map(|a| {
            (
                a.managed_account_type().to_account_type(),
                a.managed_account_type().address_pools(),
            )
        })
        .collect();

    let mut marked_used: Vec<DerivedAddressInfo> = Vec::new();
    let mut seen: HashSet<(AccountType, AddressPoolType, u32)> = HashSet::new();
    let mut touched: HashSet<AccountType> = HashSet::new();
    // A block can carry several records for the same account type
    // (and `check_transaction` is per-tx anyway), so dedup the
    // (txid, type-to-check) pairs to avoid re-matching the same
    // transaction against the same accounts. `AccountTypeToCheck`
    // isn't `Hash` upstream; a linear scan over this per-batch-sized
    // vec is cheaper than hashing anyway.
    let mut checked: Vec<(dashcore::Txid, AccountTypeToCheck)> = Vec::new();

    for record in records {
        // Source 1: the record's own details (see doc comment). These
        // survive the UTXO-set mutation that precedes this bridge.
        let mut candidates: Vec<dashcore::Address> = record
            .input_details
            .iter()
            .map(|detail| detail.address.clone())
            .collect();
        candidates.extend(record.output_details.iter().filter_map(|detail| {
            matches!(detail.role, OutputRole::Received | OutputRole::Change)
                .then(|| detail.address.clone())
                .flatten()
        }));

        // Source 2: replayed read-only match for involvement the
        // details don't carry (special-tx payload matches). Account
        // types with no Core-chain matcher (`PlatformPayment`) skip
        // the replay and rely on the details alone.
        if let Ok(type_to_check) = AccountTypeToCheck::try_from(record.account_type) {
            if !checked.contains(&(record.txid, type_to_check)) {
                checked.push((record.txid, type_to_check));
                let result = accounts.check_transaction(&record.transaction, &[type_to_check]);
                for account_match in result.affected_accounts {
                    candidates.extend(
                        account_match
                            .account_type_match
                            .all_involved_addresses()
                            .into_iter()
                            .map(|involved| involved.address),
                    );
                }
            }
        }

        // Resolve every candidate back to its owning account + pool.
        // This recovers the `pool_type` (which neither source carries)
        // and the authoritative post-mark `AddressInfo`; any account
        // monitoring the address is a genuine usage site, matching
        // upstream's per-matched-account `mark_address_used` sweep.
        for address in candidates {
            for (owner_type, pools) in &pools_by_account {
                for pool in pools {
                    let Some(pool_info) = pool.address_info(&address) else {
                        continue;
                    };
                    touched.insert(*owner_type);
                    if seen.insert((*owner_type, pool.pool_type, pool_info.index)) {
                        let mut info = pool_info.clone();
                        info.state = AddressState::Used;
                        marked_used.push(DerivedAddressInfo {
                            account_type: *owner_type,
                            pool_type: pool.pool_type,
                            info,
                        });
                    }
                }
            }
        }
    }

    // Snapshot highest-used watermarks once per touched account.
    // Standard accounts map External / Internal onto the two persisted
    // slots; single-pool accounts (provider keys, identity funding)
    // surface theirs as `external`. Folding with max keeps the
    // invariant if an account ever grows several non-internal pools.
    let mut highest_used: BTreeMap<AccountType, HighestUsedIndexes> = BTreeMap::new();
    for (owner_type, pools) in &pools_by_account {
        if !touched.contains(owner_type) {
            continue;
        }
        let mut snapshot = HighestUsedIndexes::default();
        for pool in pools {
            let slot = if pool.is_internal() {
                &mut snapshot.internal
            } else {
                &mut snapshot.external
            };
            *slot = match (*slot, pool.highest_used) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
        }
        highest_used
            .entry(*owner_type)
            .or_default()
            .merge_max(snapshot);
    }

    (marked_used, highest_used)
}

/// Returns `true` when the wallet's stored record for `txid` is in a
/// chain-locked block. Used to gate IS-lock projection.
async fn is_chain_locked(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
) -> bool {
    let guard = wallet_manager.read().await;
    let Some(info) = guard.get_wallet_info(wallet_id) else {
        return false;
    };
    // Walk every account; if any holds an in-memory record for this
    // txid, the chain-lock determination falls out of its
    // `TransactionContext`. With `keep-finalized-transactions` off
    // (the default) `transactions()` returns an empty map regardless
    // of state — chain-lock delivery is event-driven in that mode, and
    // this helper just reports "no record locally" by returning false.
    for account in info.core_wallet.accounts.all_accounts() {
        if let Some(record) = account.transactions().get(txid) {
            return matches!(record.context, TransactionContext::InChainLockedBlock(_));
        }
    }
    false
}

/// Every account slice the manager currently holds for `txid` in
/// `wallet_id` — the authoritative "all accounts matched so far"
/// snapshot behind the wallet-level fold (see the `TransactionDetected`
/// arm of [`build_core_changeset`]) — together with the credit verdicts
/// of the owned slices' outputs ([`utxo_credit_verdicts_from_wallet`]),
/// both read under ONE wallet read guard, so the records a verdict is
/// judged on and the wallet state it is judged against are the same
/// snapshot. Returns `None` when the manager doesn't know the wallet at
/// all, `Some((vec![], empty))` when it does but no account holds a record
/// for the txid (e.g. pruned at chain-lock).
async fn wallet_slices_and_verdicts_for_txid(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
) -> Option<(
    Vec<TransactionRecord>,
    BTreeMap<OutPoint, UtxoCreditVerdict>,
)> {
    let guard = wallet_manager.read().await;
    let info = guard.get_wallet_info(wallet_id)?;
    let mut slices = Vec::new();
    for account in info.core_wallet.accounts.all_accounts() {
        if let Some(record) = account.transactions().get(txid) {
            slices.push(record.clone());
        }
    }
    let owned: Vec<&TransactionRecord> = slices
        .iter()
        .filter(|r| !is_contact_watch_only(r))
        .collect();
    let verdicts = utxo_credit_verdicts_from_wallet(&info.core_wallet, &owned);
    Some((slices, verdicts))
}

/// Is this record owned by a contact's watch-only DashPay chain?
///
/// A `DashpayExternalAccount` derives its addresses from the
/// **contact's** xpub, so this wallet can observe those outputs but can
/// never sign for them. They are the contact's coins; this wallet only
/// ever pays into them.
///
/// dashpay/rust-dashcore#926 established exactly that policy at the
/// balance layer, dropping `dashpay_external_accounts` from
/// `ManagedAccountCollection::all_funding_accounts` (and `_mut`), which
/// covers `balance`, `account_balances`, `utxos` and
/// `get_spendable_utxos` in one place. `dashpay_receival_accounts` were
/// deliberately kept: those derive from *our* xpub, so a contact paying
/// into them really is money arriving.
///
/// The persistence seam is the same rule's second home. Upstream
/// `check_core_transaction` emits **one `TransactionRecord` per matched
/// account** (`key_wallet::transaction_checking::wallet_checker`), so a
/// payment to a contact produces two records sharing one txid:
///
/// | record's account          | `direction` | `net_amount`     |
/// |---------------------------|-------------|------------------|
/// | funding (BIP44/BIP32/…)   | `Outgoing`  | `change - spent` |
/// | `DashpayExternalAccount`  | `Incoming`  | `+paid`          |
///
/// The external account's record is not *wrong* about its own account —
/// that chain did receive an output. It is wrong as a description of the
/// **wallet**, and the persisted `transactions` row is keyed by txid
/// alone, with no per-account dimension to disambiguate (the
/// `transaction_account_involvements` table is only written for
/// provider-key accounts). Whichever record is stored last therefore
/// defines the row, and the watch-only one — emitted last, because
/// `all_accounts` visits the DashPay accounts after the standard ones —
/// wins: a payment *away* is persisted as an incoming credit, and its
/// output is written into `txos` as a wallet-owned coin no key of ours
/// can spend.
///
/// So external-account records are excluded from the persist-time
/// projection entirely, exactly as #926 excluded the accounts from
/// balance aggregation. What survives from the same event is everything
/// that is genuinely ours to remember: the address-used flips and
/// highest-used watermarks (so contact address rotation keeps working),
/// the derived-address rows, and `derive_spent_utxos` (so a contact
/// spending an output persisted *before* this fix still clears the
/// stale row).
///
/// The predicate itself is canonical upstream
/// ([`AccountType::is_contact_owned`], dashpay/rust-dashcore#952): its
/// exhaustive match forces any future account type to declare whether
/// its coins are the wallet's or a contact's, so this seam and #926's
/// cannot drift apart.
fn is_contact_watch_only(record: &TransactionRecord) -> bool {
    record.account_type.is_contact_owned()
}

/// Derive the "ours" UTXOs created by a transaction's outputs.
///
/// Walks `record.output_details`, keeps entries with role `Received` or
/// `Change`, and reconstructs a full `Utxo` from the corresponding
/// `transaction.output[index]` plus the record's confirmation context.
///
/// Records belonging to a contact's watch-only chain yield nothing —
/// see [`is_contact_watch_only`]. This is the direct counterpart of
/// dashpay/rust-dashcore#926 dropping those accounts from `utxos()` /
/// `get_spendable_utxos()`.
fn derive_new_utxos(record: &TransactionRecord) -> Vec<Utxo> {
    if is_contact_watch_only(record) {
        return Vec::new();
    }
    let height = record.context.block_info().map(|b| b.height()).unwrap_or(0);
    let is_confirmed = matches!(
        record.context,
        TransactionContext::InBlock(_) | TransactionContext::InChainLockedBlock(_)
    );
    let is_instant = matches!(record.context, TransactionContext::InstantSend(_));
    let is_coinbase = record.transaction.is_coin_base();
    // We own at least one input iff the wallet recorded any input details
    // (those entries are keyed to inputs that spent our outpoints).
    let owns_any_input = !record.input_details.is_empty();

    record
        .output_details
        .iter()
        .filter_map(|detail| {
            if !matches!(detail.role, OutputRole::Received | OutputRole::Change) {
                return None;
            }
            let txout = record
                .transaction
                .output
                .get(detail.index as usize)?
                .clone();
            let address = detail.address.clone()?;
            // Mirror key-wallet's "trusted change" rule: change output of a
            // transaction we authored (so it's our funds returning).
            let is_trusted = matches!(detail.role, OutputRole::Change) && owns_any_input;
            Some(Utxo {
                outpoint: OutPoint {
                    txid: record.txid,
                    vout: detail.index,
                },
                txout,
                address,
                height,
                is_coinbase,
                is_confirmed,
                is_instantlocked: is_instant,
                is_locked: false,
                is_trusted,
            })
        })
        .collect()
}

/// Derive the "ours" UTXOs spent by a transaction's inputs.
///
/// Walks `record.input_details` (the entries keyed to inputs that spent
/// our outpoints) and synthesizes a `Utxo` per entry: the outpoint from
/// `transaction.input[index].previous_output`, the value and address from
/// `InputDetail`, and the locking script rebuilt from that address.
///
/// The script is an exact reconstruction, not a guess. `InputDetail.address`
/// is cloned from the wallet's own `Utxo.address`, which key-wallet derived
/// from the spent output's `script_pubkey` via `Address::from_script`; that
/// decoder accepts canonical P2PKH, P2SH, and witness-program forms, so
/// re-encoding the address reproduces the original bytes. Note the pairing
/// is a caller convention rather than a type invariant — `Utxo::new` takes
/// the script and the address as independent parameters and validates
/// neither.
///
/// Height and the confirmation flags describe the *previous* transaction and
/// aren't carried in `InputDetail`, so they remain defaulted on this synthetic
/// spent record (height 0, all flags false).
fn derive_spent_utxos(record: &TransactionRecord) -> Vec<Utxo> {
    record
        .input_details
        .iter()
        .filter_map(|detail| {
            let outpoint = spent_outpoint(record, detail)?;
            Some(Utxo {
                outpoint,
                txout: TxOut {
                    value: detail.value,
                    script_pubkey: detail.address.script_pubkey(),
                },
                address: detail.address.clone(),
                height: 0,
                is_coinbase: false,
                is_confirmed: false,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            })
        })
        .collect()
}

/// The outpoint one [`InputDetail`] says this record spent, or `None` when the
/// detail's index does not address a real input.
///
/// The single definition of "this record spent one of ours", shared by
/// [`derive_spent_utxos`] above — which turns it into the persister's
/// [`CoreChangeSet::spent_utxos`] removals — and by
/// [`spent_outpoints`], which drives the in-broadcast fence's release. The two
/// consumers must not be able to disagree about which inputs count: the fence
/// releases an outpoint precisely when the wallet treats it as spent, so a
/// divergence would either strand a fence forever or drop one early.
///
/// [`InputDetail`]: key_wallet::managed_account::transaction_record::InputDetail
fn spent_outpoint(
    record: &TransactionRecord,
    detail: &key_wallet::managed_account::transaction_record::InputDetail,
) -> Option<OutPoint> {
    record
        .transaction
        .input
        .get(detail.index as usize)
        .map(|input| input.previous_output)
}

/// Every outpoint of ours that `record` spends.
///
/// The fence-side view of [`derive_spent_utxos`], built on the same
/// [`spent_outpoint`] walk — see that function for why they share it.
pub(crate) fn spent_outpoints(record: &TransactionRecord) -> impl Iterator<Item = OutPoint> + '_ {
    record
        .input_details
        .iter()
        .filter_map(|detail| spent_outpoint(record, detail))
}

impl CoreChangeSet {
    /// Cheap "should we bother round-tripping the persister" check used
    /// by the adapter to drop empty events without locking. Skips the
    /// `is_empty()` walk over `instant_locks_for_non_final_records`
    /// since that map is rarely populated and `Vec::is_empty` short-
    /// circuits on the common case.
    fn is_empty_no_records(&self) -> bool {
        self.records.is_empty()
            && self.account_records.is_empty()
            && self.sweeps.is_empty()
            && self.spent_utxos.is_empty()
            && self.new_utxos.is_empty()
            && self.instant_locks_for_non_final_records.is_empty()
            && self.last_processed_height.is_none()
            && self.synced_height.is_none()
            && self.last_applied_chain_lock.is_none()
            && self.addresses_derived.is_empty()
            && self.addresses_marked_used.is_empty()
            && self.account_highest_used.is_empty()
            && self.utxo_credit_verdicts.is_empty()
    }
}

#[cfg(test)]
mod swept_transaction_projection_tests {
    //! Coverage for the one subtractive arm of [`build_core_changeset`].
    //!
    //! A sweep carries txids and no records, so it has to survive the
    //! `is_empty_no_records` filter on the strength of the txids alone —
    //! that filter is what decides whether the persister is called at all,
    //! and a sweep that never reaches it leaves the dead rows on disk.

    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::Txid;
    use key_wallet::WalletCoreBalance;
    use key_wallet_manager::WalletManager;

    const WALLET_ID: WalletId = [7u8; 32];

    fn test_manager() -> Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            dashcore::Network::Testnet,
        )))
    }

    fn txid(byte: u8) -> Txid {
        Txid::from_byte_array([byte; 32])
    }

    fn outpoint(byte: u8, vout: u32) -> OutPoint {
        OutPoint {
            txid: txid(byte),
            vout,
        }
    }

    /// A minimal record for `txid` — only its identity matters here, since
    /// the merge keys reinstatement on the txid alone.
    fn record_for(txid: Txid) -> TransactionRecord {
        let tx = dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let mut record = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            key_wallet::transaction_checking::transaction_router::TransactionType::Standard,
            key_wallet::managed_account::transaction_record::TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        );
        record.txid = txid;
        record
    }

    /// Mined height every block-context sweep event in these tests carries.
    const WINNER_HEIGHT: u32 = 700;

    fn swept(txids: Vec<Txid>) -> WalletEvent {
        swept_releasing(txids, vec![])
    }

    fn swept_releasing(txids: Vec<Txid>, released_outpoints: Vec<OutPoint>) -> WalletEvent {
        WalletEvent::TransactionsSwept {
            wallet_id: WALLET_ID,
            txids,
            superseded_by: txid(0xff),
            winner_mined_height: Some(WINNER_HEIGHT),
            released_outpoints,
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn sweep_names_the_dead_transactions_and_nothing_else() {
        let cs = build_core_changeset(&test_manager(), &swept(vec![txid(1), txid(2)])).await;

        assert_eq!(
            cs.sweeps,
            vec![SweepBatch {
                txids: vec![txid(1), txid(2)],
                superseded_by: txid(0xff),
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![],
            }]
        );
        // A wallet-relevant winner claims the inputs through its own
        // record; this arm must not invent UTXO deltas of its own.
        assert!(cs.records.is_empty(), "a sweep carries no records");
        assert!(cs.spent_utxos.is_empty(), "a sweep spends nothing");
        assert!(cs.new_utxos.is_empty(), "a sweep creates nothing");
    }

    /// An IS-locked winner's sweep carries `winner_mined_height: None`
    /// through to the batch untouched. Every persister keys the lifetime of
    /// a held-but-unfunded placeholder on this field — a bridge that
    /// fabricated a height here would hand the placeholder a finality
    /// horizon the winner does not have, and one that dropped the `Some`
    /// leg would make block-context holds uncollectible.
    #[tokio::test]
    async fn sweep_carries_the_winners_finality_context_verbatim() {
        let event = WalletEvent::TransactionsSwept {
            wallet_id: WALLET_ID,
            txids: vec![txid(1)],
            superseded_by: txid(0xff),
            winner_mined_height: None,
            released_outpoints: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        };
        let cs = build_core_changeset(&test_manager(), &event).await;
        assert_eq!(
            cs.sweeps[0].winner_mined_height, None,
            "an unmined IS-locked winner must cross the bridge with no mined height"
        );
    }

    #[tokio::test]
    async fn sweep_reaches_the_persister() {
        let cs = build_core_changeset(&test_manager(), &swept(vec![txid(1)])).await;

        assert!(
            !cs.is_empty_no_records(),
            "a sweep-only round must not be filtered out as empty — that \
             filter decides whether the persister is called at all"
        );
        assert!(!Merge::is_empty(&cs));
    }

    /// The released set is what a persister acts on, so it has to survive
    /// the projection intact — it cannot be recovered from the txids, since
    /// the transaction that took the remaining inputs may never appear here.
    #[tokio::test]
    async fn sweep_carries_the_outpoints_it_released() {
        let cs = build_core_changeset(
            &test_manager(),
            &swept_releasing(vec![txid(1)], vec![outpoint(9, 1)]),
        )
        .await;

        assert_eq!(cs.sweeps[0].released_outpoints, vec![outpoint(9, 1)]);
    }

    /// An ordinary resend frees nothing: the winner took every input the
    /// removed transaction named.
    #[tokio::test]
    async fn a_sweep_that_freed_nothing_releases_nothing() {
        let cs = build_core_changeset(&test_manager(), &swept(vec![txid(1)])).await;

        assert!(cs.sweeps[0].released_outpoints.is_empty());
    }

    /// Merging keeps every sweep as its own batch, in arrival order.
    ///
    /// Folding them would lose the only thing that makes a later sweep able
    /// to correct an earlier one — see the ordering test below, which is the
    /// case that actually breaks.
    #[tokio::test]
    async fn merged_sweeps_stay_separate_and_ordered() {
        let mut cs = build_core_changeset(&test_manager(), &swept(vec![txid(1), txid(2)])).await;
        let second = build_core_changeset(&test_manager(), &swept(vec![txid(3)])).await;

        cs.merge(second);

        assert_eq!(cs.sweeps.len(), 2);
        assert_eq!(cs.sweeps[0].txids, vec![txid(1), txid(2)]);
        assert_eq!(cs.sweeps[1].txids, vec![txid(3)]);
    }

    /// A record arriving after a sweep of the same transaction reinstates
    /// it. Every persister writes records before replaying sweeps, so a
    /// buffered sweep would otherwise delete a row the wallet has since
    /// brought back.
    ///
    /// Reachable through IS-lock precedence: an unconfirmed transaction is
    /// swept when an IS-locked conflict arrives, then returns chainlocked
    /// and sweeps that conflict in turn — leaving one round holding both
    /// removals plus the reinstating record.
    #[tokio::test]
    async fn a_record_arriving_after_its_sweep_survives_the_round() {
        let reinstated = txid(1);

        let mut cs = build_core_changeset(
            &test_manager(),
            &swept_releasing(vec![reinstated], vec![outpoint(9, 1)]),
        )
        .await;
        assert_eq!(
            cs.sweeps.len(),
            1,
            "sanity: the sweep is there to begin with"
        );

        // The wallet records it again, which is the newer fact.
        let mut later = CoreChangeSet::default();
        later.records.push(record_for(reinstated));
        cs.merge(later);

        assert!(
            cs.sweeps.is_empty(),
            "the sweep must not delete a transaction the wallet brought back"
        );
        assert_eq!(cs.records.len(), 1);
    }

    /// Only the reinstated transaction leaves the batch; anything else it
    /// removed still goes — and so does everything that batch freed.
    ///
    /// `released_outpoints` is the aggregate for every loser in the batch, so
    /// dropping it would discard coins freed by the losers still going. The
    /// entries belonging to the reinstated transaction do no harm: every
    /// backend either scopes its release to the remaining losers' own inputs
    /// or withholds an outpoint a surviving record claims, and the
    /// reinstating record is exactly such a claim.
    #[tokio::test]
    async fn a_reinstated_record_only_rescues_its_own_transaction() {
        let reinstated = txid(1);
        let still_dead = txid(2);
        let freed_by_the_survivor = outpoint(9, 2);

        let mut cs = build_core_changeset(
            &test_manager(),
            &swept_releasing(vec![reinstated, still_dead], vec![freed_by_the_survivor]),
        )
        .await;
        let mut later = CoreChangeSet::default();
        later.records.push(record_for(reinstated));
        cs.merge(later);

        assert_eq!(cs.sweeps.len(), 1);
        assert_eq!(cs.sweeps[0].txids, vec![still_dead]);
        assert_eq!(
            cs.sweeps[0].released_outpoints,
            vec![freed_by_the_survivor],
            "a coin the still-swept loser freed must survive the reinstatement"
        );
    }

    /// A release is only true of the wallet the sweep that made it saw. A
    /// later sweep can remove the transaction that re-spent the freed coin
    /// while keeping the coin spent, because its own winner took it — and
    /// that answer has to win, since it is the later one.
    ///
    /// Unioning the release sets loses exactly this: the earlier "B is free"
    /// outlives the later "B is spent", and every backend then persists a
    /// coin the chain consumed as spendable.
    #[tokio::test]
    async fn a_later_sweep_that_keeps_a_coin_spent_outlives_an_earlier_release() {
        let freed = outpoint(9, 1);

        let mut cs = build_core_changeset(
            &test_manager(),
            &swept_releasing(vec![txid(1)], vec![freed]),
        )
        .await;
        // The second sweep removes the transaction that took `freed` and
        // releases nothing: its own winner consumed that coin.
        let second =
            build_core_changeset(&test_manager(), &swept_releasing(vec![txid(2)], vec![])).await;

        cs.merge(second);

        assert_eq!(
            cs.sweeps.len(),
            2,
            "the two answers must stay distinguishable"
        );
        assert_eq!(cs.sweeps[0].released_outpoints, vec![freed]);
        assert!(
            cs.sweeps[1].released_outpoints.is_empty(),
            "the later sweep kept the coin spent, and applying it after the \
             first is what makes that stick"
        );
    }
}

#[cfg(test)]
mod sent_payment_verdict_tests {
    //! Coverage for the sent-payment verdicts the adapter owns.
    //!
    //! Nothing else in the wallet writes `PaymentStatus::Failed`, and the
    //! two events that prove one — a sweep, and the finality that can undo
    //! it — never re-emit once their round is durable. So these pin both
    //! terminals, and that the verdict comes back as a ready overlay for the
    //! same `store()` round rather than a separate write.

    use super::*;
    use dashcore::ephemerealdata::instant_lock::InstantLock;
    use dashcore::hashes::Hash as _;
    use dashcore::{BlockHash, Transaction, TxIn, Txid};
    use dpp::identity::{Identity, IdentityV0};
    use dpp::prelude::Identifier;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{BlockInfo, TransactionType};
    use key_wallet::WalletCoreBalance;

    use crate::changeset::traits::PlatformWalletPersistence;
    use crate::test_support::{funded_wallet_manager, NoopTestPersister};
    use crate::wallet::identity::{PaymentDirection, PaymentEntry, PaymentStatus};
    use crate::wallet::persister::WalletPersister;

    const OWNER: [u8; 32] = [0xAA; 32];
    const CONTACT: [u8; 32] = [0xBB; 32];

    pub(super) fn owner() -> Identifier {
        Identifier::from(OWNER)
    }

    /// The spend whose payment entry every test below flips. A real
    /// `Transaction` rather than a bare txid, so the record used as finality
    /// evidence and the entry agree on the same key.
    pub(super) fn sent_transaction() -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([0x5f; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    /// A wallet holding one identity with a single `Sent` payment at
    /// `status`, keyed by `sent_transaction()`'s txid.
    pub(super) async fn wallet_with_payment(
        direction: PaymentDirection,
        status: PaymentStatus,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
        String,
    ) {
        let (wallet_manager, wallet_id, _generation, _signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let txid = sent_transaction().txid().to_string();
        let persister = WalletPersister::new(
            wallet_id,
            Arc::new(NoopTestPersister) as Arc<dyn PlatformWalletPersistence>,
        );
        {
            let mut wm = wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id: owner(),
                        public_keys: BTreeMap::new(),
                        balance: 0,
                        revision: 0,
                    }),
                    0,
                    wallet_id,
                    &persister,
                )
                .expect("add owner identity");
            let mut entry = match direction {
                PaymentDirection::Sent => {
                    PaymentEntry::new_sent(Identifier::from(CONTACT), 50_000, Some("lunch".into()))
                }
                PaymentDirection::Received => PaymentEntry::new_received(
                    Identifier::from(CONTACT),
                    50_000,
                    Some("lunch".into()),
                ),
            };
            entry.status = status;
            // The replay accessor: seeding through the live writer would run
            // its own persist round, which is the very thing under test.
            info.identity_manager
                .managed_identity_mut(&owner())
                .expect("managed identity")
                .dashpay_payments_mut()
                .insert(txid.clone(), entry);
        }
        (wallet_manager, wallet_id, txid)
    }

    pub(super) async fn stored_status(
        wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: &WalletId,
        txid: &str,
    ) -> PaymentStatus {
        let wm = wallet_manager.read().await;
        wm.get_wallet_info(wallet_id)
            .expect("wallet info")
            .identity_manager
            .managed_identity(&owner())
            .expect("managed identity")
            .dashpay()
            .payments
            .get(txid)
            .expect("entry under the sent txid")
            .status
    }

    pub(super) fn sweep_of(wallet_id: WalletId, txid: Txid) -> WalletEvent {
        WalletEvent::TransactionsSwept {
            wallet_id,
            txids: vec![txid],
            superseded_by: Txid::from_byte_array([0x77; 32]),
            winner_mined_height: Some(1_499_050),
            released_outpoints: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        }
    }

    /// A `BlockProcessed` re-emitting the spend as chain-locked — the
    /// evidence that reinstates a transaction a sweep removed.
    pub(super) fn chainlocked_reinstatement(wallet_id: WalletId) -> WalletEvent {
        let record = TransactionRecord::new(
            sent_transaction(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1_499_060,
                BlockHash::all_zeros(),
                0,
            )),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -50_000,
        );
        WalletEvent::BlockProcessed {
            wallet_id,
            height: 1_499_060,
            chain_lock: None,
            inserted: Vec::new(),
            updated: vec![record],
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    /// One row, so a test can assert the overlay is exactly the verdict and
    /// not a replay of the identity's whole payment history.
    ///
    /// Also asserts the authoritative carrier agrees: the round's identity
    /// snapshot must hold the same post-flip entry, because that snapshot —
    /// not the overlay table — is what `load()` rebuilds the payment map
    /// from.
    fn only_row(verdicts: &SentPaymentVerdicts, txid: &str) -> PaymentEntry {
        let overlay = &verdicts.overlay;
        assert_eq!(overlay.len(), 1, "exactly one identity: {overlay:?}");
        let rows = overlay.get(&owner()).expect("the owning identity");
        assert_eq!(rows.len(), 1, "exactly one row: {rows:?}");
        let row = rows.get(txid).expect("the flipped row").clone();
        let snapshot = verdicts
            .identities
            .identities
            .get(&owner())
            .expect("the round must carry the owning identity's snapshot");
        assert_eq!(
            snapshot.dashpay_payments.get(txid),
            Some(&row),
            "the identity snapshot must carry the verdict, not the status it replaced"
        );
        row
    }

    /// The defect: a swept sent payment stayed `Pending` forever because
    /// nothing in the wallet ever wrote `Failed`. The sweep is the only
    /// evidence that exists — the wallet has already deleted the loser's
    /// record — so the verdict has to be taken here or not at all.
    #[tokio::test]
    async fn a_sweep_fails_a_pending_sent_payment() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Pending).await;
        let event = sweep_of(wallet_id, sent_transaction().txid());

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert_eq!(
            only_row(&overlay, &txid).status,
            PaymentStatus::Failed,
            "the overlay must carry the Failed row for this drain's store()"
        );
        assert_eq!(
            only_row(&overlay, &txid).memo.as_deref(),
            Some("lunch"),
            "a verdict changes the status and nothing else"
        );
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Failed,
            "and the live entry must agree with what the round will store"
        );
    }

    /// The worse half of the defect: an IS-locked payment already displayed
    /// as `Confirmed`, then evicted by a chainlocked winner, is a dead
    /// payment reported as good. `Confirmed -> Failed` is the only edge that
    /// corrects it.
    #[tokio::test]
    async fn a_sweep_fails_an_already_confirmed_sent_payment() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Confirmed).await;
        let event = sweep_of(wallet_id, sent_transaction().txid());

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert_eq!(only_row(&overlay, &txid).status, PaymentStatus::Failed);
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Failed
        );
    }

    /// A sweep is not always the last word: a chainlock can reinstate the
    /// transaction it removed. `Failed -> Confirmed` is what makes that
    /// repair reachable — without it the entry would be stuck on a verdict
    /// the chain has since overruled.
    #[tokio::test]
    async fn a_chainlocked_reinstatement_repairs_a_failed_sent_payment() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Failed).await;

        let overlay =
            sent_payment_verdicts(&wallet_manager, &chainlocked_reinstatement(wallet_id)).await;

        assert_eq!(
            only_row(&overlay, &txid).status,
            PaymentStatus::Confirmed,
            "a chainlocked record must overrule the sweep that failed it"
        );
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Confirmed
        );
    }

    /// An InstantSend lock carries no record, only a txid, and is final for
    /// DashPay display — so the txid alone confirms the entry. This is the
    /// path the payment handler used to own; it now belongs to the adapter,
    /// which reaches it over the lossless channel.
    #[tokio::test]
    async fn an_instant_lock_confirms_a_pending_sent_payment() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Pending).await;
        let event = WalletEvent::TransactionInstantLocked {
            wallet_id,
            txid: sent_transaction().txid(),
            instant_lock: InstantLock::default(),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        };

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert_eq!(only_row(&overlay, &txid).status, PaymentStatus::Confirmed);
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Confirmed
        );
    }

    /// A received entry's status is settled when it is recorded from an
    /// on-chain sighting. A sweep that happens to name its txid says nothing
    /// about it, so it must not be touched.
    #[tokio::test]
    async fn a_sweep_never_touches_a_received_payment() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Received, PaymentStatus::Confirmed).await;
        let event = sweep_of(wallet_id, sent_transaction().txid());

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert!(overlay.is_empty(), "received entries carry no verdict");
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Confirmed
        );
    }

    /// Idempotence, and the reason it matters: a re-emitted sweep (a relaunch
    /// re-deriving from a frozen watermark) must produce no row at all, or
    /// every re-detection would put an unchanged row on a store round.
    #[tokio::test]
    async fn a_verdict_already_reached_emits_no_row() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Failed).await;
        let event = sweep_of(wallet_id, sent_transaction().txid());

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert!(overlay.is_empty(), "no change, no row");
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Failed
        );
    }

    /// A mempool sighting is not finality: the payment genuinely is still
    /// pending, so a `TransactionDetected` at an unconfirmed context must
    /// leave it alone.
    #[tokio::test]
    async fn a_mempool_sighting_is_not_finality() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Pending).await;
        let record = TransactionRecord::new(
            sent_transaction(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -50_000,
        );
        let event = WalletEvent::TransactionDetected {
            wallet_id,
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };

        let overlay = sent_payment_verdicts(&wallet_manager, &event).await;

        assert!(overlay.is_empty());
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Pending
        );
    }

    /// The transition table, enumerated. Written out rather than derived so
    /// that adding an edge means editing this list — the point of the table
    /// is that every edge was chosen, not inferred.
    #[test]
    fn the_transition_table_admits_exactly_the_four_intended_edges() {
        use SentPaymentEvidence::{Final, Swept};
        let expected = [
            ((PaymentStatus::Pending, Swept), Some(PaymentStatus::Failed)),
            (
                (PaymentStatus::Confirmed, Swept),
                Some(PaymentStatus::Failed),
            ),
            ((PaymentStatus::Failed, Swept), None),
            (
                (PaymentStatus::Pending, Final),
                Some(PaymentStatus::Confirmed),
            ),
            (
                (PaymentStatus::Failed, Final),
                Some(PaymentStatus::Confirmed),
            ),
            ((PaymentStatus::Confirmed, Final), None),
        ];
        for ((from, evidence), to) in expected {
            assert_eq!(
                next_sent_payment_status(from, evidence),
                to,
                "{from:?} + {evidence:?}"
            );
        }
        assert_eq!(
            expected.iter().filter(|(_, to)| to.is_some()).count(),
            4,
            "four edges move an entry; the other two are no-ops"
        );
    }

    /// `matured` is coinbase maturity — never a DashPay payment — so a
    /// confirmed record arriving only in that bucket is not evidence about a
    /// sent one.
    #[test]
    fn the_matured_bucket_is_not_finality_evidence() {
        let record = TransactionRecord::new(
            sent_transaction(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1_499_060,
                BlockHash::all_zeros(),
                0,
            )),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -50_000,
        );
        let event = WalletEvent::BlockProcessed {
            wallet_id: [0x01; 32],
            height: 1_499_060,
            chain_lock: None,
            inserted: Vec::new(),
            updated: Vec::new(),
            matured: vec![record],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        assert!(sent_payment_evidence(&event).is_empty());
    }
}

#[cfg(test)]
mod contact_watch_only_projection_tests {
    //! Regression coverage for the persist-time projection of records
    //! belonging to a contact's watch-only `DashpayExternalAccount`
    //! (dashpay/rust-dashcore#926's policy at the persistence seam —
    //! see [`is_contact_watch_only`]).
    //!
    //! Upstream emits one record per matched account, so these tests
    //! build the record pair a real payment-to-a-contact produces and
    //! assert on what [`build_core_changeset`] hands the persister:
    //! the funding account's outgoing/negative row, and no wallet TXO
    //! for the coin that landed on the contact's chain.

    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::{Address as DashAddress, BlockHash, Txid};
    use dashcore::{OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Witness};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        InputDetail, OutputDetail, TransactionDirection,
    };
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    use key_wallet::transaction_checking::BlockInfo;
    use key_wallet::{Network, WalletCoreBalance};
    use key_wallet_manager::WalletManager;

    const WALLET_ID: WalletId = [9u8; 32];

    /// Duffs, matching the testnet capture that surfaced this bug:
    /// 1.0 DASH funded, 0.69998912 paid to the contact, 0.3 back as
    /// change, 1088 duffs of fee. The wallet's true net is therefore
    /// `-70_000_000`; the store held `+69_998_912`.
    const FUNDING: u64 = 100_000_000;
    const PAID_TO_CONTACT: u64 = 69_998_912;
    const CHANGE: u64 = 30_000_000;

    fn our_change_address() -> DashAddress {
        DashAddress::dummy(Network::Testnet, 1)
    }

    fn our_receive_address() -> DashAddress {
        DashAddress::dummy(Network::Testnet, 2)
    }

    /// An address on the contact's watch-only chain — derived from the
    /// *contact's* xpub in production, so we can see it but never sign.
    fn contact_address() -> DashAddress {
        DashAddress::dummy(Network::Testnet, 3)
    }

    fn bip44_account_0() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    fn contact_external_account() -> AccountType {
        AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id: [1u8; 32],
            friend_identity_id: [2u8; 32],
        }
    }

    fn in_block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::from_slice(&[4u8; 32]).expect("valid block hash"),
            1_234_567_890,
        ))
    }

    fn funding_outpoint() -> OutPoint {
        OutPoint {
            txid: Txid::from_slice(&[5u8; 32]).expect("valid txid"),
            vout: 0,
        }
    }

    /// One input (our funding coin) and the given outputs, in order.
    fn tx_with(outputs: &[(&DashAddress, u64)]) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: funding_outpoint(),
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: Witness::new(),
            }],
            output: outputs
                .iter()
                .map(|(addr, value)| TxOut {
                    value: *value,
                    script_pubkey: addr.script_pubkey(),
                })
                .collect(),
            special_transaction_payload: None,
        }
    }

    /// The input detail upstream builds when the spent outpoint was one
    /// of the account's own UTXOs.
    fn our_input() -> InputDetail {
        InputDetail {
            index: 0,
            value: FUNDING,
            address: our_change_address(),
        }
    }

    fn output(index: u32, role: OutputRole, address: &DashAddress, value: u64) -> OutputDetail {
        OutputDetail {
            index,
            role,
            address: Some(address.clone()),
            value,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        tx: &Transaction,
        account_type: AccountType,
        direction: TransactionDirection,
        input_details: Vec<InputDetail>,
        output_details: Vec<OutputDetail>,
        net_amount: i64,
    ) -> TransactionRecord {
        record_with(
            tx,
            account_type,
            in_block(1_000),
            TransactionType::Standard,
            direction,
            input_details,
            output_details,
            net_amount,
        )
    }

    /// [`record`] with caller-chosen context and transaction type, for
    /// the lifecycle-coalescing and direction-preservation tests.
    #[allow(clippy::too_many_arguments)]
    fn record_with(
        tx: &Transaction,
        account_type: AccountType,
        context: TransactionContext,
        transaction_type: TransactionType,
        direction: TransactionDirection,
        input_details: Vec<InputDetail>,
        output_details: Vec<OutputDetail>,
        net_amount: i64,
    ) -> TransactionRecord {
        TransactionRecord::new(
            tx.clone(),
            account_type,
            context,
            transaction_type,
            direction,
            input_details,
            output_details,
            net_amount,
        )
    }

    /// The record pair a payment to a contact really produces: the
    /// funding account sees `Outgoing` with a negative net, and the
    /// contact's watch-only chain independently sees `Incoming` with a
    /// positive net for the very same txid.
    ///
    /// Output 0 pays the contact, output 1 is our change.
    fn contact_payment_records() -> (Transaction, TransactionRecord, TransactionRecord) {
        let tx = tx_with(&[
            (&contact_address(), PAID_TO_CONTACT),
            (&our_change_address(), CHANGE),
        ]);
        let funding = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Outgoing,
            vec![our_input()],
            vec![
                // Not ours as far as the funding account is concerned.
                output(0, OutputRole::Sent, &contact_address(), PAID_TO_CONTACT),
                output(1, OutputRole::Change, &our_change_address(), CHANGE),
            ],
            CHANGE as i64 - FUNDING as i64,
        );
        let watch_only = record(
            &tx,
            contact_external_account(),
            TransactionDirection::Incoming,
            // No inputs: this account owns none of the coins spent.
            Vec::new(),
            vec![output(
                0,
                OutputRole::Received,
                &contact_address(),
                PAID_TO_CONTACT,
            )],
            PAID_TO_CONTACT as i64,
        );
        (tx, funding, watch_only)
    }

    fn test_manager() -> Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            dashcore::Network::Testnet,
        )))
    }

    fn block_processed(inserted: Vec<TransactionRecord>) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id: WALLET_ID,
            height: 1_000,
            chain_lock: None,
            inserted,
            updated: vec![],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        }
    }

    fn transaction_detected(record: TransactionRecord) -> WalletEvent {
        WalletEvent::TransactionDetected {
            wallet_id: WALLET_ID,
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        }
    }

    /// A multi-account spend's per-account records
    /// must fold into ONE wallet-level row. Models the S22 field sweep in
    /// miniature: the BIP44 slice spends 2.0, the receival slice spends
    /// 0.62 with 0.005 change — the persisted row must carry the summed
    /// −2.615 net, the union of the details, and Outgoing.
    #[tokio::test]
    async fn multi_account_spend_folds_to_one_wallet_level_record() {
        // Destination the sweep pays; both spending accounts see it as
        // `Sent` (upstream marks every non-own output `Sent` whenever
        // the account owns an input).
        let dest = DashAddress::dummy(Network::Testnet, 7);
        let tx = tx_with(&[(&dest, 261_493_912), (&our_change_address(), 500_000)]);
        let bip44_slice = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 200_000_000,
                address: our_change_address(),
            }],
            vec![
                output(0, OutputRole::Sent, &dest, 261_493_912),
                // The sibling account's change is not this account's
                // address either — account-locally it reads `Sent`.
                output(1, OutputRole::Sent, &our_change_address(), 500_000),
            ],
            -200_000_000,
        );
        let receival_slice = record(
            &tx,
            AccountType::DashpayReceivingFunds {
                index: 0,
                user_identity_id: [1u8; 32],
                friend_identity_id: [2u8; 32],
            },
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 1,
                value: 62_000_000,
                address: our_change_address(),
            }],
            vec![
                output(0, OutputRole::Sent, &dest, 261_493_912),
                output(1, OutputRole::Change, &our_change_address(), 500_000),
            ],
            -61_500_000,
        );
        let cs = build_core_changeset(
            &test_manager(),
            &block_processed(vec![bip44_slice, receival_slice]),
        )
        .await;

        assert_eq!(
            cs.records.len(),
            1,
            "same-txid per-account records must fold into one wallet-level record"
        );
        let persisted = &cs.records[0];
        assert_eq!(persisted.net_amount, -261_500_000);
        assert_eq!(persisted.direction, TransactionDirection::Outgoing);
        assert_eq!(persisted.input_details.len(), 2, "input details must union");
        assert_eq!(persisted.output_details.len(), 2);
        assert_eq!(
            cs.account_records.len(),
            2,
            "the raw per-account slices ride along for account-scoped persisters"
        );
    }

    /// Records for DISTINCT transactions are never folded.
    #[tokio::test]
    async fn distinct_txids_stay_separate_records() {
        let tx_a = tx_with(&[(&our_change_address(), 1_000)]);
        let rec_a = record(
            &tx_a,
            bip44_account_0(),
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 1_000,
                address: our_change_address(),
            }],
            Vec::new(),
            -1_000,
        );
        let mut tx_b = tx_with(&[(&our_change_address(), 2_000)]);
        tx_b.lock_time = 999; // distinct txid
        let rec_b = record(
            &tx_b,
            bip44_account_0(),
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: 2_000,
                address: our_change_address(),
            }],
            Vec::new(),
            -2_000,
        );
        let cs = build_core_changeset(&test_manager(), &block_processed(vec![rec_a, rec_b])).await;
        assert_eq!(cs.records.len(), 2);
    }

    /// (1) A payment to a contact must persist as the outgoing,
    /// negative row — not the contact chain's incoming, positive one.
    ///
    /// Before the fix both records reached `CoreChangeSet.records`;
    /// because the persisted `transactions` row is keyed by txid alone,
    /// the watch-only record (emitted last, since `all_accounts` visits
    /// DashPay accounts after standard ones) defined the stored row and
    /// a 0.69998912 payment *away* was persisted as `+69_998_912`.
    #[tokio::test]
    async fn contact_directed_payment_persists_as_outgoing_and_negative() {
        let (tx, funding, watch_only) = contact_payment_records();
        // Upstream ordering: standard account first, DashPay last.
        let cs = build_core_changeset(&test_manager(), &block_processed(vec![funding, watch_only]))
            .await;

        assert_eq!(
            cs.records.len(),
            1,
            "exactly one record may define the txid-keyed transaction row"
        );
        let persisted = &cs.records[0];
        assert_eq!(persisted.txid, tx.txid());
        assert_eq!(
            persisted.direction,
            TransactionDirection::Outgoing,
            "a payment to a contact is money leaving this wallet"
        );
        assert_eq!(
            persisted.net_amount,
            CHANGE as i64 - FUNDING as i64,
            "net must be -(spent - change), not the contact's credit"
        );
        assert!(
            persisted.net_amount < 0,
            "net_amount must be negative, was {}",
            persisted.net_amount
        );

        // The coin that landed on the contact's chain is not ours, so it
        // must never enter the wallet's TXO set — this is the phantom
        // `isSpent = 0` row that only the contact's own spend could ever
        // have cleared.
        let utxo_outpoints: Vec<u32> = cs.new_utxos.iter().map(|u| u.outpoint.vout).collect();
        assert_eq!(
            utxo_outpoints,
            vec![1],
            "only our change output may become a wallet UTXO"
        );
        assert_eq!(cs.new_utxos[0].txout.value, CHANGE);
    }

    /// Same assertion for the first-sighting path, which delivers each
    /// account's record as its own `TransactionDetected` event — there
    /// is no sibling record in the batch to fall back on, so the filter
    /// has to hold standalone.
    #[tokio::test]
    async fn contact_watch_only_detection_persists_no_transaction_row() {
        let (_, _, watch_only) = contact_payment_records();
        let cs = build_core_changeset(&test_manager(), &transaction_detected(watch_only)).await;

        assert!(
            cs.records.is_empty(),
            "a contact's watch-only chain must not define a wallet transaction row"
        );
        assert!(
            cs.new_utxos.is_empty(),
            "the contact's output must not become a wallet UTXO"
        );
    }

    /// The funding account's own `TransactionDetected` still persists
    /// normally — the filter is scoped to the watch-only account, not
    /// to the transaction.
    #[tokio::test]
    async fn funding_account_detection_of_the_same_payment_still_persists() {
        let (tx, funding, _) = contact_payment_records();
        let cs = build_core_changeset(&test_manager(), &transaction_detected(funding)).await;

        assert_eq!(cs.records.len(), 1);
        assert_eq!(cs.records[0].txid, tx.txid());
        assert_eq!(cs.records[0].direction, TransactionDirection::Outgoing);
        assert!(cs.records[0].net_amount < 0);
    }

    /// (2) A genuine receive is untouched: incoming, positive, and its
    /// output still becomes a wallet UTXO.
    #[tokio::test]
    async fn genuine_receive_still_persists_incoming_and_positive() {
        let tx = tx_with(&[(&our_receive_address(), PAID_TO_CONTACT)]);
        let received = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Incoming,
            // Someone else's coins funded it.
            Vec::new(),
            vec![output(
                0,
                OutputRole::Received,
                &our_receive_address(),
                PAID_TO_CONTACT,
            )],
            PAID_TO_CONTACT as i64,
        );
        let cs = build_core_changeset(&test_manager(), &block_processed(vec![received])).await;

        assert_eq!(cs.records.len(), 1);
        assert_eq!(cs.records[0].direction, TransactionDirection::Incoming);
        assert_eq!(cs.records[0].net_amount, PAID_TO_CONTACT as i64);
        assert!(cs.records[0].net_amount > 0);
        assert_eq!(cs.new_utxos.len(), 1, "a real receive still creates a TXO");
        assert_eq!(cs.new_utxos[0].txout.value, PAID_TO_CONTACT);
    }

    /// (2b) A receive on a DashPay **receival** account — addresses
    /// derived from *our* xpub, which a contact pays into — is money
    /// genuinely arriving and must keep its incoming/positive row.
    /// This is the boundary #926 drew and this change must not blur.
    #[tokio::test]
    async fn dashpay_receival_account_receive_is_unaffected() {
        let tx = tx_with(&[(&our_receive_address(), PAID_TO_CONTACT)]);
        let received = record(
            &tx,
            AccountType::DashpayReceivingFunds {
                index: 0,
                user_identity_id: [1u8; 32],
                friend_identity_id: [2u8; 32],
            },
            TransactionDirection::Incoming,
            Vec::new(),
            vec![output(
                0,
                OutputRole::Received,
                &our_receive_address(),
                PAID_TO_CONTACT,
            )],
            PAID_TO_CONTACT as i64,
        );
        let cs = build_core_changeset(&test_manager(), &block_processed(vec![received])).await;

        assert_eq!(
            cs.records.len(),
            1,
            "receival accounts derive from OUR xpub — those funds are ours"
        );
        assert_eq!(cs.records[0].direction, TransactionDirection::Incoming);
        assert_eq!(cs.records[0].net_amount, PAID_TO_CONTACT as i64);
        assert_eq!(cs.new_utxos.len(), 1);
    }

    /// (3) An internal transfer — every output owned, none on a
    /// contact's chain — is unaffected in direction, net and TXOs.
    #[tokio::test]
    async fn internal_transfer_is_unaffected() {
        let tx = tx_with(&[
            (&our_receive_address(), PAID_TO_CONTACT),
            (&our_change_address(), CHANGE),
        ]);
        let internal = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Internal,
            vec![our_input()],
            vec![
                output(
                    0,
                    OutputRole::Received,
                    &our_receive_address(),
                    PAID_TO_CONTACT,
                ),
                output(1, OutputRole::Change, &our_change_address(), CHANGE),
            ],
            (PAID_TO_CONTACT + CHANGE) as i64 - FUNDING as i64,
        );
        let cs = build_core_changeset(&test_manager(), &block_processed(vec![internal])).await;

        assert_eq!(cs.records.len(), 1);
        assert_eq!(cs.records[0].direction, TransactionDirection::Internal);
        assert_eq!(
            cs.records[0].net_amount,
            (PAID_TO_CONTACT + CHANGE) as i64 - FUNDING as i64
        );
        assert_eq!(
            cs.new_utxos.len(),
            2,
            "both owned outputs stay in the wallet's TXO set"
        );
        assert_eq!(cs.spent_utxos.len(), 1, "the spent input is still removed");
    }

    /// Confirmation re-emits the same records under `updated`. The
    /// filter has to hold there too, or the watch-only row would
    /// re-clobber the correct one the moment the block landed.
    #[tokio::test]
    async fn confirmation_re_emit_does_not_reintroduce_the_watch_only_row() {
        let (_, funding, watch_only) = contact_payment_records();
        let event = WalletEvent::BlockProcessed {
            wallet_id: WALLET_ID,
            height: 1_001,
            chain_lock: None,
            inserted: vec![],
            updated: vec![funding, watch_only],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };
        let cs = build_core_changeset(&test_manager(), &event).await;

        assert_eq!(cs.records.len(), 1);
        assert_eq!(cs.records[0].direction, TransactionDirection::Outgoing);
    }

    /// A cross-account spend (CoinJoin-funded send with BIP44 change)
    /// emits one record per matched account, and the two slices DISAGREE
    /// on the change output's role: the funding account's slice carries it
    /// as `Sent` (its account-local view cannot attribute the sibling
    /// account's address), the change account's slice as `Change`. The
    /// fold seeds its output union from the FUNDING record, so keeping the
    /// base entry on index collision let `Sent` win — and every UTXO
    /// projection over the folded record (record_new_utxos_ffi,
    /// derive_new_utxos) then dropped the wallet's own change while the
    /// folded net stayed correct. 2026-08-19 device run: corrected record
    /// rows landed, TXOs never arrived, the reconcile tripwire healed 4.
    /// On collision the owned role must win.
    #[tokio::test]
    async fn fold_prefers_owned_output_role_on_index_collision() {
        const CHANGE_BACK: u64 = FUNDING - PAID_TO_CONTACT - 227;
        let tx = tx_with(&[
            (&contact_address(), PAID_TO_CONTACT),
            (&our_change_address(), CHANGE_BACK),
        ]);
        // Funding account's slice: knows the input; sees BOTH outputs as
        // counterparty payments.
        let funding_slice = record(
            &tx,
            AccountType::CoinJoin { index: 0 },
            TransactionDirection::Outgoing,
            vec![our_input()],
            vec![
                output(0, OutputRole::Sent, &contact_address(), PAID_TO_CONTACT),
                output(1, OutputRole::Sent, &our_change_address(), CHANGE_BACK),
            ],
            -(FUNDING as i64),
        );
        // Change account's slice: no inputs of its own; owns output 1.
        let change_slice = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Incoming,
            vec![],
            vec![output(
                1,
                OutputRole::Change,
                &our_change_address(),
                CHANGE_BACK,
            )],
            CHANGE_BACK as i64,
        );

        let event = WalletEvent::BlockProcessed {
            wallet_id: WALLET_ID,
            height: 1_001,
            chain_lock: None,
            inserted: vec![funding_slice, change_slice],
            updated: vec![],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };
        let cs = build_core_changeset(&test_manager(), &event).await;

        assert_eq!(cs.records.len(), 1, "same-txid slices fold to one row");
        let folded = &cs.records[0];
        assert_eq!(
            folded.net_amount,
            CHANGE_BACK as i64 - FUNDING as i64,
            "net is the sum of the slices"
        );
        let change_detail = folded
            .output_details
            .iter()
            .find(|o| o.index == 1)
            .expect("folded record keeps output 1");
        assert_eq!(
            change_detail.role,
            OutputRole::Change,
            "the owned role must win the index collision — a lingering Sent role \
             makes every UTXO projection drop the wallet's own change"
        );
    }

    /// A detection snapshot and its confirmation snapshot for the SAME
    /// transaction are one account contribution observed twice, not two
    /// account slices. Merging their changesets must coalesce to the
    /// newest snapshot — summing them doubled the persisted net
    /// (−100 detected + −100 confirmed = −200) and folding could keep
    /// the stale `Mempool` context over the confirmed one.
    #[tokio::test]
    async fn detection_then_confirmation_coalesces_to_the_confirmed_snapshot() {
        let tx = tx_with(&[(&contact_address(), PAID_TO_CONTACT)]);
        let mempool_slice = record_with(
            &tx,
            bip44_account_0(),
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            vec![our_input()],
            vec![output(
                0,
                OutputRole::Sent,
                &contact_address(),
                PAID_TO_CONTACT,
            )],
            -(FUNDING as i64),
        );
        let mut confirmed_slice = mempool_slice.clone();
        confirmed_slice.context = in_block(1_001);

        let manager = test_manager();
        let mut merged = build_core_changeset(&manager, &transaction_detected(mempool_slice)).await;
        let confirmation = WalletEvent::BlockProcessed {
            wallet_id: WALLET_ID,
            height: 1_001,
            chain_lock: None,
            inserted: vec![],
            updated: vec![confirmed_slice],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };
        merged.merge(build_core_changeset(&manager, &confirmation).await);

        assert_eq!(
            merged.records.len(),
            1,
            "two observations of one transaction must coalesce to one row"
        );
        let row = &merged.records[0];
        assert_eq!(
            row.net_amount,
            -(FUNDING as i64),
            "coalescing must not sum repeated snapshots"
        );
        assert!(
            matches!(row.context, TransactionContext::InBlock(_)),
            "the newest (confirmed) snapshot must win, got {:?}",
            row.context
        );
    }

    /// Wallet-level direction cannot be derived from the net's sign: a
    /// cross-account transfer nets −fee but is, wallet-level, a
    /// self-transfer. After the owned-role collision fix flips the
    /// funding slice's `Sent` view of the sibling-owned output to the
    /// sibling's `Received`, no `Sent` output remains — the fold must
    /// label the row `Internal`, exactly as upstream labels a
    /// single-account self-transfer.
    #[tokio::test]
    async fn cross_account_transfer_folds_to_internal_direction() {
        const MOVED: u64 = FUNDING - 1_000; // everything minus fee
        let tx = tx_with(&[(&our_receive_address(), MOVED)]);
        let funding_slice = record(
            &tx,
            AccountType::CoinJoin { index: 0 },
            TransactionDirection::Outgoing,
            vec![our_input()],
            // The sibling account's address is not this account's —
            // account-locally the output reads `Sent`.
            vec![output(0, OutputRole::Sent, &our_receive_address(), MOVED)],
            -(FUNDING as i64),
        );
        let receiving_slice = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Incoming,
            vec![],
            vec![output(
                0,
                OutputRole::Received,
                &our_receive_address(),
                MOVED,
            )],
            MOVED as i64,
        );
        let cs = build_core_changeset(
            &test_manager(),
            &block_processed(vec![funding_slice, receiving_slice]),
        )
        .await;

        assert_eq!(cs.records.len(), 1);
        let row = &cs.records[0];
        assert_eq!(row.net_amount, -1_000, "wallet net is just the fee");
        assert_eq!(
            row.direction,
            TransactionDirection::Internal,
            "a cross-account move is a wallet-level self-transfer, \
             not an Outgoing spend"
        );
    }

    /// `CoinJoin` is assigned from the transaction TYPE upstream, never
    /// from amounts — a multi-account CoinJoin round with a nonzero
    /// wallet net must keep the `CoinJoin` direction through the fold.
    #[tokio::test]
    async fn coinjoin_fold_keeps_coinjoin_direction() {
        let tx = tx_with(&[(&our_receive_address(), FUNDING - 500)]);
        let slice_a = record_with(
            &tx,
            AccountType::CoinJoin { index: 0 },
            in_block(1_000),
            TransactionType::CoinJoin,
            TransactionDirection::CoinJoin,
            vec![our_input()],
            vec![output(
                0,
                OutputRole::Received,
                &our_receive_address(),
                FUNDING - 500,
            )],
            -500,
        );
        let slice_b = record_with(
            &tx,
            bip44_account_0(),
            in_block(1_000),
            TransactionType::CoinJoin,
            TransactionDirection::CoinJoin,
            vec![],
            vec![],
            0,
        );
        let cs =
            build_core_changeset(&test_manager(), &block_processed(vec![slice_a, slice_b])).await;

        assert_eq!(cs.records.len(), 1);
        assert_eq!(
            cs.records[0].direction,
            TransactionDirection::CoinJoin,
            "a nonzero net must not rewrite a CoinJoin row as Outgoing/Incoming"
        );
    }

    /// A fold lands at the group's FIRST position even when the funding
    /// record (the metadata source) appears later — unrelated records
    /// between the slices must not move ahead of the folded transaction.
    #[tokio::test]
    async fn fold_lands_at_the_groups_first_position() {
        let tx = tx_with(&[(&our_change_address(), CHANGE)]);
        let no_input_slice = record(
            &tx,
            bip44_account_0(),
            TransactionDirection::Incoming,
            vec![],
            vec![output(0, OutputRole::Change, &our_change_address(), CHANGE)],
            CHANGE as i64,
        );
        let mut unrelated_tx = tx_with(&[(&our_receive_address(), 1_000)]);
        unrelated_tx.lock_time = 77; // distinct txid
        let unrelated = record(
            &unrelated_tx,
            bip44_account_0(),
            TransactionDirection::Incoming,
            vec![],
            vec![output(
                0,
                OutputRole::Received,
                &our_receive_address(),
                1_000,
            )],
            1_000,
        );
        let funding_slice = record(
            &tx,
            AccountType::CoinJoin { index: 0 },
            TransactionDirection::Outgoing,
            vec![our_input()],
            vec![output(0, OutputRole::Sent, &our_change_address(), CHANGE)],
            -(FUNDING as i64),
        );

        let mut records = vec![no_input_slice, unrelated, funding_slice];
        crate::changeset::changeset::fold_same_txid_records(&mut records);

        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].txid,
            tx.txid(),
            "the folded record must keep the group's first position"
        );
        assert_eq!(
            records[0].account_type,
            AccountType::CoinJoin { index: 0 },
            "funding metadata still comes from the funding slice"
        );
        assert_eq!(records[1].txid, unrelated_tx.txid());
    }

    /// The adapter drain is NOT where a multi-account transaction's
    /// mempool slices reliably meet: live matching emits one
    /// `TransactionDetected` per account, and a drain can commit
    /// between them. The projection must therefore rebuild the
    /// wallet-level row from the MANAGER's slices — an event carrying
    /// one account's slice still yields the full fold, so the
    /// persisted row converges no matter how events land in drains.
    #[tokio::test]
    async fn mempool_slice_event_rebuilds_the_full_fold_from_the_manager() {
        use crate::wallet::core::WalletGeneration;
        use crate::wallet::identity::IdentityManager;
        use key_wallet::test_utils::TestWalletContext;

        // A wallet funded on TWO standard accounts (mirrors
        // `test_support::funded_wallet_manager_dual_standard`, kept
        // inline because the spend must be checked before the managed
        // wallet moves into the manager).
        let mut ctx = TestWalletContext::new_random();
        let bip44_address = ctx.receive_address.clone();
        let bip32_address = {
            let xpub = ctx
                .wallet
                .accounts
                .standard_bip32_accounts
                .get(&0)
                .expect("bip32 account")
                .account_xpub;
            ctx.managed_wallet
                .first_bip32_managed_account_mut()
                .expect("bip32 managed account")
                .next_receive_address(Some(&xpub), true)
                .expect("bip32 receive address")
        };
        let bip44_funding = Transaction::dummy(&bip44_address, 0..1, &[100_000_000]);
        let bip32_funding = Transaction::dummy(&bip32_address, 1..2, &[50_000_000]);
        for funding in [&bip44_funding, &bip32_funding] {
            let result = ctx
                .check_transaction(
                    funding,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        1,
                        BlockHash::from_slice(&[4u8; 32]).expect("valid block hash"),
                        1_700_000_000,
                    )),
                )
                .await;
            assert!(result.is_relevant, "funding tx should be relevant");
        }

        // One transaction spending BOTH accounts' coins — the manager
        // records one slice per account for it.
        let spend = Transaction {
            version: 2,
            lock_time: 0,
            input: [&bip44_funding, &bip32_funding]
                .iter()
                .map(|funding| TxIn {
                    previous_output: OutPoint {
                        txid: funding.txid(),
                        vout: 0,
                    },
                    script_sig: ScriptBuf::new(),
                    sequence: 0xffffffff,
                    witness: Witness::new(),
                })
                .collect(),
            output: vec![TxOut {
                value: 149_999_000,
                script_pubkey: DashAddress::dummy(Network::Testnet, 7).script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let result = ctx
            .check_transaction(&spend, TransactionContext::Mempool)
            .await;
        assert!(result.is_relevant, "spend should match both accounts");

        let info = PlatformWalletInfo {
            core_wallet: ctx.managed_wallet,
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
            observed_input_conflicts: Default::default(),
        };
        let mut wm = WalletManager::<PlatformWalletInfo>::new(dashcore::Network::Testnet);
        let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
        let manager = Arc::new(RwLock::new(wm));

        let (slices, _) = wallet_slices_and_verdicts_for_txid(&manager, &wallet_id, &spend.txid())
            .await
            .expect("manager knows the wallet");
        assert_eq!(slices.len(), 2, "both funding accounts hold a slice");
        let lone_slice = slices
            .iter()
            .find(|r| r.account_type == bip44_account_0())
            .expect("bip44 slice")
            .clone();
        assert_eq!(
            lone_slice.net_amount, -100_000_000,
            "the lone slice carries only its own account's net"
        );

        // The event delivers ONE slice — as live mempool matching does.
        let lone_event = WalletEvent::TransactionDetected {
            wallet_id,
            record: Box::new(lone_slice),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };
        let cs = build_core_changeset(&manager, &lone_event).await;

        assert_eq!(cs.records.len(), 1, "one wallet-level row");
        assert_eq!(
            cs.records[0].net_amount, -150_000_000,
            "the row is rebuilt from ALL of the manager's slices, \
             not the one slice the event happened to carry"
        );
        assert_eq!(
            cs.account_records.len(),
            2,
            "both account slices ride along for account-scoped persisters"
        );
    }

    /// A contact spending an output that a *pre-fix* build already
    /// persisted must still clear that stale row, so `derive_spent_utxos`
    /// stays deliberately unfiltered. Only the transaction row and the
    /// new-TXO projection are suppressed.
    #[tokio::test]
    async fn contact_spend_still_clears_a_stale_pre_fix_txo() {
        let tx = tx_with(&[(&contact_address(), PAID_TO_CONTACT)]);
        let watch_only_spend = record(
            &tx,
            contact_external_account(),
            TransactionDirection::Outgoing,
            vec![InputDetail {
                index: 0,
                value: PAID_TO_CONTACT,
                address: contact_address(),
            }],
            vec![output(
                0,
                OutputRole::Sent,
                &contact_address(),
                PAID_TO_CONTACT,
            )],
            -(PAID_TO_CONTACT as i64),
        );
        let cs =
            build_core_changeset(&test_manager(), &block_processed(vec![watch_only_spend])).await;

        assert!(
            cs.records.is_empty(),
            "the contact spending their own coin is not a transaction of ours"
        );
        assert_eq!(
            cs.spent_utxos.len(),
            1,
            "the stale pre-fix TXO must still be removed"
        );
        assert_eq!(cs.spent_utxos[0].outpoint, funding_outpoint());
    }
}

#[cfg(test)]
mod usage_delta_tests {
    //! Regression coverage for [`collect_usage_deltas_from_accounts`] —
    //! the matching/resolution seam that rebuilds `mark_address_used`
    //! results the `WalletEvent` bus doesn't carry. Drives a real
    //! `ManagedWalletInfo` through `check_core_transaction` (the same
    //! mutating entry point SPV block processing uses) and then runs
    //! the bridge's derivation over the post-mutation state, exactly
    //! as the event adapter does at runtime.

    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::transaction_checking::{BlockInfo, WalletTransactionChecker};

    fn bip44_account_0() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    fn in_block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::from_slice(&[1u8; 32]).expect("valid block hash"),
            1_234_567_890,
        ))
    }

    /// A P2PKH script the wallet does not monitor, for counterparty
    /// outputs. Built from the secp256k1 generator point.
    fn foreign_script() -> ScriptBuf {
        const TEST_PUBKEY_G: [u8; 33] = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
            0x5b, 0x16, 0xf8, 0x17, 0x98,
        ];
        let pubkey =
            dashcore::PublicKey::from_slice(&TEST_PUBKEY_G).expect("generator point is valid");
        dashcore::Address::p2pkh(&pubkey, key_wallet::Network::Testnet).script_pubkey()
    }

    fn spend_to(previous_output: OutPoint, script_pubkey: ScriptBuf, value: u64) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output,
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value,
                script_pubkey,
            }],
            special_transaction_payload: None,
        }
    }

    /// An output paying a monitored BIP44 receive address must surface
    /// as a marked-used entry (External pool, index 0, `used == true`)
    /// and advance the account's external highest-used watermark to 0.
    #[tokio::test]
    async fn receive_output_marks_address_used_and_advances_highest_used() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();

        let funding_outpoint = OutPoint {
            txid: Txid::from_slice(&[2u8; 32]).expect("valid txid"),
            vout: 0,
        };
        let tx = spend_to(funding_outpoint, receive_address.script_pubkey(), 75_000);
        let result = managed_wallet
            .check_core_transaction(&tx, in_block(100_000), &mut wallet, true, true)
            .await;
        assert!(result.is_relevant, "fixture tx must match the wallet");

        let records: Vec<&TransactionRecord> = result.new_records.iter().collect();
        let (marked, highest) =
            collect_usage_deltas_from_accounts(&managed_wallet.accounts, &records);

        let entry = marked
            .iter()
            .find(|d| d.info.address == receive_address)
            .expect("receive address must be in the marked-used delta");
        assert_eq!(entry.account_type, bip44_account_0());
        assert_eq!(entry.pool_type, AddressPoolType::External);
        assert_eq!(entry.info.index, 0);
        assert!(matches!(entry.info.state, AddressState::Used));

        let watermarks = highest
            .get(&bip44_account_0())
            .expect("touched account must carry a highest-used snapshot");
        assert_eq!(watermarks.external, Some(0));
    }

    /// Spending a wallet-owned UTXO removes it from the live account
    /// BEFORE the wallet event fires, so the replayed
    /// `check_transaction` can no longer see the input match. The
    /// spent address must still be captured — via the record's
    /// `input_details`, which key-wallet populates pre-removal.
    #[tokio::test]
    async fn spent_input_address_is_captured_after_utxo_removal() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();

        // Fund the wallet at the receive address...
        let funding_outpoint = OutPoint {
            txid: Txid::from_slice(&[2u8; 32]).expect("valid txid"),
            vout: 0,
        };
        let fund_tx = spend_to(funding_outpoint, receive_address.script_pubkey(), 75_000);
        let fund_result = managed_wallet
            .check_core_transaction(&fund_tx, in_block(100_000), &mut wallet, true, true)
            .await;
        assert!(fund_result.is_relevant);

        // ...then spend that UTXO entirely to a foreign address. After
        // this call the funded outpoint is gone from the account's
        // live UTXO set.
        let spend_tx = spend_to(
            OutPoint {
                txid: fund_tx.txid(),
                vout: 0,
            },
            foreign_script(),
            74_000,
        );
        let spend_result = managed_wallet
            .check_core_transaction(&spend_tx, in_block(100_001), &mut wallet, true, true)
            .await;
        assert!(spend_result.is_relevant, "spend of our UTXO must match");

        // Derive usage deltas from the SPEND records only — the
        // funding record is deliberately excluded, so the only path to
        // the spent address is the record's own `input_details`.
        let records: Vec<&TransactionRecord> = spend_result
            .new_records
            .iter()
            .chain(spend_result.updated_records.iter())
            .collect();
        assert!(!records.is_empty(), "spend must produce a record");
        let (marked, highest) =
            collect_usage_deltas_from_accounts(&managed_wallet.accounts, &records);

        let entry = marked
            .iter()
            .find(|d| d.info.address == receive_address)
            .expect("spent-input address must be in the marked-used delta");
        assert_eq!(entry.pool_type, AddressPoolType::External);
        assert!(matches!(entry.info.state, AddressState::Used));
        // The foreign output must NOT resolve to any pool.
        assert!(
            marked.iter().all(|d| d.info.address != {
                dashcore::Address::from_script(&foreign_script(), key_wallet::Network::Testnet)
                    .expect("foreign script is a valid P2PKH")
            }),
            "counterparty addresses must drop out at pool resolution"
        );
        assert_eq!(
            highest
                .get(&bip44_account_0())
                .expect("account touched via input details")
                .external,
            Some(0)
        );
    }

    /// `derive_spent_utxos`' script reconstruction must hold on the real
    /// funding-then-spend path, not just on a hand-built `InputDetail`:
    /// fund a receive address via `check_core_transaction`, spend that
    /// UTXO, and check the derived spent UTXO's script against the
    /// *original funding output's* script — the one `InputDetail.address`
    /// was independently derived from via `Address::from_script`.
    #[tokio::test]
    async fn spent_utxo_script_matches_the_real_funding_output() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();
        let funding_script = receive_address.script_pubkey();

        let funding_outpoint = OutPoint {
            txid: Txid::from_slice(&[2u8; 32]).expect("valid txid"),
            vout: 0,
        };
        let fund_tx = spend_to(funding_outpoint, funding_script.clone(), 75_000);
        let fund_result = managed_wallet
            .check_core_transaction(&fund_tx, in_block(100_000), &mut wallet, true, true)
            .await;
        assert!(fund_result.is_relevant);

        let spent_outpoint = OutPoint {
            txid: fund_tx.txid(),
            vout: 0,
        };
        let spend_tx = spend_to(spent_outpoint, foreign_script(), 74_000);
        let spend_result = managed_wallet
            .check_core_transaction(&spend_tx, in_block(100_001), &mut wallet, true, true)
            .await;
        assert!(spend_result.is_relevant, "spend of our UTXO must match");

        let record = spend_result
            .new_records
            .iter()
            .chain(spend_result.updated_records.iter())
            .find(|r| !r.input_details.is_empty())
            .expect("spend must produce a record carrying the spent input's details");

        let spent = derive_spent_utxos(record);
        let spent_utxo = spent
            .iter()
            .find(|u| u.outpoint == spent_outpoint)
            .expect("derived spent UTXOs must include the funding outpoint");
        assert_eq!(
            spent_utxo.txout.script_pubkey, funding_script,
            "the derived script must match the real funding output's script, \
             not just round-trip through the address"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::freeze_synced_height_if_faulted;
    use crate::changeset::changeset::CoreChangeSet;

    /// A spent UTXO must carry the real locking script of the output it
    /// spends, reconstructed from the address the input detail already
    /// carries. `TxOut::script_pubkey` has no encoding for "unknown", so a
    /// default-filled script is a claim about the chain that was never
    /// observed, and any consumer reading stored scripts sees an unusable row.
    ///
    /// The reconstruction is exact: `InputDetail.address` is cloned from the
    /// wallet's own `Utxo.address`, which key-wallet derived from that
    /// output's script via `Address::from_script`, and `is_p2pkh`/`is_p2sh`
    /// accept only the canonical form — so `script_pubkey()` rebuilds the same
    /// bytes. This test is what keeps that true, since `Utxo::new` does not
    /// enforce the address/script pairing.
    #[test]
    fn spent_utxos_carry_the_real_script_of_the_address_they_spend() {
        use dashcore::hashes::Hash;
        use dashcore::{OutPoint, Transaction, TxIn, Txid};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::{
            InputDetail, TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{TransactionContext, TransactionType};

        let addresses = [
            dashcore::Address::new(
                dashcore::Network::Testnet,
                dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array(
                    [0x11u8; 20],
                )),
            ),
            dashcore::Address::new(
                dashcore::Network::Testnet,
                dashcore::address::Payload::ScriptHash(dashcore::ScriptHash::from_byte_array(
                    [0x22u8; 20],
                )),
            ),
        ];
        let transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: addresses
                .iter()
                .enumerate()
                .map(|(index, _)| TxIn {
                    previous_output: OutPoint {
                        txid: Txid::from_byte_array([index as u8 + 1; 32]),
                        vout: index as u32,
                    },
                    ..Default::default()
                })
                .collect(),
            output: vec![],
            special_transaction_payload: None,
        };
        let record = TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            addresses
                .iter()
                .enumerate()
                .map(|(index, address)| InputDetail {
                    index: index as u32,
                    value: 1_000 * (index as u64 + 1),
                    address: address.clone(),
                })
                .collect(),
            Vec::new(),
            -2_000,
        );

        let spent = super::derive_spent_utxos(&record);
        assert_eq!(spent.len(), addresses.len());
        for (utxo, address) in spent.iter().zip(addresses.iter()) {
            assert!(
                !utxo.txout.script_pubkey.is_empty(),
                "a spent UTXO must never carry a fabricated empty script"
            );
            assert_eq!(
                utxo.txout.script_pubkey,
                address.script_pubkey(),
                "the script must be the address's own locking script"
            );
            assert_eq!(
                dashcore::Address::from_script(
                    &utxo.txout.script_pubkey,
                    dashcore::Network::Testnet
                )
                .expect("the emitted script must decode as an address"),
                *address,
                "the script must round-trip back to the input's own address"
            );
        }
    }

    /// While persistence is healthy the sync
    /// watermark flows through untouched.
    #[test]
    fn healthy_persistence_keeps_synced_height() {
        let mut core = CoreChangeSet {
            synced_height: Some(100),
            last_processed_height: Some(300),
            ..CoreChangeSet::default()
        };
        freeze_synced_height_if_faulted(&mut core, false);
        assert_eq!(core.synced_height, Some(100));
        assert_eq!(core.last_processed_height, Some(300));
    }

    /// Once persistence has faulted, the durable
    /// watermark is frozen (`synced_height` stripped) so it can't outrun
    /// the rows — but ONLY `synced_height` is dropped; every other field
    /// (here `last_processed_height`, standing in for records/UTXO
    /// deltas) still persists.
    #[test]
    fn faulted_persistence_freezes_only_synced_height() {
        let mut core = CoreChangeSet {
            synced_height: Some(200),
            last_processed_height: Some(300),
            ..CoreChangeSet::default()
        };
        freeze_synced_height_if_faulted(&mut core, true);
        assert_eq!(
            core.synced_height, None,
            "watermark must be frozen after a persistence fault"
        );
        assert_eq!(
            core.last_processed_height,
            Some(300),
            "non-watermark fields must still persist while faulted"
        );
    }

    /// Per-wallet fault scoping: a `store()`
    /// rejection freezes ONLY the named wallet; a sibling keeps advancing.
    /// (There is no global `Lagged` latch — the lossless
    /// unbounded persistence channel can never lag.)
    #[test]
    fn fault_state_scopes_store_rejection_per_wallet() {
        let a = [0xAAu8; 32];
        let b = [0xBBu8; 32];
        let signal = std::sync::atomic::AtomicBool::new(false);

        let mut fault = AdapterFaultState::default();
        assert!(!fault.is_faulted(&a) && !fault.is_faulted(&b));

        // store() rejection for A: A frozen, B untouched.
        fault.fault_wallet(a, &signal);
        assert!(fault.is_faulted(&a), "rejected wallet must freeze");
        assert!(!fault.is_faulted(&b), "sibling wallet must keep advancing");
        assert!(
            signal.load(std::sync::atomic::Ordering::Relaxed),
            "hard-fault signal must be raised on a store rejection"
        );
    }

    // ── Adapter-loop integration tests ──
    //
    // These drive `run_wallet_event_adapter` with a real lossless
    // `mpsc::UnboundedSender` and a probe persister so the LOOP — not just
    // the `freeze_synced_height_if_faulted` helper — is exercised: a large
    // lossless burst, a rejected `store()`, the per-wallet freeze, and
    // per-wallet batch folding.

    use super::{
        run_wallet_event_adapter, AdapterFaultState, PaymentOverlay, ADAPTER_STORE_BATCH_LIMIT,
    };
    use crate::changeset::changeset::PlatformWalletChangeSet;
    use crate::changeset::client_start_state::ClientStartState;
    use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence};
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use dpp::prelude::Identifier;
    use key_wallet::WalletCoreBalance;
    use key_wallet_manager::{WalletEvent, WalletManager};
    use std::collections::{BTreeMap, HashSet};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
    use tokio::sync::RwLock;
    use tokio_util::sync::CancellationToken;

    /// One observed `store()` call, projected to just the fields the
    /// watermark-freeze invariant cares about.
    #[derive(Debug)]
    struct StoreObserved {
        wallet_id: WalletId,
        synced_height: Option<u32>,
        last_processed_height: Option<u32>,
        n_records: usize,
        n_asset_locks: usize,
        n_asset_locks_removed: usize,
        /// Swept transactions the round carries — the core change a verdict
        /// must be coupled to, since the sweep never re-emits once its round
        /// is durable.
        n_sweeps: usize,
        /// The round's `dashpay_payments_overlay` verbatim — `None` when the
        /// round carried none, which is also what a payments-blind persister
        /// must see after the capability gate has withheld one.
        dashpay_payments: Option<PaymentOverlay>,
        /// The payment maps carried by the round's identity snapshots. This
        /// is the carrier `load()` actually rebuilds `dashpay_payments` from,
        /// so a verdict that reaches only `dashpay_payments` above does not
        /// survive a restart. `None` when the round carried no `identities`
        /// sub-changeset at all.
        dashpay_identity_payments: Option<PaymentOverlay>,
        rejected: bool,
    }

    /// Test persister: records every `store()` (over an unbounded tokio
    /// channel so the async test can await progress without blocking the
    /// current-thread runtime) and can be told to reject the NEXT store for
    /// a given wallet exactly once.
    struct ProbePersister {
        obs: UnboundedSender<StoreObserved>,
        fail_once: Mutex<HashSet<WalletId>>,
        /// Wallets whose NEXT `store()` panics instead of returning. Models a
        /// backend that dies mid-write — which surfaces as a `JoinError`
        /// instead of unwinding the whole adapter task.
        panic_once: Mutex<HashSet<WalletId>>,
        /// Held closed to keep a `store()` call parked. The SQLite backend
        /// commits a real transaction per call, so a slow disk parks the caller
        /// for real; this makes that duration controllable.
        block_until: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        /// Raised as soon as a blocked `store()` is entered, so a test can wait
        /// for the block to be in effect rather than sleeping and hoping.
        blocked: Arc<AtomicBool>,
        capabilities: crate::changeset::PersistenceCapabilities,
    }

    impl ProbePersister {
        fn new(obs: UnboundedSender<StoreObserved>) -> Self {
            Self {
                obs,
                fail_once: Mutex::new(HashSet::new()),
                panic_once: Mutex::new(HashSet::new()),
                block_until: Mutex::new(None),
                blocked: Arc::new(AtomicBool::new(false)),
                capabilities: crate::changeset::PersistenceCapabilities::NONE,
            }
        }
        /// A probe that additionally attests `capabilities` — used by the
        /// `CORE_SWEEP_REMOVAL` gate tests, which need a persister on record
        /// as (not) supporting the sweep contract.
        fn with_capabilities(
            obs: UnboundedSender<StoreObserved>,
            capabilities: crate::changeset::PersistenceCapabilities,
        ) -> Self {
            Self {
                capabilities,
                ..Self::new(obs)
            }
        }
        /// Park the next `store()` until the returned sender is dropped or
        /// signalled. `blocked` reports when the park is actually in effect.
        fn block_next(&self) -> (std::sync::mpsc::Sender<()>, Arc<AtomicBool>) {
            let (tx, rx) = std::sync::mpsc::channel();
            *self.block_until.lock().unwrap() = Some(rx);
            (tx, Arc::clone(&self.blocked))
        }
        fn fail_next(&self, wallet_id: WalletId) {
            self.fail_once.lock().unwrap().insert(wallet_id);
        }
        fn panic_next(&self, wallet_id: WalletId) {
            self.panic_once.lock().unwrap().insert(wallet_id);
        }
    }

    impl PlatformWalletPersistence for ProbePersister {
        fn persistence_capabilities(&self) -> crate::changeset::PersistenceCapabilities {
            self.capabilities
        }

        fn store(
            &self,
            wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let core = changeset.core.as_ref();
            if let Some(gate) = self.block_until.lock().unwrap().take() {
                self.blocked.store(true, Ordering::Relaxed);
                // Blocks the calling thread outright — the whole point is to
                // model a synchronous backend, so an async wait would prove
                // nothing.
                let _ = gate.recv();
                self.blocked.store(false, Ordering::Relaxed);
            }
            if self.panic_once.lock().unwrap().remove(&wallet_id) {
                panic!("probe persister: store panicked for {wallet_id:?}");
            }
            let rejected = self.fail_once.lock().unwrap().remove(&wallet_id);
            let _ = self.obs.send(StoreObserved {
                wallet_id,
                synced_height: core.and_then(|c| c.synced_height),
                last_processed_height: core.and_then(|c| c.last_processed_height),
                n_records: core.map(|c| c.records.len()).unwrap_or(0),
                n_asset_locks: changeset
                    .asset_locks
                    .as_ref()
                    .map(|a| a.asset_locks.len())
                    .unwrap_or(0),
                n_asset_locks_removed: changeset
                    .asset_locks
                    .as_ref()
                    .map(|a| a.removed.len())
                    .unwrap_or(0),
                n_sweeps: core.map(|c| c.sweeps.len()).unwrap_or(0),
                dashpay_payments: changeset.dashpay_payments_overlay.clone(),
                dashpay_identity_payments: changeset.identities.as_ref().map(|ids| {
                    ids.identities
                        .iter()
                        .map(|(id, entry)| (*id, entry.dashpay_payments.clone()))
                        .collect()
                }),
                rejected,
            });
            if rejected {
                Err(PersistenceError::backend("probe: forced store rejection"))
            } else {
                Ok(())
            }
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn test_manager() -> Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            dashcore::Network::Testnet,
        )))
    }

    /// A bare watermark event — the ONLY event whose height reaches the
    /// persisted `syncedHeight`, and therefore the one the guard strips.
    fn sync_height_event(wallet_id: WalletId, height: u32) -> WalletEvent {
        WalletEvent::SyncHeightAdvanced { wallet_id, height }
    }

    /// A record-bearing (non-watermark) event: carries
    /// `last_processed_height` but no `synced_height`, so it survives the
    /// freeze untouched. Used both as a sentinel to prove loop progress and
    /// to prove records still persist after the guard activates.
    fn block_processed_event(wallet_id: WalletId, height: u32) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id,
            height,
            chain_lock: None,
            inserted: vec![],
            updated: vec![],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        }
    }

    /// (b) THE ROOT-CAUSE FIX: a burst far larger than the old bounded ring
    /// (`DEFAULT_WALLET_EVENT_CAPACITY` == 1000) is delivered losslessly over
    /// the unbounded persistence channel, so the fault latch never trips and
    /// the durable watermark advances all the way to the tip of the
    /// catch-up. On a bounded broadcast this burst would `Lagged` and freeze
    /// the watermark forever.
    #[tokio::test]
    async fn lossless_burst_never_freezes_and_watermark_reaches_tip() {
        const BURST: u32 = 3000; // >> the old broadcast ring (1000)
        let wallet_id = [7u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        // Pre-buffer the whole burst; an unbounded mpsc cannot drop it.
        for h in 1..=BURST {
            tx.send(sync_height_event(wallet_id, h)).unwrap();
        }

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Close the channel so the adapter drains the entire burst (across as
        // many batches as `ADAPTER_STORE_BATCH_LIMIT` requires) and exits.
        drop(tx);
        handle.await.unwrap();

        assert!(
            !sync_fault.load(Ordering::Relaxed),
            "a lossless burst must never raise the freeze signal"
        );

        let mut max_synced = 0u32;
        let mut any_store = false;
        while let Ok(observed) = obs_rx.try_recv() {
            any_store = true;
            assert_eq!(observed.wallet_id, wallet_id);
            assert!(
                !observed.rejected,
                "no store should be rejected on the happy path"
            );
            if let Some(h) = observed.synced_height {
                max_synced = max_synced.max(h);
            }
        }
        assert!(any_store, "the burst must have produced at least one store");
        assert_eq!(
            max_synced, BURST,
            "the durable watermark must advance to the tip across the whole catch-up"
        );
    }

    /// A cancelled adapter commits the backlog already in the channel before
    /// exiting, instead of racing the token against `recv` and discarding
    /// whatever the producer had already handed to the lossless channel.
    ///
    /// The persister outlives the drain here, which is the `shutdown()` shape:
    /// a joined shutdown holds the manager — and therefore the persister —
    /// alive for as long as the drain it triggered. A dirty `Drop` gives no
    /// such guarantee; see the `Drop` rustdoc on `PlatformWalletManager`.
    #[tokio::test]
    async fn cancellation_commits_the_events_already_buffered() {
        let wallet_id = [11u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        tx.send(sync_height_event(wallet_id, 41)).unwrap();
        tx.send(sync_height_event(wallet_id, 42)).unwrap();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let cancel = CancellationToken::new();
        // Already cancelled when the loop starts: the shape a cancelled
        // manager leaves behind.
        cancel.cancel();

        run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::new(AtomicBool::new(false)),
            cancel,
        )
        .await;

        let observed = obs_rx
            .try_recv()
            .expect("a cancelled adapter must still commit the buffered backlog");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            observed.synced_height,
            Some(42),
            "both buffered events belong to the same drain"
        );
        assert!(
            obs_rx.try_recv().is_err(),
            "the drain stops at the backlog it found, and never waits for more"
        );
        // Held to the end so the exit is the cancel path, not a closed channel.
        drop(tx);
    }

    /// A drain owns the persister until its whole backlog is committed, so a
    /// chunk boundary is not a loss boundary.
    ///
    /// A backlog larger than [`ADAPTER_STORE_BATCH_LIMIT`] is committed in
    /// several chunks. Claiming the persister only after a chunk has folded
    /// its events makes the first chunk's commit release the last strong
    /// reference, and the next chunk then finds nothing to commit to — after
    /// it has already taken its events off the lossless channel. The owner
    /// releasing its `Arc` mid-drain (what `Drop` does) is exactly the
    /// interleaving that exposes it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancelled_drain_holds_the_persister_until_its_backlog_is_committed() {
        use std::time::{Duration, Instant};

        let wallet_id = [0x55u8; 32];
        // One past the limit: the tail event cannot ride the first chunk.
        let backlog = ADAPTER_STORE_BATCH_LIMIT as u32 + 1;
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        for height in 1..=backlog {
            tx.send(sync_height_event(wallet_id, height)).unwrap();
        }

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let (release, blocked) = persister.block_next();
        let probe = Arc::downgrade(&persister);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::new(AtomicBool::new(false)),
            cancel,
        ));

        // Park inside the first chunk's `store()`, then release the only
        // strong reference outside the adapter — the manager's own drop,
        // landing while the drain is under way.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !blocked.load(Ordering::Relaxed) {
            assert!(
                Instant::now() < deadline,
                "the first chunk's store must park before the drop below means anything"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        drop(persister);
        drop(release);

        let first = obs_rx
            .recv()
            .await
            .expect("the first chunk of the backlog must commit");
        assert_eq!(
            first.synced_height,
            Some(ADAPTER_STORE_BATCH_LIMIT as u32),
            "the first chunk folds up to the batch limit"
        );
        let second = obs_rx.recv().await.expect(
            "the chunk after the first must still commit: a drain owns the \
             persister until its backlog is on disk",
        );
        assert_eq!(
            second.synced_height,
            Some(backlog),
            "the tail of the backlog must reach the store, not the warn log"
        );

        handle.await.unwrap();
        assert!(
            probe.upgrade().is_none(),
            "a finished drain must release the persister it claimed"
        );
        // Held to the end so the exit is the cancel path, not a closed channel.
        drop(tx);
    }

    /// (c) A rejected `store()` faults the wallet, and the very next
    /// watermark-only event is stripped and dropped (not delivered).
    #[tokio::test]
    async fn rejected_store_freezes_wallet_and_strips_watermark() {
        let wallet_id = [9u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        persister.fail_next(wallet_id); // first store() for this wallet is rejected
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // 1) Record-bearing event whose store() is rejected → faults wallet.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        let first = obs_rx
            .recv()
            .await
            .expect("rejected store still calls store()");
        assert!(first.rejected, "first store must be the forced rejection");
        assert_eq!(first.last_processed_height, Some(10));

        // 2) Watermark-only event → stripped, never delivered.
        tx.send(sync_height_event(wallet_id, 200)).unwrap();
        // 3) Sentinel proving the loop moved past the watermark.
        tx.send(block_processed_event(wallet_id, 20)).unwrap();

        let sentinel = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("the sentinel store must arrive rather than hanging the suite")
            .expect("sentinel store must arrive");
        assert_eq!(
            sentinel.last_processed_height,
            Some(20),
            "the sentinel (not the stripped watermark) is the next delivered store"
        );
        assert_eq!(sentinel.synced_height, None);
        assert!(
            obs_rx.try_recv().is_err(),
            "the watermark-only changeset must not have been delivered after the fault"
        );
        assert!(sync_fault.load(Ordering::Relaxed));

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (d) Record-bearing changesets keep persisting after the guard is
    /// active — only the watermark is held back.
    #[tokio::test]
    async fn record_bearing_changesets_persist_after_guard_activates() {
        let wallet_id = [4u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        persister.fail_next(wallet_id); // activate the guard via a rejection
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Trip the guard.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        let _ = obs_rx.recv().await.expect("first (rejected) store");

        // A later record-bearing event must STILL be persisted.
        tx.send(block_processed_event(wallet_id, 30)).unwrap();
        let later = obs_rx
            .recv()
            .await
            .expect("post-guard record-bearing store must arrive");
        assert_eq!(
            later.last_processed_height,
            Some(30),
            "record-bearing changesets must still persist while the watermark is frozen"
        );
        assert_eq!(later.synced_height, None, "watermark stays frozen");
        assert!(!later.rejected, "second store must succeed");

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (e) No startup race: an event emitted BEFORE the adapter task is
    /// spawned (i.e. before it polls) is still delivered, because an
    /// `mpsc::UnboundedReceiver` buffers messages sent before the first
    /// `recv()` rather than dropping them (unlike a `broadcast::Receiver`,
    /// which only sees messages sent after it subscribes). This is the exact
    /// invariant the manager relies on.
    #[tokio::test]
    async fn events_emitted_before_task_poll_are_received() {
        let wallet_id = [3u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        // Emit BEFORE the task is spawned / polls.
        tx.send(block_processed_event(wallet_id, 77)).unwrap();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        let observed = obs_rx
            .recv()
            .await
            .expect("a pre-spawn event must still be delivered");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(observed.last_processed_height, Some(77));
        assert_eq!(observed.n_records, 0);
        assert!(
            !sync_fault.load(Ordering::Relaxed),
            "no fault expected on the happy path"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (e) Events already buffered in the ring are folded into a SINGLE
    /// `store()` per wallet, with the watermark taking the monotonic max.
    ///
    /// This is the throughput property that keeps the ring from
    /// overflowing in the first place: the adapter's cost is one
    /// (JNI + Room) store per *batch*, not per event. Every event is
    /// published before the task is spawned, so the first `recv()` sees
    /// event 1 and the `try_recv()` drain folds in 2..=5 without ever
    /// awaiting — making the batch boundary deterministic.
    #[tokio::test]
    async fn buffered_events_fold_into_one_store_per_wallet() {
        let wallet_id = [11u8; 32];
        // Comfortably larger than the burst, so nothing is dropped.
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        for h in 1..=5u32 {
            tx.send(sync_height_event(wallet_id, h)).unwrap();
        }

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        let observed = obs_rx.recv().await.expect("merged store must arrive");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            observed.synced_height,
            Some(5),
            "folded watermark must be the monotonic max of the batch"
        );
        assert!(
            !sync_fault.load(Ordering::Relaxed),
            "a clean batch must not raise the fault signal"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();

        // Exactly one store for the whole burst — five events, one
        // round-trip. Drained after the task has exited so no further
        // store can still be in flight.
        assert!(
            obs_rx.try_recv().is_err(),
            "the buffered burst must collapse into a single store"
        );
    }

    /// (f) Folding is scoped per wallet: a batch carrying events for two
    /// wallets produces one store each, correctly attributed. Merging
    /// across wallets would mis-persist one wallet's rows under the
    /// other's id.
    #[tokio::test]
    async fn batch_folds_per_wallet_not_across_wallets() {
        let wallet_a = [1u8; 32];
        let wallet_b = [2u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        // Interleaved on purpose.
        tx.send(sync_height_event(wallet_a, 10)).unwrap();
        tx.send(sync_height_event(wallet_b, 20)).unwrap();
        tx.send(sync_height_event(wallet_a, 11)).unwrap();
        tx.send(sync_height_event(wallet_b, 21)).unwrap();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        let mut heights: BTreeMap<WalletId, Option<u32>> = BTreeMap::new();
        for _ in 0..2 {
            let observed = obs_rx.recv().await.expect("both wallets must store");
            heights.insert(observed.wallet_id, observed.synced_height);
        }

        assert_eq!(heights.get(&wallet_a), Some(&Some(11)));
        assert_eq!(heights.get(&wallet_b), Some(&Some(21)));

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
        assert!(
            obs_rx.try_recv().is_err(),
            "one store per wallet, not per event"
        );
    }

    /// (h) SAFETY INVARIANT under a commit-thread PANIC: a wallet whose
    /// `store()` panicked must be frozen just as if the store had been
    /// rejected, because its rows have an unknown fate.
    ///
    /// This is a regression guard on the move to `spawn_blocking`. Before it,
    /// a panic unwound the adapter task itself, which stopped every later
    /// watermark advance by killing the writer outright. `spawn_blocking`
    /// turns that into a recoverable `JoinError` — and merely logging it would
    /// let the NEXT batch persist a higher `synced_height` for a wallet whose
    /// earlier rows may never have landed, which is exactly the hole the
    /// durable-watermark guard exists to close.
    #[tokio::test]
    async fn a_panicking_commit_freezes_the_batch_wallets() {
        let wallet_id = [0xEEu8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        persister.panic_next(wallet_id);
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // The store for this batch panics: no observation is emitted, and the
        // adapter must fault the wallet rather than carry on unaffected.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        // Bounded, so a regression fails the test instead of hanging it: with
        // the fault-on-panic path removed, `sync_fault` is simply never raised
        // and an unbounded spin would wedge CI with no diagnosis.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !sync_fault.load(Ordering::Relaxed) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("a panicked commit must raise the hard-fault signal");

        // The adapter must still be alive — the point of moving the commit off
        // the runtime is that one bad batch does not take the writer with it.
        assert!(
            !handle.is_finished(),
            "a panicked commit must not kill the adapter"
        );

        // A later watermark for the same wallet must not reach the store.
        tx.send(block_processed_event(wallet_id, 60)).unwrap();
        tx.send(sync_height_event(wallet_id, 900)).unwrap();

        let post = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("a faulted wallet must still persist its rows")
            .expect("the record-bearing event must still persist while faulted");
        assert_eq!(post.wallet_id, wallet_id);
        assert_eq!(
            post.synced_height, None,
            "a wallet whose commit panicked must not advance its durable watermark"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (j) THE PRIMARY BEHAVIOUR OF THIS PR: a blocked `store()` must not park
    /// the async runtime.
    ///
    /// `store()` is synchronous and, for the SQLite backend, commits a real
    /// transaction per call. Called inline on a tokio worker it held that
    /// worker for the duration — a field restore showed one drain park the
    /// runtime long enough for the metrics tick covering it to report a 1.4s
    /// mean poll, with the durable watermark left hundreds of thousands of
    /// blocks behind the chain tip.
    ///
    /// Every other test in this module would still pass with `commit_batch`
    /// moved back inline, because they only check persistence outcomes. This
    /// one runs the adapter on a SINGLE worker, parks a `store()`, and requires
    /// a spawned task to still get scheduled.
    ///
    /// Deliberately built out of `std` primitives — a `std::mpsc` handoff and
    /// `std::thread::sleep` on the test thread — rather than `tokio::time`.
    /// The regression parks the runtime's only worker, and a tokio timer needs
    /// that runtime to fire: an async timeout here hangs instead of failing,
    /// which is worse than the bug it is meant to catch.
    #[test]
    fn a_blocked_store_does_not_park_the_runtime() {
        use std::sync::mpsc as std_mpsc;
        use std::time::{Duration, Instant};

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let wallet_id = [0x33u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let (release, blocked) = persister.block_next();
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();

        let handle = runtime.spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Park the commit inside a synchronous `store()`.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !blocked.load(Ordering::Relaxed) {
            assert!(
                Instant::now() < deadline,
                "the store must actually park before the assertion below means anything"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        // The discriminator: a SPAWNED task has to be scheduled on the
        // runtime's single worker. With the commit on `spawn_blocking` the
        // worker is free and this arrives at once; with it inline the worker is
        // sitting inside `store()` and this times out.
        let (sentinel_tx, sentinel_rx) = std_mpsc::channel();
        runtime.spawn(async move {
            let _ = sentinel_tx.send(());
        });
        sentinel_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("a blocked store must not hold the runtime's only worker");

        // Release, then let the drain finish so the adapter shuts down cleanly.
        drop(release);
        runtime.block_on(async {
            let observed = tokio::time::timeout(Duration::from_secs(5), obs_rx.recv())
                .await
                .expect("the released store must complete")
                .expect("store observed");
            assert_eq!(observed.wallet_id, wallet_id);

            cancel.cancel();
            drop(tx);
            handle.await.unwrap();
        });
    }

    /// The adapter upgrades its weak persister reference for exactly the span
    /// of a batch commit — the sole bound on the manager's synchronous release,
    /// since a drop racing a commit reclaims the persister only when the parked
    /// `store()` returns (issue #4133).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_in_flight_commit_holds_a_strong_persister_reference() {
        use std::time::{Duration, Instant};

        let wallet_id = [0x44u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let (release, blocked) = persister.block_next();
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        assert_eq!(
            Arc::strong_count(&persister),
            1,
            "an idle adapter must hold the persister weakly — only this test's \
             own reference may be strong"
        );

        // Park the commit inside `store()`, and wait until the park is in
        // effect so the count below is read during the commit, not before it.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !blocked.load(Ordering::Relaxed) {
            assert!(
                Instant::now() < deadline,
                "the store must actually park before the assertion below means anything"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            Arc::strong_count(&persister),
            2,
            "a commit in flight must hold the upgraded reference for the whole \
             of its store()"
        );

        drop(release);
        obs_rx
            .recv()
            .await
            .expect("the released store must complete");
        cancel.cancel();
        drop(tx);
        handle.await.unwrap();

        assert_eq!(
            Arc::strong_count(&persister),
            1,
            "the upgraded reference must be released with the finished commit"
        );
    }

    /// (i) A commit panic must punish exactly the wallets whose outcome it
    /// left unknown — no more, no less.
    ///
    /// `commit_batch` walks the batch serially (a `BTreeMap`, so in wallet-id
    /// order), and a panic cuts it in two. A wallet whose `store()` already
    /// returned is settled: its rows are on disk and its watermark is safe,
    /// so freezing it would strip its `synced_height` for the rest of the
    /// session over a sibling's bad batch. A wallet ordered AFTER the panic
    /// is the opposite case — unwinding dropped its consumed changes before
    /// `store()` was ever attempted, so nothing knows whether its rows
    /// landed, and it must freeze or a later watermark advances past rows
    /// that never existed.
    ///
    /// Both halves are checked here, because they are guarded by the same
    /// `batch_wallet_ids - settled` expression and a regression that faults
    /// only the wallet that actually panicked satisfies neither.
    ///
    /// Guards the fix for the first version of the panic handler, which
    /// faulted every wallet in the drain.
    #[tokio::test]
    async fn a_panicking_commit_spares_the_wallets_it_already_stored() {
        // `BTreeMap` order decides who is committed first, so the ids are
        // chosen to put the healthy wallet ahead of the panicking one.
        let healthy = [0x11u8; 32];
        let doomed = [0x22u8; 32];
        // Sorts after `doomed`, so the commit unwinds before it is reached.
        let unreached = [0x33u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        persister.panic_next(doomed);
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // All three in one drain: `healthy` stores, `doomed` panics, and
        // `unreached` never gets its turn. Sent before any is observed so they
        // fold into a single batch.
        tx.send(block_processed_event(healthy, 10)).unwrap();
        tx.send(block_processed_event(doomed, 10)).unwrap();
        tx.send(block_processed_event(unreached, 10)).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !sync_fault.load(Ordering::Relaxed) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the panicked commit must raise the hard-fault signal");

        // Drain what the panicking batch managed to observe.
        while obs_rx.try_recv().is_ok() {}

        // The healthy wallet's watermark must still advance: its store
        // returned, so its rows are accounted for.
        tx.send(sync_height_event(healthy, 900)).unwrap();
        // Bounded: on a regression the healthy wallet is frozen, its
        // watermark-only changeset collapses to nothing, and no store is
        // observed at all — an unbounded `recv` would hang CI instead of
        // reporting which invariant broke.
        let after_healthy = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("a wallet whose store completed must still be storable")
            .expect("healthy wallet still stores");
        assert_eq!(after_healthy.wallet_id, healthy);
        assert_eq!(
            after_healthy.synced_height,
            Some(900),
            "a wallet whose store completed must not be frozen by a sibling's panic"
        );

        // The wallet whose store panicked must be frozen.
        tx.send(block_processed_event(doomed, 60)).unwrap();
        tx.send(sync_height_event(doomed, 900)).unwrap();
        let after_doomed = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("a faulted wallet must still persist its rows")
            .expect("doomed wallet still persists rows");
        assert_eq!(after_doomed.wallet_id, doomed);
        assert_eq!(
            after_doomed.synced_height, None,
            "the wallet whose commit panicked must not advance its watermark"
        );

        // The wallet the commit never reached must be frozen too: its changes
        // went down with the unwind without a `store()` ever being attempted,
        // so its rows are exactly as unaccounted-for as the panicking
        // wallet's. A handler that faults only the direct casualty leaves this
        // one free to advance past rows that never landed.
        tx.send(block_processed_event(unreached, 60)).unwrap();
        tx.send(sync_height_event(unreached, 900)).unwrap();
        let after_unreached =
            tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
                .await
                .expect("a faulted wallet must still persist its rows")
                .expect("unreached wallet still persists rows");
        assert_eq!(after_unreached.wallet_id, unreached);
        assert_eq!(
            after_unreached.synced_height, None,
            "a wallet the panicking commit never reached must not advance its watermark"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (g) SAFETY INVARIANT under a fault: once the per-wallet fault latch is
    /// set (here by a rejected `store()`), no later changeset for that wallet
    /// may advance the durable `synced_height` — whether the watermark arrives
    /// standalone or folded together with a record. The freeze is applied
    /// after the fold, so a `synced_height` that entered via `Merge` is
    /// stripped just like a standalone one; otherwise folding would smuggle
    /// the watermark past the guard (durable watermark outrunning the rows
    /// it implies).
    #[tokio::test]
    async fn watermark_is_still_stripped_after_a_fault() {
        let wallet_id = [9u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        // Fault the wallet via a rejected store — the trigger with a known
        // outcome. The other one, a commit panic, is covered by
        // `a_panicking_commit_freezes_the_batch_wallets`.
        persister.fail_next(wallet_id);
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Trip the fault and wait until it has latched.
        tx.send(block_processed_event(wallet_id, 10)).unwrap();
        let _ = obs_rx.recv().await.expect("first (rejected) store");
        while !sync_fault.load(Ordering::Relaxed) {
            tokio::task::yield_now().await;
        }

        // A record-bearing event and a watermark event, evaluated under the
        // fault. Whether they fold into one changeset or arrive as two, the
        // durable watermark must never be persisted while faulted, and the
        // record must still land. Await the record-bearing store *before*
        // cancelling so the cancel can't race ahead of processing it.
        tx.send(block_processed_event(wallet_id, 60)).unwrap();
        tx.send(sync_height_event(wallet_id, 900)).unwrap();

        let post = obs_rx
            .recv()
            .await
            .expect("the record-bearing event must still persist while faulted");
        assert_eq!(post.wallet_id, wallet_id);
        assert_eq!(
            post.synced_height, None,
            "no store may carry a synced_height once the wallet is faulted"
        );
        assert_eq!(
            post.last_processed_height,
            Some(60),
            "record-bearing fields must survive the freeze"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();

        // Any further store (if the watermark arrived unfolded) must also have
        // been stripped — never a bare advancing watermark.
        while let Ok(observed) = obs_rx.try_recv() {
            assert_eq!(
                observed.synced_height, None,
                "no store may carry a synced_height once the wallet is faulted"
            );
        }
    }

    /// Mined height every block-context sweep event in this module carries.
    const WINNER_HEIGHT: u32 = 700;

    /// A `TransactionsSwept` event for a helper below.
    fn swept_event(wallet_id: WalletId, txid_byte: u8, superseded_by_byte: u8) -> WalletEvent {
        use dashcore::hashes::Hash as _;
        WalletEvent::TransactionsSwept {
            wallet_id,
            txids: vec![dashcore::Txid::from_byte_array([txid_byte; 32])],
            superseded_by: dashcore::Txid::from_byte_array([superseded_by_byte; 32]),
            winner_mined_height: Some(WINNER_HEIGHT),
            released_outpoints: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        }
    }

    /// dashpay/platform#4406 (finding 2): sweeps reach an FFI host only
    /// through the persistence extension's size-negotiated sweep slot, so a
    /// persister predating it processes the rest of the round and returns
    /// success without ever seeing `core.sweeps`. A `store()` that comes
    /// back `Ok` therefore proves nothing about whether a swept loser's
    /// row was actually removed unless the persister has separately
    /// attested `CORE_SWEEP_REMOVAL`. A persister that never declares it
    /// (the probe's default) must be treated exactly like a rejection when
    /// a round carries a sweep — even though, unlike the rejection tests
    /// above, the probe's own `store()` call reports success.
    #[tokio::test]
    async fn sweep_without_declared_capability_freezes_the_wallet_despite_a_successful_store() {
        let wallet_id = [21u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        // No capabilities declared — the pre-`CORE_SWEEP_REMOVAL` shape.
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        tx.send(swept_event(wallet_id, 0x51, 0x52)).unwrap();
        let first = obs_rx
            .recv()
            .await
            .expect("the round is still handed to store()");
        assert!(
            !first.rejected,
            "the probe's own store() must succeed — the gate lives in the \
             adapter, not in a persister that has no idea sweeps exist"
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !sync_fault.load(Ordering::Relaxed) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect(
            "the fail-closed guard must trip for an undeclared sweep even \
             though store() itself reported success",
        );

        // A later watermark-only event must be stripped just like it would
        // be after a real store() rejection.
        tx.send(sync_height_event(wallet_id, 500)).unwrap();
        tx.send(block_processed_event(wallet_id, 40)).unwrap();
        let sentinel = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("the sentinel store must arrive rather than hanging the suite")
            .expect("sentinel store must arrive");
        assert_eq!(sentinel.last_processed_height, Some(40));
        assert_eq!(
            sentinel.synced_height, None,
            "the watermark must stay frozen: a removal must never be \
             reported durable to a backend that never attested it can apply it"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// The coalesced shape of the same gap, which is the one that actually
    /// loses data. The adapter folds whatever is buffered, so a sweep and a
    /// following watermark advance arrive in ONE changeset — and
    /// `synced_height` sits in the unchanged prefix a pre-sweep persister
    /// does read and commit.
    ///
    /// Faulting after `store()` returns cannot retract a watermark the
    /// backend has already made durable: on the next launch the wallet
    /// believes those blocks are scanned, never re-matches them, and the
    /// removal that round carried is lost for good. So the height has to be
    /// stripped before the changeset is handed over, not after.
    #[tokio::test]
    async fn a_coalesced_sweep_and_watermark_never_commits_the_height() {
        let wallet_id = [23u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        // Buffered before the adapter starts, so both events are guaranteed
        // to land in the same drain rather than racing it.
        tx.send(swept_event(wallet_id, 0x61, 0x62)).unwrap();
        tx.send(sync_height_event(wallet_id, 900)).unwrap();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        // No capabilities declared — the pre-`CORE_SWEEP_REMOVAL` shape.
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Bounded like the neighbouring capability tests below: both the
        // adapter and `ProbePersister` hold their own sender, so a
        // regression that stops the folded round from reaching `store()`
        // would otherwise hang this test instead of failing its assertion.
        let observed = tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv())
            .await
            .expect("the folded round reaches store() within the timeout")
            .expect("the folded round reaches store()");
        assert_eq!(
            observed.synced_height, None,
            "an unattested persister must never be handed the watermark of a \
             round whose removal it cannot apply"
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !sync_fault.load(Ordering::Relaxed) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the fail-closed guard must still trip for the folded round");

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// The positive case for the same gate: a persister that attests
    /// `CORE_SWEEP_REMOVAL` is trusted normally, and the watermark keeps
    /// advancing through a sweep-bearing round exactly as it would through
    /// any other.
    #[tokio::test]
    async fn sweep_with_declared_capability_does_not_freeze() {
        let wallet_id = [22u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::with_capabilities(
            obs_tx,
            crate::changeset::PersistenceCapabilities::CORE_SWEEP_REMOVAL,
        ));
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::downgrade(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        tx.send(swept_event(wallet_id, 0x61, 0x62)).unwrap();
        // A watermark-bearing event right behind it, folded or not — either
        // way it must reach the store untouched while the capability holds.
        tx.send(sync_height_event(wallet_id, 700)).unwrap();

        let mut last_synced = None;
        // Drain until a store carries the watermark. Each receive is bounded:
        // the adapter and the probe both hold the sender alive, so a plain
        // `recv()` would never report the channel quiet — a regression that
        // stops the watermark would hang here until the suite's own timeout
        // instead of failing on the assertion below.
        for _ in 0..10 {
            match tokio::time::timeout(std::time::Duration::from_secs(5), obs_rx.recv()).await {
                Ok(Some(observed)) => {
                    assert!(!observed.rejected);
                    if let Some(h) = observed.synced_height {
                        last_synced = Some(h);
                        break;
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }
        assert_eq!(
            last_synced,
            Some(700),
            "the watermark must advance normally once the backend attests \
             CORE_SWEEP_REMOVAL"
        );
        assert!(
            !sync_fault.load(Ordering::Relaxed),
            "an attested backend must never trip the fail-closed guard"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// End-to-end restore-scan shape through the real adapter loop: a
    /// `BlockProcessed` event whose inserted record is an asset-lock tx
    /// filed under a funding account must (a) repopulate the wallet's
    /// in-memory `tracked_asset_locks` and (b) carry the reconstructed
    /// row to the persister in the same `store()` as the core record.
    /// This is the path that rebuilds the host's persisted asset-lock
    /// mirror after a wipe & recover.
    #[tokio::test]
    async fn block_processed_asset_lock_record_reconstructs_and_persists() {
        use dashcore::hashes::Hash as _;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use tokio::sync::Notify;

        use super::spawn_wallet_event_adapter;
        use crate::test_support::{
            funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
        };
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::asset_lock::tracked::AssetLockStatus;
        use crate::wallet::persister::WalletPersister;

        // A wallet whose identity-registration funding account has a
        // real address pool, plus an asset-lock tx whose credit output
        // pays into it (built by the production builder).
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(dashcore::Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let asset_lock_manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn crate::changeset::PlatformWalletPersistence>,
            ),
        );
        let (tx, _path) = asset_lock_manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");

        // The record a restore scan would file under the funding account.
        let record = TransactionRecord::new(
            tx.clone(),
            AccountType::IdentityRegistration,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                4321,
                dashcore::BlockHash::all_zeros(),
                1_650_000_000,
            )),
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            vec![],
            vec![],
            0,
        );

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let (event_tx, event_rx) = unbounded_channel();
        let cancel = CancellationToken::new();
        let sync_fault = Arc::new(AtomicBool::new(false));
        let handle = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::downgrade(&persister),
            event_rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        );

        event_tx
            .send(WalletEvent::BlockProcessed {
                wallet_id,
                height: 4321,
                chain_lock: None,
                inserted: vec![record],
                updated: vec![],
                matured: vec![],
                balance: WalletCoreBalance::default(),
                account_balances: BTreeMap::new(),
                addresses_derived: vec![],
            })
            .expect("send event");

        let observed = obs_rx.recv().await.expect("adapter must store the batch");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(observed.n_records, 1, "the core record must persist");
        assert_eq!(
            observed.n_asset_locks, 1,
            "the reconstructed asset-lock row must ride the same store()"
        );

        let out_point = dashcore::OutPoint::new(tx.txid(), 0);
        {
            let wm = wallet_manager.read().await;
            let lock = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("reconstructed in-memory entry");
            assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
            assert_eq!(lock.amount, 1_000_000);
        }

        cancel.cancel();
        handle.await.expect("adapter task joins");
    }

    /// The `TransactionsSwept` arm end to end: a sweep naming a tracked
    /// lock's funding tx must drop the in-memory entry and carry the
    /// tombstone to the persister through the same `removed` channel a
    /// rejected-at-broadcast `Built` row uses. A swept funding tx can
    /// never confirm, so without this the entry is a zombie
    /// `resume_asset_lock` re-broadcasts and waits on without bound, and
    /// every store mirrors it forever.
    #[tokio::test]
    async fn transactions_swept_removes_the_tracked_asset_lock_it_funded() {
        use dashcore::hashes::Hash as _;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use tokio::sync::Notify;

        use super::spawn_wallet_event_adapter;
        use crate::test_support::{
            funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
        };
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::persister::WalletPersister;

        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(dashcore::Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let asset_lock_manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn crate::changeset::PlatformWalletPersistence>,
            ),
        );
        let (tx, _path) = asset_lock_manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");

        let record = TransactionRecord::new(
            tx.clone(),
            AccountType::IdentityRegistration,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                4321,
                dashcore::BlockHash::all_zeros(),
                1_650_000_000,
            )),
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            vec![],
            vec![],
            0,
        );

        let (obs_tx, mut obs_rx) = unbounded_channel();
        // Attested for sweeps AND payments — a fully capable backend. Only
        // the sweep half matters to this test's assertions; the payments bit
        // keeps the fixture from silently withholding a sent-payment verdict
        // if this event ever carries one.
        let persister = Arc::new(ProbePersister::with_capabilities(
            obs_tx,
            crate::changeset::PersistenceCapabilities::CORE_SWEEP_REMOVAL
                .union(crate::changeset::PersistenceCapabilities::DASHPAY_PAYMENTS)
                .union(crate::changeset::PersistenceCapabilities::ATOMIC_CHANGESETS),
        ));
        let (event_tx, event_rx) = unbounded_channel();
        let cancel = CancellationToken::new();
        let sync_fault = Arc::new(AtomicBool::new(false));
        let handle = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::downgrade(&persister),
            event_rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        );

        // Track the lock the same way a restore scan would.
        event_tx
            .send(WalletEvent::BlockProcessed {
                wallet_id,
                height: 4321,
                chain_lock: None,
                inserted: vec![record],
                updated: vec![],
                matured: vec![],
                balance: WalletCoreBalance::default(),
                account_balances: BTreeMap::new(),
                addresses_derived: vec![],
            })
            .expect("send reconstruction event");
        let observed = obs_rx.recv().await.expect("reconstruction store");
        assert_eq!(observed.n_asset_locks, 1, "sanity: the entry is tracked");

        // The funding tx is swept.
        event_tx
            .send(WalletEvent::TransactionsSwept {
                wallet_id,
                txids: vec![tx.txid()],
                superseded_by: dashcore::Txid::from_byte_array([0x77; 32]),
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![],
                balance: WalletCoreBalance::default(),
                account_balances: BTreeMap::new(),
            })
            .expect("send sweep event");

        let observed = obs_rx.recv().await.expect("sweep store");
        assert_eq!(
            observed.n_asset_locks_removed, 1,
            "the dead lock's tombstone must ride the sweep's own store()"
        );

        let out_point = dashcore::OutPoint::new(tx.txid(), 0);
        {
            let wm = wallet_manager.read().await;
            assert!(
                !wm.get_wallet_info(&wallet_id)
                    .expect("wallet")
                    .tracked_asset_locks
                    .contains_key(&out_point),
                "the in-memory entry must not outlive its swept funding tx"
            );
        }

        cancel.cancel();
        handle.await.expect("adapter task joins");
    }

    /// The coalesced sweep-then-chainlocked-reinstatement fold, driven
    /// through the REAL producers rather than hand-built changesets: the
    /// sweep arm removes the tracked entry and emits its tombstone, the
    /// reinstating chainlocked record re-inserts through reconstruction at
    /// a non-Consumed status, and folding the two — exactly what the
    /// adapter's batched drain does — must cancel the tombstone. Before
    /// `AssetLockChangeSet::merge` learned that, the merged changeset
    /// carried both, and SQLite (upserts before removals) deleted the row
    /// it had just reinstated while the in-memory wallet kept it: the
    /// durable tracked lock vanished across a restart even though its
    /// funding transaction survived.
    #[tokio::test]
    async fn a_reinstating_reconstruction_folded_after_a_sweep_cancels_its_tombstone() {
        use dashcore::hashes::Hash as _;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use tokio::sync::Notify;

        use crate::changeset::merge::Merge as _;
        use crate::test_support::{
            funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
        };
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::asset_lock::sync::reconstruction;
        use crate::wallet::asset_lock::tracked::AssetLockStatus;
        use crate::wallet::persister::WalletPersister;

        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(dashcore::Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let asset_lock_manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn crate::changeset::PlatformWalletPersistence>,
            ),
        );
        let (tx, _path) = asset_lock_manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");
        let record = TransactionRecord::new(
            tx.clone(),
            AccountType::IdentityRegistration,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                4321,
                dashcore::BlockHash::all_zeros(),
                1_650_000_000,
            )),
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            vec![],
            vec![],
            0,
        );
        let out_point = dashcore::OutPoint::new(tx.txid(), 0);

        // Track the lock the way a restore scan would.
        let tracked = reconstruction::reconstruct_tracked_asset_locks(
            &wallet_manager,
            &wallet_id,
            &[&record],
        )
        .await;
        assert_eq!(tracked.asset_locks.len(), 1, "sanity: the entry is tracked");

        // The sweep's own changeset, then the reinstating record's — the
        // two events a single folded drain can carry back to back.
        let mut folded = reconstruction::remove_tracked_asset_locks_for_swept(
            &wallet_manager,
            &wallet_id,
            &[tx.txid()],
        )
        .await;
        assert!(
            folded.removed.contains(&out_point),
            "sanity: the sweep produced the tombstone"
        );
        let reinstated = reconstruction::reconstruct_tracked_asset_locks(
            &wallet_manager,
            &wallet_id,
            &[&record],
        )
        .await;
        let reinstated_entry = reinstated
            .asset_locks
            .get(&out_point)
            .expect("reconstruction must re-insert the entry the sweep removed");
        assert_ne!(
            reinstated_entry.status,
            AssetLockStatus::Consumed,
            "sanity: the load-bearing premise — a reinstating reconstruction is non-Consumed"
        );
        folded.merge(reinstated);

        assert!(
            folded.removed.is_empty(),
            "the reinstating upsert must cancel the folded sweep tombstone"
        );
        assert!(
            folded.asset_locks.contains_key(&out_point),
            "and the reinstated entry rides the store round"
        );
    }

    /// The `ChainLockProcessed` arm end to end: a lock the scan
    /// reconstructed at a pre-finality status (its block wasn't
    /// chain-locked yet — the restore-scan norm) upgrades to
    /// `RecoveredFromChain` + chain proof when the chainlock
    /// promotion names its txid, in the same session, with the
    /// upgraded row riding the drained batch to the store. Before the
    /// arm existed, the promotion surfaced only as metadata and the
    /// entry stayed pre-finality until an app restart re-emitted its
    /// record.
    #[tokio::test]
    async fn chain_lock_processed_event_upgrades_reconstructed_lock() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;

        use dashcore::ephemerealdata::chain_lock::ChainLock;
        use dashcore::hashes::Hash as _;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::transaction_router::TransactionType;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use tokio::sync::Notify;

        use super::spawn_wallet_event_adapter;
        use crate::test_support::{
            funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
        };
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::asset_lock::tracked::AssetLockStatus;
        use crate::wallet::persister::WalletPersister;

        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(dashcore::Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let asset_lock_manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn crate::changeset::PlatformWalletPersistence>,
            ),
        );
        let (tx, _path) = asset_lock_manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");

        let record_with = |context: TransactionContext| {
            TransactionRecord::new(
                tx.clone(),
                AccountType::IdentityRegistration,
                context,
                TransactionType::AssetLock,
                TransactionDirection::Internal,
                vec![],
                vec![],
                0,
            )
        };

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        let (event_tx, event_rx) = unbounded_channel();
        let cancel = CancellationToken::new();
        let sync_fault = Arc::new(AtomicBool::new(false));
        let handle = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::downgrade(&persister),
            event_rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        );

        // Scan sighting in a not-yet-chain-locked block → tracked at
        // the pre-finality Broadcast status, no proof.
        event_tx
            .send(WalletEvent::BlockProcessed {
                wallet_id,
                height: 4321,
                chain_lock: None,
                inserted: vec![record_with(TransactionContext::InBlock(BlockInfo::new(
                    4321,
                    dashcore::BlockHash::all_zeros(),
                    1_650_000_000,
                )))],
                updated: vec![],
                matured: vec![],
                balance: WalletCoreBalance::default(),
                account_balances: BTreeMap::new(),
                addresses_derived: vec![],
            })
            .expect("send block event");
        let observed = obs_rx.recv().await.expect("first batch stored");
        assert_eq!(observed.n_asset_locks, 1, "pre-finality reconstruction");

        let out_point = dashcore::OutPoint::new(tx.txid(), 0);
        {
            let wm = wallet_manager.read().await;
            let lock = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("tracked entry");
            assert_eq!(lock.status, AssetLockStatus::Broadcast);
            assert!(lock.proof.is_none());
        }

        // The chainlock promotion names the txid under its funding
        // account. No record accompanies it — under the default
        // `keep-finalized-transactions=OFF` feature the promotion
        // evicted it, which is exactly why the arm must not need one.
        event_tx
            .send(WalletEvent::ChainLockProcessed {
                wallet_id,
                chain_lock: ChainLock::dummy(4321),
                locked_transactions: BTreeMap::from([(
                    AccountType::IdentityRegistration,
                    vec![tx.txid()],
                )]),
            })
            .expect("send chainlock event");

        let observed = obs_rx.recv().await.expect("second batch stored");
        assert_eq!(
            observed.n_asset_locks, 1,
            "the upgraded row must ride the chainlock drain"
        );
        {
            let wm = wallet_manager.read().await;
            let lock = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("tracked entry");
            assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
            assert!(lock.proof.is_some(), "chain proof attached");
        }

        cancel.cancel();
        handle.await.expect("adapter task joins");
    }

    /// A drain whose only payload is reconstructed asset-lock rows (no
    /// core rows at all) must still reach the store — the empty-skip
    /// predicate considers both sub-changesets.
    #[test]
    fn asset_locks_only_batch_reaches_store() {
        use dashcore::hashes::Hash as _;
        use dashcore::OutPoint;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        use crate::changeset::changeset::AssetLockEntry;
        use crate::wallet::asset_lock::tracked::AssetLockStatus;

        let wallet_id = [3u8; 32];
        let out_point = OutPoint::new(dashcore::Txid::all_zeros(), 0);
        let mut asset_locks = super::AssetLockChangeSet::default();
        asset_locks.asset_locks.insert(
            out_point,
            AssetLockEntry {
                out_point,
                transaction: dashcore::Transaction {
                    version: 3,
                    lock_time: 0,
                    input: vec![],
                    output: vec![],
                    special_transaction_payload: None,
                },
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityRegistration,
                identity_index: 0,
                amount_duffs: 1,
                status: AssetLockStatus::RecoveredFromChain,
                proof: None,
            },
        );

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            super::WalletBatch {
                core: CoreChangeSet::default(),
                asset_locks,
                payments: Default::default(),
            },
        );
        commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        let observed = obs_rx
            .try_recv()
            .expect("an asset-locks-only batch must not be skipped");
        assert_eq!(observed.n_asset_locks, 1);
        assert!(!observed.rejected);
    }

    // ── Batch-diagnostic reporting ──
    //
    // The per-drain `wallet-event batch: ...` line is read off a mainnet
    // tester's logcat to answer "is the durable watermark advancing?", so what
    // it reports is a tested property, not a comment.
    //
    // The invariant these lock down: `core.synced_height` folds into
    // `synced_height_persisted` only AFTER `persister.store(...)` accepts.
    // Folding it before the call would make a rejected store log
    // `synced_height_persisted=Some(h)` in the very same drain that faulted the
    // wallet *because* height `h`'s rows were not accepted — an internally
    // contradictory trace that points a diagnosis at the wrong subsystem.
    //
    // These drive the real `commit_batch` (the production commit path,
    // including the fail-closed guard) so the assertions cover the shipped
    // code, not a restatement of it.

    use super::{commit_batch, AssetLockChangeSet, BatchDiagnostics, SweepBatch, WalletBatch};

    /// A changeset that both proposes a watermark and carries a record-bearing
    /// field, so it survives `is_empty_no_records()` and actually reaches
    /// `store()`.
    fn watermark_with_rows(synced: u32, processed: u32) -> CoreChangeSet {
        CoreChangeSet {
            synced_height: Some(synced),
            last_processed_height: Some(processed),
            ..CoreChangeSet::default()
        }
    }

    fn one_wallet_batch(
        wallet_id: WalletId,
        core: CoreChangeSet,
    ) -> BTreeMap<WalletId, WalletBatch> {
        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            WalletBatch {
                core,
                asset_locks: AssetLockChangeSet::default(),
                payments: Default::default(),
            },
        );
        batch
    }

    /// The overlay half of [`one_verdict`], on its own — what a round's
    /// `dashpay_payments_overlay` must look like.
    fn one_verdict_overlay(
        status: crate::wallet::identity::PaymentStatus,
    ) -> super::PaymentOverlay {
        use crate::wallet::identity::PaymentEntry;
        let mut entry = PaymentEntry::new_sent(Identifier::from([0xBB; 32]), 50_000, None);
        entry.status = status;
        super::PaymentOverlay::from([(
            Identifier::from([0xAA; 32]),
            BTreeMap::from([("deadbeef".to_string(), entry)]),
        )])
    }

    /// One sent-payment verdict, in the shape the adapter folds into a
    /// wallet's batch: the bounded overlay row plus the identity snapshot
    /// that makes it survive a restart.
    fn one_verdict(status: crate::wallet::identity::PaymentStatus) -> super::SentPaymentVerdicts {
        let overlay = one_verdict_overlay(status);
        let owner = Identifier::from([0xAA; 32]);
        let mut identities = crate::changeset::IdentityChangeSet::default();
        identities.identities.insert(
            owner,
            crate::changeset::IdentityEntry {
                id: owner,
                balance: 0,
                revision: 0,
                identity_index: None,
                last_updated_balance_block_time: None,
                last_synced_keys_block_time: None,
                dpns_names: Vec::new(),
                contested_dpns_names: Vec::new(),
                status: Default::default(),
                wallet_id: None,
                dashpay_profile: None,
                dashpay_payments: overlay.get(&owner).cloned().unwrap_or_default(),
                contact_profiles: Default::default(),
                ignored_senders: Default::default(),
            },
        );
        super::SentPaymentVerdicts {
            overlay,
            identities,
        }
    }

    /// A persister that attests `DASHPAY_PAYMENTS` gets the verdict on the
    /// same round as everything else the drain folded.
    #[test]
    fn a_verdict_reaches_a_persister_that_attests_dashpay_payments() {
        use crate::wallet::identity::PaymentStatus;
        let wallet_id = [0x31u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::with_capabilities(
            obs_tx,
            crate::changeset::PersistenceCapabilities::DASHPAY_PAYMENTS,
        );
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            WalletBatch {
                core: watermark_with_rows(700, 700),
                asset_locks: AssetLockChangeSet::default(),
                payments: one_verdict(PaymentStatus::Failed),
            },
        );
        let diag = commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        let observed = obs_rx.try_recv().expect("the round reaches store()");
        assert_eq!(
            observed.dashpay_payments,
            Some(one_verdict_overlay(PaymentStatus::Failed)),
            "the verdict must ride the same store() as the rows that justify it"
        );
        assert_eq!(
            observed.dashpay_identity_payments,
            Some(one_verdict_overlay(PaymentStatus::Failed)),
            "and the identity snapshot must ride it too, or the verdict is undone \
             by the next load()"
        );
        assert_eq!(diag.persisted, Some(700));
        assert_eq!(diag.faulted, 0, "a payments-capable host is not a fault");
    }

    /// A persister that never attested `DASHPAY_PAYMENTS` cannot apply the
    /// overlay, so handing it one would let the round return `Ok` while the
    /// verdict was silently dropped. Withhold it instead — and unlike a
    /// withheld sweep this does NOT freeze the watermark: the verdict is
    /// derived state a later host re-derives, whereas a dropped removal has
    /// no recovery.
    #[test]
    fn a_verdict_is_withheld_from_a_persister_without_dashpay_payments() {
        use crate::wallet::identity::PaymentStatus;
        let wallet_id = [0x32u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        // No capabilities declared — the payments-blind host.
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            WalletBatch {
                core: watermark_with_rows(700, 700),
                asset_locks: AssetLockChangeSet::default(),
                payments: one_verdict(PaymentStatus::Failed),
            },
        );
        let diag = commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        let observed = obs_rx.try_recv().expect("the rest of the round still runs");
        assert_eq!(
            observed.dashpay_payments, None,
            "a payments-blind persister must never be handed an overlay"
        );
        assert_eq!(
            observed.dashpay_identity_payments, None,
            "both carriers are withheld together — a snapshot smuggling the \
             verdict past the gate would make DASHPAY_PAYMENTS meaningless"
        );
        assert_eq!(
            diag.persisted,
            Some(700),
            "the rest of the round is unaffected — only the overlay is withheld"
        );
        assert_eq!(diag.frozen, None, "a withheld verdict does not freeze");
        assert_eq!(diag.faulted, 0);
        assert!(!sync_fault.load(Ordering::Relaxed));
    }

    /// A round whose ONLY content is a verdict the capability gate withholds
    /// has nothing left to persist, so it must skip the store round-trip
    /// entirely rather than send an empty changeset.
    #[test]
    fn a_withheld_verdict_alone_never_reaches_the_store() {
        use crate::wallet::identity::PaymentStatus;
        let wallet_id = [0x33u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            WalletBatch {
                core: CoreChangeSet::default(),
                asset_locks: AssetLockChangeSet::default(),
                payments: one_verdict(PaymentStatus::Failed),
            },
        );
        commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );
        assert!(
            obs_rx.try_recv().is_err(),
            "nothing left to persist must not reach store()"
        );
    }

    /// The mirror of the case above: a verdict is the ONLY thing a round
    /// carries when the wallet's other projections are empty — a
    /// `TransactionInstantLocked` for an already chain-locked txid projects
    /// no core rows at all. That round must still reach the store, or the
    /// verdict is lost with no event left to re-derive it from.
    #[test]
    fn a_verdict_alone_still_reaches_a_capable_store() {
        use crate::wallet::identity::PaymentStatus;
        let wallet_id = [0x34u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::with_capabilities(
            obs_tx,
            crate::changeset::PersistenceCapabilities::DASHPAY_PAYMENTS,
        );
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            WalletBatch {
                core: CoreChangeSet::default(),
                asset_locks: AssetLockChangeSet::default(),
                payments: one_verdict(PaymentStatus::Confirmed),
            },
        );
        commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );
        let observed = obs_rx
            .try_recv()
            .expect("a verdict-only round must still be stored");
        assert_eq!(
            observed.dashpay_payments,
            Some(one_verdict_overlay(PaymentStatus::Confirmed))
        );
        assert_eq!(
            observed.dashpay_identity_payments,
            Some(one_verdict_overlay(PaymentStatus::Confirmed))
        );
    }

    /// The sweep guard strips the height BEFORE the store sees it, so a
    /// backend that never attested `CORE_SWEEP_REMOVAL` and returns `Ok`
    /// must report the height as FROZEN, not rejected: `rejected` would
    /// send an operator to a persister that in fact accepted the round,
    /// when the missing piece is the host's sweep capability.
    #[test]
    fn undeclared_sweep_capability_reports_the_watermark_as_frozen_not_rejected() {
        use dashcore::hashes::Hash as _;
        let wallet_id = [9u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        // No capabilities declared, store() succeeds.
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut core = watermark_with_rows(600, 600);
        core.sweeps = vec![SweepBatch {
            txids: vec![dashcore::Txid::from_byte_array([0x61; 32])],
            superseded_by: dashcore::Txid::from_byte_array([0x62; 32]),
            winner_mined_height: Some(590),
            released_outpoints: vec![],
        }];
        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, core),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        let observed = obs_rx.try_recv().expect("the round still reaches store()");
        assert!(!observed.rejected, "the probe's own store() succeeds");
        assert_eq!(
            observed.synced_height, None,
            "the guard stripped the height before the store saw it"
        );
        assert_eq!(diag.persisted, None);
        assert_eq!(
            diag.frozen,
            Some(600),
            "a height the adapter withheld is reported under `frozen`"
        );
        assert_eq!(
            diag.rejected, None,
            "…and never as rejected: the store did not reject anything"
        );
        assert_eq!(diag.faulted, 1);
        assert!(sync_fault.load(Ordering::Relaxed));
        let line = diag.to_string();
        assert!(line.contains("synced_height_frozen=Some(600)"), "{line}");
        assert!(line.contains("synced_height_rejected=None"), "{line}");
    }

    /// Baseline: a height the store ACCEPTED is the one case that may be
    /// reported as persisted.
    #[test]
    fn accepted_store_reports_the_watermark_as_persisted() {
        let wallet_id = [1u8; 32];
        let (obs_tx, _obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(500, 500)),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        assert_eq!(diag.persisted, Some(500));
        assert_eq!(diag.frozen, None);
        assert_eq!(diag.rejected, None);
        assert_eq!(diag.faulted, 0);
        assert!(!sync_fault.load(Ordering::Relaxed));
        assert!(diag
            .to_string()
            .contains("synced_height_persisted=Some(500)"));
    }

    /// Invariant: a REJECTED `store()` must never be
    /// reported as persisted.
    #[test]
    fn rejected_store_is_not_reported_as_persisted() {
        let wallet_id = [9u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        persister.fail_next(wallet_id);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(500, 500)),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        // The height was genuinely offered to the store...
        let observed = obs_rx.try_recv().expect("the rejected store still ran");
        assert!(observed.rejected);
        assert_eq!(observed.synced_height, Some(500));

        // ...and the store rejected it, so it is NOT on disk.
        assert_eq!(
            diag.persisted, None,
            "a rejected watermark must never be counted as persisted"
        );
        assert_eq!(
            diag.rejected,
            Some(500),
            "the rejected height belongs under its own field"
        );
        assert_eq!(
            diag.frozen, None,
            "the guard did not strip this one — the store rejected it"
        );
        assert_eq!(diag.faulted, 1);

        // The rendered logcat line must not claim the height reached disk.
        let line = diag.to_string();
        assert!(
            line.contains("synced_height_persisted=None"),
            "logcat line must report no persisted watermark: {line}"
        );
        assert!(
            !line.contains("synced_height_persisted=Some(500)"),
            "logcat line must not report the rejected height as persisted: {line}"
        );
        assert!(
            line.contains("synced_height_rejected=Some(500)"),
            "logcat line must surface the rejected height: {line}"
        );

        // The fail-closed guard itself is untouched: the wallet is faulted, the
        // host-visible signal is latched, and the one-shot frozen marker fired.
        assert!(fault.is_faulted(&wallet_id));
        assert!(sync_fault.load(Ordering::Relaxed));
        assert!(
            freeze_logged.load(Ordering::Relaxed),
            "the one-shot SYNC WATERMARK FROZEN marker must have been emitted"
        );
    }

    /// A watermark withheld by the fail-closed guard is reported as `frozen` —
    /// never as persisted, and distinguishably from a drain that simply carried
    /// no watermark at all.
    #[test]
    fn guard_stripped_watermark_is_reported_as_frozen_not_persisted() {
        let wallet_id = [9u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        // Pre-fault the wallet, as an earlier drain's rejection would have.
        fault.fault_wallet(wallet_id, &sync_fault);
        let freeze_logged = AtomicBool::new(true); // one-shot already spent

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(900, 900)),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        assert_eq!(diag.persisted, None, "a frozen watermark is not persisted");
        assert_eq!(diag.frozen, Some(900));
        assert_eq!(diag.rejected, None);
        assert_eq!(diag.faulted, 1);

        // Guard behaviour unchanged: the store never saw the height, while the
        // record-bearing field still persisted.
        let observed = obs_rx
            .try_recv()
            .expect("record-bearing rows still persist while frozen");
        assert_eq!(
            observed.synced_height, None,
            "the guard must strip the watermark before it reaches the store"
        );
        assert_eq!(observed.last_processed_height, Some(900));

        let line = diag.to_string();
        assert!(line.contains("synced_height_persisted=None"), "{line}");
        assert!(line.contains("synced_height_frozen=Some(900)"), "{line}");
    }

    /// A watermark-ONLY changeset under the guard is stripped to empty and
    /// skips the store round-trip entirely. It must still be reported as
    /// frozen, otherwise that drain reads as idle.
    #[test]
    fn frozen_watermark_only_batch_is_still_reported() {
        let wallet_id = [9u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        fault.fault_wallet(wallet_id, &sync_fault);
        let freeze_logged = AtomicBool::new(true);

        let core = CoreChangeSet {
            synced_height: Some(1234),
            ..CoreChangeSet::default()
        };
        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, core),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        assert_eq!(diag.frozen, Some(1234));
        assert_eq!(diag.persisted, None);
        assert_eq!(diag.rejected, None);
        assert!(
            obs_rx.try_recv().is_err(),
            "a stripped watermark-only batch must skip the store round-trip"
        );
    }

    /// One drain can span a healthy wallet and a rejecting one. Each outcome is
    /// reported under its own field instead of collapsing into a single
    /// "persisted" number that would over-report the rejecting wallet.
    #[test]
    fn mixed_batch_reports_persisted_and_rejected_separately() {
        let healthy = [1u8; 32];
        let rejecting = [2u8; 32];
        let (obs_tx, _obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        persister.fail_next(rejecting);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let freeze_logged = AtomicBool::new(false);

        let mut batch = BTreeMap::new();
        batch.extend(one_wallet_batch(healthy, watermark_with_rows(10, 10)));
        batch.extend(one_wallet_batch(rejecting, watermark_with_rows(20, 20)));

        let diag = commit_batch(
            &persister,
            batch,
            2,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        assert_eq!(
            diag.persisted,
            Some(10),
            "only the healthy wallet's height landed"
        );
        assert_eq!(
            diag.rejected,
            Some(20),
            "the rejecting wallet's height did not land"
        );
        assert_eq!(diag.wallets, 2);
        assert_eq!(diag.folded, 2);
        assert!(
            !fault.is_faulted(&healthy),
            "a sibling wallet must not be faulted by another's rejection"
        );
        assert!(fault.is_faulted(&rejecting));
    }

    /// A wallet that entered the drain already faulted and whose store is
    /// rejected AGAIN counts once, not twice: `faulted` is a wallet count and
    /// must never exceed `wallets`.
    #[test]
    fn repeat_rejection_of_a_faulted_wallet_counts_once() {
        let wallet_id = [9u8; 32];
        let (obs_tx, _obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        persister.fail_next(wallet_id);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        // Pre-fault the wallet, as an earlier drain's rejection would have.
        fault.fault_wallet(wallet_id, &sync_fault);
        let freeze_logged = AtomicBool::new(true);

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(700, 700)),
            1,
            &mut fault,
            &sync_fault,
            &freeze_logged,
            &mut Vec::new(),
        );

        assert_eq!(diag.wallets, 1);
        assert_eq!(
            diag.faulted, 1,
            "an already-faulted wallet rejected again must count once"
        );
        assert_eq!(diag.frozen, Some(700), "the guard stripped the watermark");
        assert_eq!(
            diag.rejected, None,
            "the stripped changeset offered no watermark to reject"
        );
    }

    /// Each field takes its monotonic max independently, so a lower height
    /// later in a drain cannot pull a reported watermark backwards and the
    /// three outcomes never bleed into each other.
    #[test]
    fn diagnostics_fields_take_independent_monotonic_max() {
        let mut diag = BatchDiagnostics::new(3, 3);
        diag.record_persisted(10);
        diag.record_persisted(4); // lower — must not regress
        diag.record_frozen(7);
        diag.record_rejected(99);
        assert_eq!(diag.persisted, Some(10));
        assert_eq!(diag.frozen, Some(7));
        assert_eq!(diag.rejected, Some(99));
    }

    /// The exact logcat contract a tester greps for.
    #[test]
    fn diagnostic_line_format_is_stable() {
        let mut diag = BatchDiagnostics::new(512, 2);
        diag.record_persisted(100);
        diag.record_rejected(200);
        assert_eq!(
            diag.to_string(),
            "wallet-event batch: folded=512 wallets=2 synced_height_persisted=Some(100) \
             synced_height_frozen=None synced_height_rejected=Some(200) faulted=0"
        );
    }

    // ── Verdicts through the real adapter drain ──
    //
    // The unit tests above call `sent_payment_verdicts` directly, and the
    // `commit_batch` tests inject a ready `SentPaymentVerdicts` into a
    // `WalletBatch`. Neither notices if the call disappears from one of the
    // two fold sites in `run_wallet_event_adapter`, or if its result is
    // dropped on the floor. These drive the loop itself and assert on what
    // reaches the store.
    //
    // Determinism comes from buffering: every event is queued on the
    // unbounded channel and the sender is dropped BEFORE the adapter is
    // spawned, so the first `recv` and the `try_recv` fold that follows see
    // the whole sequence and commit it as exactly one round. No sleeping, no
    // racing the drain.

    use super::sent_payment_verdict_tests::{
        chainlocked_reinstatement, owner, sent_transaction, stored_status, sweep_of,
        wallet_with_payment,
    };
    use crate::wallet::identity::{PaymentDirection, PaymentStatus};

    /// A probe that attests both bits this round needs: `CORE_SWEEP_REMOVAL`
    /// so the sweep half is not withheld (which would fault the wallet and
    /// strip the watermark), and `DASHPAY_PAYMENTS` so the verdict is not.
    fn sweep_and_payments_probe(obs: UnboundedSender<StoreObserved>) -> Arc<ProbePersister> {
        Arc::new(ProbePersister::with_capabilities(
            obs,
            crate::changeset::PersistenceCapabilities::CORE_SWEEP_REMOVAL
                .union(crate::changeset::PersistenceCapabilities::DASHPAY_PAYMENTS),
        ))
    }

    /// Run `events` through the real adapter as one buffered drain and
    /// return the single `store()` it produced.
    async fn one_drained_round(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        events: Vec<WalletEvent>,
    ) -> StoreObserved {
        let (tx, rx) = unbounded_channel::<WalletEvent>();
        for event in events {
            tx.send(event).unwrap();
        }
        // Dropped before the adapter starts: the backlog is already buffered,
        // so the drain folds all of it and then exits on `Disconnected`.
        drop(tx);

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = sweep_and_payments_probe(obs_tx);
        let handle = tokio::spawn(run_wallet_event_adapter(
            wallet_manager,
            Arc::downgrade(&persister),
            rx,
            Arc::new(AtomicBool::new(false)),
            CancellationToken::new(),
        ));
        handle.await.expect("the adapter exits on a closed channel");

        let observed = obs_rx
            .recv()
            .await
            .expect("the buffered drain must reach store()");
        assert!(
            obs_rx.try_recv().is_err(),
            "a buffered drain commits once — a second round would mean the \
             verdict and the rows that justify it were split across stores"
        );
        observed
    }

    /// The row the round must carry, in both carriers, with the assertion
    /// that there is exactly one of it: a verdict is a status flip, never a
    /// replay of the identity's payment history.
    fn sole_verdict_row(observed: &StoreObserved, txid: &str) -> PaymentStatus {
        let overlay = observed
            .dashpay_payments
            .as_ref()
            .expect("the round must carry the overlay");
        assert_eq!(overlay.len(), 1, "exactly one identity: {overlay:?}");
        let rows = overlay.get(&owner()).expect("the owning identity");
        assert_eq!(rows.len(), 1, "exactly one row: {rows:?}");
        let status = rows.get(txid).expect("the flipped row").status;

        let snapshot = observed
            .dashpay_identity_payments
            .as_ref()
            .expect("the round must carry the identity snapshot too");
        assert_eq!(
            snapshot
                .get(&owner())
                .and_then(|p| p.get(txid))
                .map(|e| e.status),
            Some(status),
            "the authoritative carrier must agree with the overlay — `load()` \
             rebuilds payments from the snapshot, never from the overlay table"
        );
        status
    }

    /// A sweep and the finality that overrules it, buffered into one drain in
    /// that order. The store must see one row at the verdict the drain ended
    /// on (`Confirmed -> Failed -> Confirmed` collapses to `Confirmed`), on
    /// the same round as the record that justifies it.
    ///
    /// Starting at `Confirmed` rather than `Pending` is what makes this test
    /// discriminate BOTH fold sites. `Confirmed + Final` is a no-op edge, so
    /// an adapter that skipped the first event's verdict would emit no row at
    /// all rather than the same one by a different route; and an adapter that
    /// skipped the folded events' verdicts would stop at `Failed`.
    #[tokio::test]
    async fn a_buffered_sweep_then_final_reaches_the_store_as_one_confirmed_row() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Confirmed).await;

        let observed = one_drained_round(
            Arc::clone(&wallet_manager),
            vec![
                sweep_of(wallet_id, sent_transaction().txid()),
                chainlocked_reinstatement(wallet_id),
            ],
        )
        .await;

        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            sole_verdict_row(&observed, &txid),
            PaymentStatus::Confirmed,
            "last write wins inside a drain"
        );
        assert_eq!(
            observed.n_sweeps, 0,
            "the reinstating record retracts the sweep inside the same core \
             merge (see `CoreChangeSet::merge`), so this round carries the \
             record and no removal — exactly what the Confirmed verdict says"
        );
        assert_eq!(
            observed.n_records, 1,
            "the verdict rides the same round as the record that justifies it"
        );
        assert_eq!(
            observed.last_processed_height,
            Some(1_499_060),
            "and alongside the core watermark the same drain projected"
        );
        assert!(!observed.rejected);
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Confirmed,
            "memory and the round must agree"
        );
    }

    /// The same two events in the opposite order. `Failed -> Confirmed ->
    /// Failed` collapses to `Failed`, which is the verdict that matters:
    /// nothing else in the wallet ever writes it, and it must not be lost to
    /// the confirm that preceded it in the same drain.
    ///
    /// Starting at `Failed` is the mirror of the test above: `Failed + Swept`
    /// is the no-op edge here, so dropping either fold site's verdict changes
    /// what reaches the store rather than arriving at it another way.
    #[tokio::test]
    async fn a_buffered_final_then_sweep_reaches_the_store_as_one_failed_row() {
        let (wallet_manager, wallet_id, txid) =
            wallet_with_payment(PaymentDirection::Sent, PaymentStatus::Failed).await;

        let observed = one_drained_round(
            Arc::clone(&wallet_manager),
            vec![
                chainlocked_reinstatement(wallet_id),
                sweep_of(wallet_id, sent_transaction().txid()),
            ],
        )
        .await;

        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            sole_verdict_row(&observed, &txid),
            PaymentStatus::Failed,
            "the sweep is the later evidence, so it is the drain's verdict"
        );
        assert_eq!(
            observed.n_sweeps, 1,
            "the verdict must ride the sweep's own round — that coupling is \
             the whole point of resolving it here, since a sweep never \
             re-emits once its round is durable"
        );
        assert_eq!(observed.last_processed_height, Some(1_499_060));
        assert!(!observed.rejected);
        assert_eq!(
            stored_status(&wallet_manager, &wallet_id, &txid).await,
            PaymentStatus::Failed
        );
    }
}

#[cfg(test)]
mod utxo_credit_verdict_tests {
    //! Coverage for [`utxo_credit_verdicts_from_wallet`] — the engine's
    //! verdict on outputs a record classifies as ours but the engine never
    //! credited. Drives a real `ManagedWalletInfo` through
    //! `check_core_transaction` in the exact arrival orders that produce
    //! each verdict, then runs the bridge's derivation over the
    //! post-mutation state, as the event adapter does at runtime.

    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::transaction_checking::{BlockInfo, WalletTransactionChecker};
    use key_wallet::WalletCoreBalance;
    use key_wallet_manager::WalletManager;

    fn in_block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::from_slice(&[9u8; 32]).expect("valid block hash"),
            1_234_567_890,
        ))
    }

    /// A P2PKH script the wallet does not monitor (the secp256k1
    /// generator point), for counterparty outputs.
    fn foreign_script() -> ScriptBuf {
        const TEST_PUBKEY_G: [u8; 33] = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
            0x5b, 0x16, 0xf8, 0x17, 0x98,
        ];
        let pubkey =
            dashcore::PublicKey::from_slice(&TEST_PUBKEY_G).expect("generator point is valid");
        dashcore::Address::p2pkh(&pubkey, key_wallet::Network::Testnet).script_pubkey()
    }

    fn input(previous_output: OutPoint) -> TxIn {
        TxIn {
            previous_output,
            script_sig: ScriptBuf::new(),
            sequence: 0xffffffff,
            witness: Witness::new(),
        }
    }

    fn spend_to(previous_output: OutPoint, script_pubkey: ScriptBuf, value: u64) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![input(previous_output)],
            output: vec![TxOut {
                value,
                script_pubkey,
            }],
            special_transaction_payload: None,
        }
    }

    /// The rust-dashcore#992 shape: one input (the coin) and a sole
    /// zero-value `OP_RETURN` output — a CoinJoin collateral burn. It pays
    /// nothing back to the wallet, so its only tie to us is the input.
    fn collateral_burn(coin: OutPoint) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![input(coin)],
            output: vec![TxOut {
                value: 0,
                script_pubkey: dashcore::blockdata::script::Builder::new()
                    .push_opcode(dashcore::opcodes::all::OP_RETURN)
                    .into_script(),
            }],
            special_transaction_payload: None,
        }
    }

    fn funding_of(receive_script: ScriptBuf, seed: u8) -> Transaction {
        spend_to(
            OutPoint {
                txid: Txid::from_slice(&[seed; 32]).expect("valid txid"),
                vout: 0,
            },
            receive_script,
            19_549,
        )
    }

    /// The field case: the burn is processed BEFORE the funding output is
    /// recognised, matches nothing, and is discarded — but the #649 map
    /// notes the spend. When the funding record then arrives it classifies
    /// the output `Received` and the persister would materialise an unspent
    /// row, while the engine skipped the credit. The verdict names the
    /// observed spending height.
    #[tokio::test]
    async fn collateral_burn_seen_before_its_funding_yields_observed_spent() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();
        let fund_tx = funding_of(receive_address.script_pubkey(), 2);
        let coin = OutPoint {
            txid: fund_tx.txid(),
            vout: 0,
        };

        let burn_result = managed_wallet
            .check_core_transaction(
                &collateral_burn(coin),
                in_block(100_001),
                &mut wallet,
                true,
                true,
            )
            .await;
        assert!(
            !burn_result.is_relevant,
            "a burn of an unknown coin matches nothing"
        );
        assert!(burn_result.new_records.is_empty());
        assert!(burn_result.updated_records.is_empty());

        let fund_result = managed_wallet
            .check_core_transaction(&fund_tx, in_block(100_000), &mut wallet, true, true)
            .await;
        assert!(fund_result.is_relevant);
        let funding_record = fund_result
            .new_records
            .first()
            .expect("the funding transaction is recorded");
        assert!(
            funding_record
                .output_details
                .iter()
                .any(|d| d.index == 0 && d.role == OutputRole::Received),
            "the record still classifies the output as ours"
        );
        let funds = managed_wallet
            .first_bip44_managed_account()
            .expect("bip44 account");
        assert!(
            !funds.utxos.contains_key(&coin),
            "the engine never credited the coin"
        );

        let verdicts = utxo_credit_verdicts_from_wallet(&managed_wallet, &[funding_record]);
        assert_eq!(
            verdicts.get(&coin),
            Some(&UtxoCreditVerdict::ObservedSpent { height: 100_001 })
        );
        assert_eq!(verdicts.len(), 1);
    }

    /// The ordinary case carries no verdict at all: a credited output must
    /// keep today's behaviour byte for byte.
    #[tokio::test]
    async fn credited_output_yields_no_verdict() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();
        let fund_tx = funding_of(receive_address.script_pubkey(), 3);
        let fund_result = managed_wallet
            .check_core_transaction(&fund_tx, in_block(100_000), &mut wallet, true, true)
            .await;
        let funding_record = fund_result.new_records.first().expect("funding record");
        let funds = managed_wallet
            .first_bip44_managed_account()
            .expect("bip44 account");
        assert!(funds.utxos.contains_key(&OutPoint {
            txid: fund_tx.txid(),
            vout: 0
        }));

        assert!(utxo_credit_verdicts_from_wallet(&managed_wallet, &[funding_record]).is_empty());
    }

    /// A mempool transaction whose input a block already spent is recorded
    /// (history keeps the attempt) but credits nothing — and no sweep will
    /// ever delete its row, since the winner arrived first. Its outputs
    /// read `Doomed`.
    #[tokio::test]
    async fn doomed_mempool_record_yields_doomed() {
        let TestWalletContext {
            mut managed_wallet,
            mut wallet,
            receive_address,
            ..
        } = TestWalletContext::new_random();
        let fund_tx = funding_of(receive_address.script_pubkey(), 4);
        let coin = OutPoint {
            txid: fund_tx.txid(),
            vout: 0,
        };
        managed_wallet
            .check_core_transaction(&fund_tx, in_block(100_000), &mut wallet, true, true)
            .await;
        // The winner: a block spend of the coin to a stranger.
        let winner = spend_to(coin, foreign_script(), 19_000);
        let winner_result = managed_wallet
            .check_core_transaction(&winner, in_block(100_001), &mut wallet, true, true)
            .await;
        assert!(winner_result.is_relevant);
        // The loser arrives afterwards from the mempool, paying us back.
        let loser = spend_to(coin, receive_address.script_pubkey(), 18_000);
        let loser_result = managed_wallet
            .check_core_transaction(&loser, TransactionContext::Mempool, &mut wallet, true, true)
            .await;
        assert!(loser_result.is_relevant, "it pays one of our addresses");
        let loser_record = loser_result.new_records.first().expect("loser record");
        let loser_output = OutPoint {
            txid: loser.txid(),
            vout: 0,
        };
        let funds = managed_wallet
            .first_bip44_managed_account()
            .expect("bip44 account");
        assert!(!funds.utxos.contains_key(&loser_output));

        let verdicts = utxo_credit_verdicts_from_wallet(&managed_wallet, &[loser_record]);
        assert_eq!(
            verdicts.get(&loser_output),
            Some(&UtxoCreditVerdict::Doomed)
        );
    }

    /// End to end through the event bridge: the funding record's
    /// `BlockProcessed` changeset carries the verdict next to the very
    /// `new_utxos` entry the persister would otherwise trust, and a wallet
    /// the manager does not know yields no verdict rather than a wrong one.
    #[tokio::test]
    async fn block_processed_changeset_carries_the_verdict() {
        use crate::wallet::core::WalletGeneration;
        use crate::wallet::identity::IdentityManager;

        let mut ctx = TestWalletContext::new_random();
        let fund_tx = funding_of(ctx.receive_address.script_pubkey(), 5);
        let coin = OutPoint {
            txid: fund_tx.txid(),
            vout: 0,
        };
        let burn_result = ctx
            .check_transaction(&collateral_burn(coin), in_block(100_001))
            .await;
        assert!(!burn_result.is_relevant);
        let fund_result = ctx.check_transaction(&fund_tx, in_block(100_000)).await;
        let funding_record = fund_result
            .new_records
            .first()
            .expect("funding record")
            .clone();

        let info = PlatformWalletInfo {
            core_wallet: ctx.managed_wallet,
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
            observed_input_conflicts: Default::default(),
        };
        let mut wm = WalletManager::<PlatformWalletInfo>::new(dashcore::Network::Testnet);
        let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
        let manager = Arc::new(RwLock::new(wm));

        let event = |wallet_id: WalletId| WalletEvent::BlockProcessed {
            wallet_id,
            height: 100_000,
            chain_lock: None,
            inserted: vec![funding_record.clone()],
            updated: vec![],
            matured: vec![],
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };

        let cs = build_core_changeset(&manager, &event(wallet_id)).await;
        assert_eq!(
            cs.utxo_credit_verdicts.get(&coin),
            Some(&UtxoCreditVerdict::ObservedSpent { height: 100_001 })
        );
        assert!(
            cs.new_utxos.iter().any(|u| u.outpoint == coin),
            "the additive projection is unchanged; the verdict rides beside it"
        );
        assert!(!cs.is_empty_no_records());

        let unknown = build_core_changeset(&manager, &event([0xEEu8; 32])).await;
        assert!(unknown.utxo_credit_verdicts.is_empty());
    }

    /// `TransactionDetected` judges the outputs of the records it READS,
    /// under the guard it reads them with — never the event's own record,
    /// which is a clone taken at emit time and can be stale by drain time.
    /// Here the funding was still in the mempool when emitted; since then
    /// it confirmed (its inputs now sit in the observed-spent map) and a
    /// mempool child took the coin. Judged on the stale clone the coin
    /// reads `Doomed` — a durable spent mark for a spend that may never
    /// confirm. Judged on the manager's own confirmed record it reads
    /// `Uncredited`, and the store learns the rest from the child's record.
    #[tokio::test]
    async fn transaction_detected_judges_the_records_it_reads_not_the_stale_event_clone() {
        use crate::wallet::core::WalletGeneration;
        use crate::wallet::identity::IdentityManager;

        let mut ctx = TestWalletContext::new_random();
        let fund_tx = funding_of(ctx.receive_address.script_pubkey(), 6);
        let coin = OutPoint {
            txid: fund_tx.txid(),
            vout: 0,
        };
        let seen_in_mempool = ctx
            .check_transaction(&fund_tx, TransactionContext::Mempool)
            .await;
        let stale_clone = seen_in_mempool
            .new_records
            .first()
            .expect("mempool funding record")
            .clone();
        assert!(matches!(stale_clone.context, TransactionContext::Mempool));
        // The funding confirms: its inputs enter the observed-spent map.
        assert!(
            ctx.check_transaction(&fund_tx, in_block(100_000))
                .await
                .is_relevant
        );
        // A mempool child takes the coin.
        let child = spend_to(coin, foreign_script(), 19_000);
        assert!(
            ctx.check_transaction(&child, TransactionContext::Mempool)
                .await
                .is_relevant
        );
        assert!(!ctx
            .managed_wallet
            .first_bip44_managed_account()
            .expect("bip44 account")
            .utxos
            .contains_key(&coin));
        // The stale clone, judged against the wallet as it is now, WOULD
        // read `Doomed`: that is the wrong verdict the bridge must not emit.
        assert_eq!(
            utxo_credit_verdicts_from_wallet(&ctx.managed_wallet, &[&stale_clone]).get(&coin),
            Some(&UtxoCreditVerdict::Doomed)
        );

        let info = PlatformWalletInfo {
            core_wallet: ctx.managed_wallet,
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
            observed_input_conflicts: Default::default(),
        };
        let mut wm = WalletManager::<PlatformWalletInfo>::new(dashcore::Network::Testnet);
        let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
        let manager = Arc::new(RwLock::new(wm));

        let event = WalletEvent::TransactionDetected {
            wallet_id,
            record: Box::new(stale_clone),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: vec![],
        };
        let cs = build_core_changeset(&manager, &event).await;
        assert_eq!(
            cs.utxo_credit_verdicts.get(&coin),
            Some(&UtxoCreditVerdict::Uncredited),
            "judged on the manager's confirmed record: taken, not doomed"
        );
        assert!(
            cs.records.iter().all(|r| r.context.block_info().is_some()),
            "the row is rebuilt from the manager's record, not the stale clone"
        );
    }
}
