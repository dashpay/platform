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
//! rows it implies and freeze forever (dashpay/platform#4069). The unbounded
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
use std::sync::{Arc, Mutex};

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
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
    AssetLockChangeSet, CoreChangeSet, HighestUsedIndexes, PlatformWalletChangeSet,
};
use crate::changeset::merge::Merge;
use crate::changeset::traits::PlatformWalletPersistence;
use crate::wallet::asset_lock::sync::reconstruction;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Maximum number of `WalletEvent`s folded into a single
/// `persister.store(..)` round-trip by [`run_wallet_event_adapter`].
///
/// # Why batch at all (dashpay/platform#4069 follow-up)
///
/// This adapter's per-event cost is overwhelmingly the persister call: on
/// Android that is a JNI hop into a Room transaction (milliseconds), while
/// projecting a `WalletEvent` into a [`CoreChangeSet`] is microseconds.
/// Storing one event per `store()` therefore pinned the drain rate at
/// roughly the *store* rate — low hundreds per second — which is far below
/// what a historical SPV catch-up emits. Draining the lossless persistence
/// channel one store at a time let its backlog grow without bound during a
/// catch-up (and, on the old bounded broadcast this consumer used to read,
/// overflowed the ring and froze the watermark — the root cause the
/// dedicated unbounded channel removes).
///
/// Folding every event *already buffered* in the channel into one changeset
/// per wallet collapses a burst of N events into a single store, so the
/// drain keeps pace with the producer at projection speed. This is
/// exactly the fold [`Merge`] was specified for — `CoreChangeSet` merging
/// is commutative and associative, and its doc comment already anticipates
/// "a flush can fold multiple events together (TransactionDetected +
/// BlockProcessed for the same wallet over a sync round)".
///
/// The cap bounds the worst-case size of a single merged changeset (and
/// hence one Room transaction), and keeps a saturated producer from
/// starving the cancellation branch of the select below.
const ADAPTER_STORE_BATCH_LIMIT: usize = 512;

/// Session fault state for the durable-watermark guard
/// (dashpay/platform#4069).
///
/// Now that the persistence channel is a lossless unbounded `mpsc`, the only
/// remaining fault trigger is a **`store()` rejection**, which carries a
/// `wallet_id`, so only THAT wallet's watermark freezes. A sibling wallet
/// whose rows are still landing atomically keeps advancing — freezing it too
/// would force a redundant rescan of a wallet that never lost a row.
///
/// The old global (`broadcast::Lagged`) latch is gone: the unbounded channel
/// can never `Lagged`, so there is no more "dropped events of unknown wallet"
/// signal to freeze everything for. This per-wallet freeze remains as a
/// fail-closed backstop — in a healthy run it never fires.
#[derive(Default)]
struct AdapterFaultState {
    /// Set by a `store()` rejection: freezes only the named wallet.
    per_wallet: HashMap<WalletId, bool>,
}

impl AdapterFaultState {
    /// Whether `wallet_id`'s durable watermark must be held frozen.
    fn is_faulted(&self, wallet_id: &WalletId) -> bool {
        self.per_wallet.get(wallet_id).copied().unwrap_or(false)
    }

    /// Fault a single wallet after its `store()` was rejected, and raise
    /// the host-visible hard-fault signal.
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
/// persistence channel's sender (the manager) is dropped.
///
/// `sync_fault` is the host-visible hard-fault latch: the task sets it
/// (and never clears it) the first time it freezes a durable watermark, so
/// an integrator can surface "verification failed / rescan pending" rather
/// than silently re-freezing on the next launch.
///
/// Generic over `P` so the spawned task gets static-dispatch on
/// every `persister.store(...)` call. Pass the manager's own
/// `Arc<P>` (not the `Arc<dyn PlatformWalletPersistence>`
/// coercion) to actually realize the static-dispatch win.
pub fn spawn_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<P>,
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
/// # Lossless persistence channel (dashpay/platform#4069)
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
/// This closes the historical freeze: previously this consumer read the
/// manager's *bounded* broadcast ring, and during a historical SPV catch-up
/// the manager processed blocks far faster than this single-threaded adapter
/// could drain them through the (slow, JNI + Room) persister, so the ring
/// overflowed and `recv()` returned `Lagged` — the dropped events being
/// exactly the record/UTXO/spent-marker events, while the bare
/// `SyncHeightAdvanced` watermark kept flowing and advanced the persisted
/// `syncedHeight` past blocks whose rows never reached disk. The durable
/// watermark then outran its rows and the guard below latched it frozen
/// forever. With the lossless channel that path no longer exists.
///
/// # Durable-watermark guard (fail-closed backstop)
///
/// One fault trigger remains: a rejected `store()` (the rows for that batch
/// are not on disk). When a wallet faults this way, we never advance ITS
/// persisted sync watermark again this session — [`freeze_synced_height_if_faulted`]
/// strips `synced_height` from every subsequent changeset, holding the
/// durable watermark at the last height whose rows were fully committed.
/// Records/UTXO deltas in the same changeset still persist; only the height
/// advance is suppressed. On the next launch the SPV scan resumes from that
/// (lower) watermark and the persister's idempotent upserts re-apply the
/// missing rows. This is a fail-closed safety property — the durable
/// watermark never outruns the rows it implies — and in a healthy run it
/// never fires, since the channel is lossless and a `store()` rejection means
/// a genuine backend error, not overload.
///
/// When a wallet faults, the task raises `sync_fault` (an `AtomicBool` the
/// host polls via `PlatformWalletManager::sync_fault_detected`) and logs a
/// one-shot `SYNC WATERMARK FROZEN` line via the `log` facade (which
/// android_logger forwards to logcat; `tracing` may not), so integrators can
/// show a hard "verification failed / rescan pending" state.
async fn run_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<P>,
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

    loop {
        // Block for the first event of a batch. Everything already sitting in
        // the channel behind it is folded in below without another await, so a
        // burst costs one `store()` per wallet instead of one per event (see
        // [`ADAPTER_STORE_BATCH_LIMIT`]).
        let first = tokio::select! {
            recv = receiver.recv() => recv,
            _ = cancel.cancelled() => break,
        };

        // `recv()` on an mpsc returns `None` only when every sender (the
        // manager) has been dropped — the lossless channel has no `Lagged`.
        let Some(event) = first else {
            if !cancel.is_cancelled() {
                tracing::error!("WalletEvent persistence channel closed unexpectedly");
            }
            break;
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
            let entry = batch.entry(wallet_id).or_default();
            entry.core.merge(core);
            entry.asset_locks.merge(asset_locks);
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
                    let entry = batch.entry(wallet_id).or_default();
                    entry.core.merge(core);
                    entry.asset_locks.merge(asset_locks);
                    folded += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }

        // Commit the folded batch. The channel is lossless, so the only way a
        // watermark is held back is a rejected `store()` (the fail-closed
        // backstop inside `commit_batch`).
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
        let batch_wallet_ids: Vec<WalletId> = batch.keys().copied().collect();
        // Filled by `commit_batch` as each wallet's `store()` returns. Lives
        // out here so a panicking commit thread cannot take it down with it:
        // what it holds is the difference between "this wallet's rows are
        // accounted for" and "nobody knows".
        let settled: Arc<Mutex<Vec<WalletId>>> = Arc::new(Mutex::new(Vec::new()));
        let settled_for_commit = Arc::clone(&settled);
        let persister_for_commit = Arc::clone(&persister);
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
            let mut freeze_logged = freeze_for_commit.load(Ordering::Relaxed);
            let mut settled = settled_for_commit
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let diag = commit_batch(
                &*persister_for_commit,
                batch,
                folded,
                &mut fault,
                &sync_fault_for_commit,
                &mut freeze_logged,
                &mut settled,
            );
            freeze_for_commit.store(freeze_logged, Ordering::Relaxed);
            diag
        })
        .await;

        let diag = match committed {
            Ok(diag) => diag,
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
///    guard and reintroduce dashpay/platform#4069).
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
    freeze_logged: &mut bool,
    settled: &mut Vec<WalletId>,
) -> BatchDiagnostics
where
    P: PlatformWalletPersistence + ?Sized,
{
    let mut diag = BatchDiagnostics::new(folded, batch.len());
    for (
        wallet_id,
        WalletBatch {
            mut core,
            asset_locks,
        },
    ) in batch
    {
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
        if core.is_empty_no_records() && Merge::is_empty(&asset_locks) {
            // SyncHeightAdvanced for an unknown wallet, empty BlockProcessed, a
            // watermark-only batch stripped by the fault guard above, etc. —
            // nothing to persist. Skip the round-trip.
            continue;
        }
        // The height this changeset OFFERS to the store. It is counted as
        // persisted only in the `Ok` arm below.
        let offered_height = core.synced_height;
        let cs = PlatformWalletChangeSet {
            core: Some(core),
            // Tracked-asset-lock rows reconstructed from this drain's
            // records (see `reconstruct_asset_locks_for_event`) ride the
            // same store round-trip so the row and the record that
            // implies it land atomically.
            asset_locks: (!Merge::is_empty(&asset_locks)).then_some(asset_locks),
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
            Ok(()) => {
                if let Some(h) = offered_height {
                    diag.record_persisted(h);
                }
            }
            Err(e) => {
                // A rejected changeset means these rows are not on disk. Fault
                // THIS wallet's watermark so it can't outrun them; the next
                // scan re-emits and the idempotent upserts recover the state.
                if let Some(h) = offered_height {
                    diag.record_rejected(h);
                }
                fault.fault_wallet(wallet_id, sync_fault);
                // Count each faulted wallet once per drain: a wallet that
                // entered already faulted was counted at the top of the loop,
                // and a repeat rejection must not count it again.
                if !is_faulted {
                    diag.faulted += 1;
                }
                // One-shot, unambiguous logcat marker via the `log` facade
                // (android_logger forwards `log` to logcat; `tracing` may not).
                if !*freeze_logged {
                    *freeze_logged = true;
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
    diag
}

/// Durable-watermark guard for dashpay/platform#4069.
///
/// When a wallet has faulted this session (a `store()` was rejected), its
/// persisted `synced_height` watermark must not advance past the last height
/// whose rows were fully committed — otherwise the wallet believes it is
/// scanned and never re-matches the blocks whose rows were lost. This
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
            // Derive UTXO deltas before moving the record into `records`
            // so the per-record borrows are still live.
            let (addresses_marked_used, account_highest_used) =
                collect_usage_deltas(wallet_manager, wallet_id, vec![&**record]).await;
            CoreChangeSet {
                new_utxos: derive_new_utxos(record),
                spent_utxos: derive_spent_utxos(record),
                // A contact's watch-only chain never defines the
                // wallet's transaction row (see `is_contact_watch_only`).
                // The usage deltas below are still emitted, so the
                // event is not dropped and the contact's address pool
                // still advances.
                records: if is_contact_watch_only(record) {
                    Vec::new()
                } else {
                    vec![(**record).clone()]
                },
                // Mirror the upstream-emitted derived addresses
                // through to the persister so newly-extended pool
                // rows are written transactionally with the tx that
                // triggered the extension. See
                // `CoreChangeSet.addresses_derived` for the cascade-
                // link rationale.
                addresses_derived: addresses_derived.clone(),
                addresses_marked_used,
                account_highest_used,
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
            cs.records.extend(
                inserted
                    .iter()
                    .chain(updated.iter())
                    .chain(matured.iter())
                    .filter(|r| !is_contact_watch_only(r))
                    .cloned(),
            );
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
            cs
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
            // boundary advance is no longer invisible to this bridge.
            // The earlier `TransactionsChainlocked`-only signal had a
            // gap on the "metadata advanced but per-account empty"
            // path; the new event closes it deterministically.
            CoreChangeSet {
                last_applied_chain_lock: Some(chain_lock.clone()),
                ..CoreChangeSet::default()
            }
        }
    }
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
/// our outpoints) and synthesizes a `Utxo` per entry using the data we
/// have: the outpoint from `transaction.input[index].previous_output`,
/// the value and address from `InputDetail`. The script_pubkey, height,
/// and confirmation flags belong to the *previous* transaction's
/// output and aren't carried in `InputDetail`; they're filled with
/// defaults (`ScriptBuf::default()`, height 0, all flags false). The
/// persister deletes by `outpoint` so the missing fields are
/// informational only — they never affect correctness of the spent-set
/// removal, only the audit-trail richness on the way out.
fn derive_spent_utxos(record: &TransactionRecord) -> Vec<Utxo> {
    record
        .input_details
        .iter()
        .filter_map(|detail| {
            let input = record.transaction.input.get(detail.index as usize)?;
            Some(Utxo {
                outpoint: input.previous_output,
                txout: TxOut {
                    value: detail.value,
                    script_pubkey: ScriptBuf::default(),
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

impl CoreChangeSet {
    /// Cheap "should we bother round-tripping the persister" check used
    /// by the adapter to drop empty events without locking. Skips the
    /// `is_empty()` walk over `instant_locks_for_non_final_records`
    /// since that map is rarely populated and `Vec::is_empty` short-
    /// circuits on the common case.
    fn is_empty_no_records(&self) -> bool {
        self.records.is_empty()
            && self.spent_utxos.is_empty()
            && self.new_utxos.is_empty()
            && self.instant_locks_for_non_final_records.is_empty()
            && self.last_processed_height.is_none()
            && self.synced_height.is_none()
            && self.last_applied_chain_lock.is_none()
            && self.addresses_derived.is_empty()
            && self.addresses_marked_used.is_empty()
            && self.account_highest_used.is_empty()
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
        TransactionRecord::new(
            tx.clone(),
            account_type,
            in_block(1_000),
            TransactionType::Standard,
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
}

#[cfg(test)]
mod tests {
    use super::freeze_synced_height_if_faulted;
    use crate::changeset::changeset::CoreChangeSet;

    /// dashpay/platform#4069: while persistence is healthy the sync
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

    /// dashpay/platform#4069: once persistence has faulted, the durable
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

    /// dashpay/platform#4069 (per-wallet fault scoping): a `store()`
    /// rejection freezes ONLY the named wallet; a sibling keeps advancing.
    /// (The old global `broadcast::Lagged` latch is gone — the lossless
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

    // ── Adapter-loop integration tests (dashpay/platform#4069) ──
    //
    // These drive `run_wallet_event_adapter` with a real lossless
    // `mpsc::UnboundedSender` and a probe persister so the LOOP — not just
    // the `freeze_synced_height_if_faulted` helper — is exercised: a large
    // lossless burst, a rejected `store()`, the per-wallet freeze, and
    // per-wallet batch folding.

    use super::{run_wallet_event_adapter, AdapterFaultState};
    use crate::changeset::changeset::PlatformWalletChangeSet;
    use crate::changeset::client_start_state::ClientStartState;
    use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence};
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
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
        /// backend that dies mid-write — the case that used to unwind the whole
        /// adapter task and now surfaces as a `JoinError`.
        panic_once: Mutex<HashSet<WalletId>>,
        /// Held closed to keep a `store()` call parked. The SQLite backend
        /// commits a real transaction per call, so a slow disk parks the caller
        /// for real; this makes that duration controllable.
        block_until: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        /// Raised as soon as a blocked `store()` is entered, so a test can wait
        /// for the block to be in effect rather than sleeping and hoping.
        blocked: Arc<AtomicBool>,
    }

    impl ProbePersister {
        fn new(obs: UnboundedSender<StoreObserved>) -> Self {
            Self {
                obs,
                fail_once: Mutex::new(HashSet::new()),
                panic_once: Mutex::new(HashSet::new()),
                block_until: Mutex::new(None),
                blocked: Arc::new(AtomicBool::new(false)),
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
    /// catch-up. On the old broadcast this burst would `Lagged` and freeze the
    /// watermark forever (dashpay/platform#4069).
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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

        let sentinel = obs_rx.recv().await.expect("sentinel store must arrive");
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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
    /// earlier rows may never have landed, which is exactly the hole
    /// dashpay/platform#4069 closed.
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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

    /// (i) A commit panic must not punish the wallets it did not reach.
    ///
    /// `commit_batch` walks the batch serially (a `BTreeMap`, so in wallet-id
    /// order). If an earlier wallet's `store()` returned and a later one
    /// panics, the earlier wallet's rows are on disk and its watermark is
    /// safe — freezing it would strip its `synced_height` for the rest of the
    /// session over a sibling's bad batch.
    ///
    /// Guards the fix for the first version of the panic handler, which
    /// faulted every wallet in the drain.
    #[tokio::test]
    async fn a_panicking_commit_spares_the_wallets_it_already_stored() {
        // `BTreeMap` order decides who is committed first, so the ids are
        // chosen to put the healthy wallet ahead of the panicking one.
        let healthy = [0x11u8; 32];
        let doomed = [0x22u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        persister.panic_next(doomed);
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::clone(&persister),
            rx,
            Arc::clone(&sync_fault),
            cancel.clone(),
        ));

        // Both wallets in one drain: `healthy` stores, then `doomed` panics.
        // Sent before either is observed so they fold into a single batch.
        tx.send(block_processed_event(healthy, 10)).unwrap();
        tx.send(block_processed_event(doomed, 10)).unwrap();

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
    /// the watermark past the guard and reintroduce dashpay/platform#4069
    /// (durable watermark outrunning the rows it implies).
    #[tokio::test]
    async fn watermark_is_still_stripped_after_a_fault() {
        let wallet_id = [9u8; 32];
        let (tx, rx) = unbounded_channel::<WalletEvent>();

        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = Arc::new(ProbePersister::new(obs_tx));
        // Fault the wallet via a rejected store (the only remaining trigger
        // now that the lossless channel can't lag).
        persister.fail_next(wallet_id);
        let sync_fault = Arc::new(AtomicBool::new(false));
        let cancel = CancellationToken::new();
        let handle = tokio::spawn(run_wallet_event_adapter(
            test_manager(),
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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
            Arc::clone(&persister),
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
        let mut freeze_logged = false;

        let mut batch = BTreeMap::new();
        batch.insert(
            wallet_id,
            super::WalletBatch {
                core: CoreChangeSet::default(),
                asset_locks,
            },
        );
        commit_batch(
            &persister,
            batch,
            1,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
            &mut Vec::new(),
        );

        let observed = obs_rx
            .try_recv()
            .expect("an asset-locks-only batch must not be skipped");
        assert_eq!(observed.n_asset_locks, 1);
        assert!(!observed.rejected);
    }

    // ── Batch-diagnostic reporting (dashpay/platform#4290 review) ──
    //
    // The per-drain `wallet-event batch: ...` line is read off a mainnet
    // tester's logcat to answer "is the durable watermark advancing?", so what
    // it reports is a tested property, not a comment.
    //
    // The regression these lock down: the diagnostic used to fold
    // `core.synced_height` into `synced_height_persisted` BEFORE calling
    // `persister.store(...)`. A rejected store therefore logged
    // `synced_height_persisted=Some(h)` in the very same drain that faulted the
    // wallet *because* height `h`'s rows were not accepted — an internally
    // contradictory trace that points a diagnosis at the wrong subsystem.
    //
    // These drive the real `commit_batch` (the production commit path,
    // including the fail-closed guard) so the assertions cover the shipped
    // code, not a restatement of it.

    use super::{commit_batch, AssetLockChangeSet, BatchDiagnostics, WalletBatch};

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
            },
        );
        batch
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
        let mut freeze_logged = false;

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(500, 500)),
            1,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
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

    /// REGRESSION (PR #4290 review): a REJECTED `store()` must never be
    /// reported as persisted.
    #[test]
    fn rejected_store_is_not_reported_as_persisted() {
        let wallet_id = [9u8; 32];
        let (obs_tx, mut obs_rx) = unbounded_channel();
        let persister = ProbePersister::new(obs_tx);
        persister.fail_next(wallet_id);
        let sync_fault = AtomicBool::new(false);
        let mut fault = AdapterFaultState::default();
        let mut freeze_logged = false;

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(500, 500)),
            1,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
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
            freeze_logged,
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
        let mut freeze_logged = true; // one-shot already spent

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(900, 900)),
            1,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
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
        let mut freeze_logged = true;

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
            &mut freeze_logged,
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
        let mut freeze_logged = false;

        let mut batch = BTreeMap::new();
        batch.extend(one_wallet_batch(healthy, watermark_with_rows(10, 10)));
        batch.extend(one_wallet_batch(rejecting, watermark_with_rows(20, 20)));

        let diag = commit_batch(
            &persister,
            batch,
            2,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
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
    /// must never exceed `wallets` (#4315 review finding 30c2e8e95003).
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
        let mut freeze_logged = true;

        let diag = commit_batch(
            &persister,
            one_wallet_batch(wallet_id, watermark_with_rows(700, 700)),
            1,
            &mut fault,
            &sync_fault,
            &mut freeze_logged,
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
}
