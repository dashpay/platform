//! Adapter that turns upstream `WalletEvent`s into `PlatformWalletChangeSet`s.
//!
//! Upstream `key_wallet_manager::WalletManager` exposes a
//! `broadcast::Sender<WalletEvent>` and a `subscribe_events()` accessor
//! returning a `broadcast::Receiver<WalletEvent>`; consumers attach at
//! startup and drain the stream. [`spawn_wallet_event_adapter`] is the
//! platform-wallet-side consumer: a tokio task that pulls events off
//! that broadcast, projects each one into a
//! [`CoreChangeSet`](crate::changeset::CoreChangeSet), wraps it in a
//! [`PlatformWalletChangeSet`](crate::changeset::PlatformWalletChangeSet),
//! and forwards to the [`PlatformWalletPersistence`] sink.
//!
//! # Why a single subscriber, not per-wallet
//!
//! The broadcast channel emits every event for every wallet. Each
//! event already carries a `wallet_id`, which the adapter forwards
//! verbatim to [`PlatformWalletPersistence::store`] — no need to fan
//! out a subscriber per wallet.
//!
//! # Lifetime
//!
//! [`spawn_wallet_event_adapter`] returns a [`JoinHandle`]. The caller
//! (typically `PlatformWalletManager`) keeps the handle for the
//! manager's lifetime; on shutdown, fire the [`CancellationToken`] to
//! make the task exit cleanly.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, AddressState};
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::transaction_router::AccountTypeToCheck;
use key_wallet::transaction_checking::{DerivedAddressInfo, TransactionContext};
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{CoreChangeSet, HighestUsedIndexes, PlatformWalletChangeSet};
use crate::changeset::merge::Merge;
use crate::changeset::traits::PlatformWalletPersistence;
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
/// what a historical SPV catch-up emits. The upstream producer publishes
/// fire-and-forget onto a bounded ring (`DEFAULT_WALLET_EVENT_CAPACITY`,
/// 1000), so a consumer that slow overflows it, `recv()` returns
/// `Lagged`, and the durable-watermark guard freezes the wallet's
/// `synced_height` for the rest of the process lifetime. In the field that
/// presented as a sync that climbs toward completion and then "rolls back"
/// to near the install position on every relaunch: the watermark froze
/// after the first batch, so every later restart resumed from that same
/// frozen height.
///
/// Folding every event *already buffered* in the ring into one changeset
/// per wallet collapses a burst of N events into a single store, so the
/// drain rate is bounded by the ring rather than by the persister. This is
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
/// Faults are scoped as narrowly as the triggering signal allows:
///
/// - A **`store()` rejection** carries a `wallet_id`, so only THAT
///   wallet's watermark freezes. A sibling wallet whose rows are still
///   landing atomically keeps advancing — freezing it too would force a
///   redundant rescan of a wallet that never lost a row (the CodeRabbit /
///   reviewer suggestion made concrete).
/// - A broadcast **`Lagged`** has no `wallet_id` — the dropped events
///   could have belonged to any wallet — so it freezes EVERY wallet,
///   including wallets first seen in a later event. That is modelled as a
///   single `global` latch rather than a per-wallet entry so future
///   wallet ids are covered without the adapter having to enumerate them.
#[derive(Default)]
struct AdapterFaultState {
    /// Set by a broadcast `Lagged`: freezes every wallet's watermark.
    global: bool,
    /// Set by a `store()` rejection: freezes only the named wallet.
    per_wallet: HashMap<WalletId, bool>,
}

impl AdapterFaultState {
    /// Whether `wallet_id`'s durable watermark must be held frozen.
    fn is_faulted(&self, wallet_id: &WalletId) -> bool {
        self.global || self.per_wallet.get(wallet_id).copied().unwrap_or(false)
    }

    /// Fault a single wallet after its `store()` was rejected, and raise
    /// the host-visible hard-fault signal.
    fn fault_wallet(&mut self, wallet_id: WalletId, hard_signal: &AtomicBool) {
        self.per_wallet.insert(wallet_id, true);
        hard_signal.store(true, Ordering::Relaxed);
    }

    /// Fault every wallet after a dropped-event broadcast lag, and raise
    /// the host-visible hard-fault signal.
    fn fault_all(&mut self, hard_signal: &AtomicBool) {
        self.global = true;
        hard_signal.store(true, Ordering::Relaxed);
    }
}

/// Spawn the wallet-event subscriber task.
///
/// The `receiver` MUST be subscribed by the caller *before the manager is
/// published to producers* (see [`run_wallet_event_adapter`] for why) and
/// then handed to this function. This function only moves it into the
/// spawned task; it does not subscribe. Exits when `cancel` fires or the
/// upstream broadcast channel closes.
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
    receiver: broadcast::Receiver<WalletEvent>,
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
/// (drive a real `broadcast::Sender`, inject a probe persister).
///
/// # Subscribe-before-publish
///
/// The caller subscribes and passes the `receiver` in, rather than the
/// task subscribing itself. A `tokio::broadcast::Receiver` only sees
/// messages sent *after* its `subscribe()` returns; a message sent before
/// the receiver exists is simply invisible to it — NOT reported as
/// `Lagged`. If subscription happened inside the spawned task (as it used
/// to), every event emitted between the manager being handed to producers
/// and the task's first poll would be silently lost with no fault signal.
/// Subscribing synchronously before the manager is published closes that
/// window.
///
/// # Durable-watermark guard (dashpay/platform#4069)
///
/// The upstream `WalletManager` publishes `WalletEvent`s onto a *bounded*
/// `tokio::broadcast` ring (capacity `DEFAULT_WALLET_EVENT_CAPACITY`,
/// 1000) via fire-and-forget `let _ = event_sender.send(..)`. During a
/// historical SPV catch-up the manager processes blocks far faster than
/// this single-threaded adapter can drain them through the (slow, JNI +
/// Room) persister, so the ring overflows and `recv()` returns
/// `RecvError::Lagged(n)` — the `n` dropped events are gone for good.
/// Those dropped events are exactly the `TransactionDetected` /
/// `BlockProcessed` records that carry the new UTXOs and the
/// spent-outpoint markers. Meanwhile the separate `SyncHeightAdvanced`
/// event (a bare height watermark, the ONLY event whose height reaches the
/// host persister's `syncedHeight` — see
/// `WalletChangeSetFFI::from_changeset`) keeps flowing and eventually
/// lands, advancing the persisted watermark past blocks whose rows never
/// made it to disk.
///
/// Symptoms (all one root cause): rows dropped entirely while the
/// watermark advances (fresh scan persists nothing yet reports "scanned");
/// spent-markers lost so consumed outputs rehydrate as spendable (inflated
/// balance); and — because the wallet's own `synced_height` is what gates
/// a rescan — the wallet believes it is fully scanned and never
/// re-matches, so the corruption is unrecoverable without deleting +
/// recreating the wallet.
///
/// Fix: once a wallet has faulted this session (a broadcast lag OR a
/// `store()` rejection), never advance ITS persisted sync watermark again
/// (see [`AdapterFaultState`] for the per-wallet vs. global scoping). We
/// strip `synced_height` from every subsequent changeset, freezing the
/// durable watermark at the last height whose rows were fully committed.
///
/// This is a **fail-closed mitigation, not lossless recovery.** On the
/// next process launch the wallet restores that (lower) watermark and the
/// SPV scan resumes from it, re-emitting the dropped `BlockProcessed`
/// records; the persister's upserts are idempotent on the outpoint key, so
/// re-applying them restores the missing rows and re-marks the lost
/// spends. But the capacity-1000 producer is still fire-and-forget: if the
/// replayed catch-up again outruns this consumer, the ring lags again and
/// the watermark re-freezes. A single restart therefore resumes from the
/// last durable watermark — it does NOT *guarantee* a one-restart
/// self-heal under repeated overload; repeated overload keeps freezing
/// until the throughput mismatch is resolved (or the host acts on the
/// hard-fault signal below). The guarantee it DOES make is the safety one:
/// the durable watermark never outruns the rows it implies, so funds are
/// never silently lost or inflated — the worst case is a repeated rescan.
///
/// When any wallet faults, the task also raises `sync_fault` (an
/// `AtomicBool` the host can poll via
/// `PlatformWalletManager::sync_fault_detected`) and logs at error level,
/// so integrators can show a hard "verification failed / rescan pending"
/// state instead of the failure being visible only in logs.
async fn run_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<P>,
    mut receiver: broadcast::Receiver<WalletEvent>,
    sync_fault: Arc<AtomicBool>,
    cancel: CancellationToken,
) where
    P: PlatformWalletPersistence + 'static,
{
    tracing::debug!("wallet-event adapter task started");
    let mut fault = AdapterFaultState::default();

    loop {
        // Block for the first event of a batch. Everything already sitting
        // in the ring behind it is folded in below without another await,
        // so a burst costs one `store()` per wallet instead of one per
        // event (see [`ADAPTER_STORE_BATCH_LIMIT`]).
        let first = tokio::select! {
            recv = receiver.recv() => recv,
            _ = cancel.cancelled() => break,
        };

        let mut batch: BTreeMap<WalletId, CoreChangeSet> = BTreeMap::new();
        let mut missed: u64 = 0;
        let mut closed = false;

        match first {
            Ok(event) => {
                let wallet_id = event.wallet_id();
                // For events that need to consult per-wallet state (today
                // only `TransactionInstantLocked`, which checks finality
                // before recording the IS lock), grab a brief read lock on
                // the manager.
                let core = build_core_changeset(&wallet_manager, &event).await;
                batch.entry(wallet_id).or_default().merge(core);
            }
            Err(RecvError::Lagged(n)) => missed += n,
            Err(RecvError::Closed) if cancel.is_cancelled() => break,
            Err(RecvError::Closed) => {
                tracing::error!("WalletEvent broadcast closed unexpectedly");
                break;
            }
        }

        // Fold in whatever else is already buffered. `try_recv` never
        // waits, so this drains the backlog at projection speed and stops
        // as soon as the ring is empty.
        let mut folded = 1usize;
        while folded < ADAPTER_STORE_BATCH_LIMIT {
            match receiver.try_recv() {
                Ok(event) => {
                    let wallet_id = event.wallet_id();
                    let core = build_core_changeset(&wallet_manager, &event).await;
                    batch.entry(wallet_id).or_default().merge(core);
                    folded += 1;
                }
                Err(TryRecvError::Lagged(n)) => {
                    missed += n;
                    folded += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Closed) => {
                    closed = true;
                    break;
                }
            }
        }

        if missed > 0 {
            // The dropped events carried record/UTXO/spend rows we will
            // never see again this session, and we don't know which
            // wallet(s) they belonged to. Fault EVERY wallet so no
            // watermark outruns its rows; the next scan re-emits the lost
            // blocks (dashpay/platform#4069).
            //
            // Faulting before storing this batch also covers the events
            // folded in *ahead* of the lag: stripping their `synced_height`
            // can only hold the watermark lower, never advance it past
            // uncommitted rows, so the conservative direction is the safe
            // one.
            fault.fault_all(&sync_fault);
            tracing::error!(
                missed,
                "wallet-event adapter lagged on broadcast channel; {missed} persistence events dropped — freezing every wallet's sync watermark so the next scan re-persists them (dashpay/platform#4069)"
            );
        }

        for (wallet_id, mut core) in batch {
            // Hold this wallet's durable watermark at the last fully
            // persisted height once it has faulted (see the guard doc
            // above). Records/UTXOs in this changeset are still persisted
            // — only the height advance is suppressed. Applied after the
            // fold so a `synced_height` that arrived via merge is stripped
            // too.
            freeze_synced_height_if_faulted(&mut core, fault.is_faulted(&wallet_id));
            if core.is_empty_no_records() {
                // SyncHeightAdvanced for an unknown wallet, empty
                // BlockProcessed, a watermark-only batch stripped by the
                // fault guard above, etc. — nothing to persist. Skip the
                // round-trip.
                continue;
            }
            let cs = PlatformWalletChangeSet {
                core: Some(core),
                ..PlatformWalletChangeSet::default()
            };
            if let Err(e) = persister.store(wallet_id, cs) {
                // A rejected changeset means these rows are not on disk.
                // Fault THIS wallet's watermark so it can't outrun them;
                // the next scan re-emits and the idempotent upserts
                // recover the state.
                fault.fault_wallet(wallet_id, &sync_fault);
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "Persister rejected core changeset; freezing this wallet's sync watermark so the next scan re-persists the missing rows (dashpay/platform#4069)"
                );
            }
        }

        if closed {
            if !cancel.is_cancelled() {
                tracing::error!("WalletEvent broadcast closed unexpectedly");
            }
            break;
        }
    }
    tracing::debug!("wallet-event adapter task exiting");
}

/// Durable-watermark guard for dashpay/platform#4069.
///
/// When a wallet has faulted this session (a broadcast lag dropped
/// record-bearing events, or a `store()` was rejected), its persisted
/// `synced_height` watermark must not advance past the last height whose
/// rows were fully committed — otherwise the wallet believes it is
/// scanned and never re-matches the blocks whose rows were lost. This
/// strips ONLY `synced_height`; every other field (records, UTXO
/// deltas, `last_processed_height`, chain-lock) is left intact so
/// in-flight rows still persist. Factored out as a pure function so the
/// invariant is unit-testable without the async broadcast plumbing.
fn freeze_synced_height_if_faulted(core: &mut CoreChangeSet, persistence_faulted: bool) {
    if persistence_faulted {
        core.synced_height = None;
    }
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
                records: vec![(**record).clone()],
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
            cs.records.extend(inserted.iter().cloned());
            cs.records.extend(updated.iter().cloned());
            cs.records.extend(matured.iter().cloned());
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

/// Derive the "ours" UTXOs created by a transaction's outputs.
///
/// Walks `record.output_details`, keeps entries with role `Received` or
/// `Change`, and reconstructs a full `Utxo` from the corresponding
/// `transaction.output[index]` plus the record's confirmation context.
fn derive_new_utxos(record: &TransactionRecord) -> Vec<Utxo> {
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
    /// rejection freezes ONLY the named wallet; a broadcast `Lagged`
    /// (no wallet id) freezes every wallet, including one first seen later.
    #[test]
    fn fault_state_scopes_store_rejection_per_wallet_and_lag_globally() {
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

        // A broadcast lag freezes everything, including a never-before-seen
        // wallet id.
        let mut fault2 = AdapterFaultState::default();
        let c = [0xCCu8; 32];
        fault2.fault_all(&signal);
        assert!(
            fault2.is_faulted(&c),
            "lag must freeze every wallet, even future ids"
        );
    }

    // ── Adapter-loop integration tests (dashpay/platform#4069) ──
    //
    // These drive `run_wallet_event_adapter` with a real `broadcast`
    // channel and a probe persister so the LOOP — not just the
    // `freeze_synced_height_if_faulted` helper — is exercised: actual
    // `RecvError::Lagged`, a rejected `store()`, the per-wallet freeze,
    // and subscribe-before-publish timing.

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
    use tokio::sync::broadcast;
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
        rejected: bool,
    }

    /// Test persister: records every `store()` (over an unbounded tokio
    /// channel so the async test can await progress without blocking the
    /// current-thread runtime) and can be told to reject the NEXT store for
    /// a given wallet exactly once.
    struct ProbePersister {
        obs: UnboundedSender<StoreObserved>,
        fail_once: Mutex<HashSet<WalletId>>,
    }

    impl ProbePersister {
        fn new(obs: UnboundedSender<StoreObserved>) -> Self {
            Self {
                obs,
                fail_once: Mutex::new(HashSet::new()),
            }
        }
        fn fail_next(&self, wallet_id: WalletId) {
            self.fail_once.lock().unwrap().insert(wallet_id);
        }
    }

    impl PlatformWalletPersistence for ProbePersister {
        fn store(
            &self,
            wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let core = changeset.core.as_ref();
            let rejected = self.fail_once.lock().unwrap().remove(&wallet_id);
            let _ = self.obs.send(StoreObserved {
                wallet_id,
                synced_height: core.and_then(|c| c.synced_height),
                last_processed_height: core.and_then(|c| c.last_processed_height),
                n_records: core.map(|c| c.records.len()).unwrap_or(0),
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

    /// (b) A real `RecvError::Lagged` flips the fault latch, and a
    /// subsequent watermark-only event is stripped (never delivered as a
    /// store), while a record-bearing event still persists.
    #[tokio::test]
    async fn lagged_broadcast_freezes_and_strips_subsequent_watermark() {
        let wallet_id = [7u8; 32];
        // Capacity 2, send 4 before the adapter drains → the receiver's
        // first recv() observes Lagged(2). Ring retains heights 3,4.
        let (tx, rx) = broadcast::channel::<WalletEvent>(2);
        for h in 1..=4u32 {
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

        // Post-lag watermark: must be stripped (never stored).
        tx.send(sync_height_event(wallet_id, 100)).unwrap();
        // Record-bearing sentinel: proves the loop advanced past every
        // watermark AND that records still persist after the freeze.
        tx.send(block_processed_event(wallet_id, 50)).unwrap();

        let observed = obs_rx.recv().await.expect("sentinel store must arrive");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            observed.synced_height, None,
            "watermark must be stripped after a broadcast lag"
        );
        assert_eq!(
            observed.last_processed_height,
            Some(50),
            "record-bearing field must persist after the freeze"
        );
        assert!(
            obs_rx.try_recv().is_err(),
            "no watermark-only store should have been delivered after the lag"
        );
        assert!(
            sync_fault.load(Ordering::Relaxed),
            "hard sync-fault signal must be raised on a lag"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }

    /// (c) A rejected `store()` faults the wallet, and the very next
    /// watermark-only event is stripped and dropped (not delivered).
    #[tokio::test]
    async fn rejected_store_freezes_wallet_and_strips_watermark() {
        let wallet_id = [9u8; 32];
        let (tx, rx) = broadcast::channel::<WalletEvent>(16);
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
        let (tx, rx) = broadcast::channel::<WalletEvent>(16);
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

    /// (e) Subscribe-before-publish: an event emitted BEFORE the adapter
    /// task is spawned (i.e. before it polls) is still delivered, because
    /// the receiver was subscribed up-front and handed to the task. This is
    /// the exact invariant the manager now relies on.
    #[tokio::test]
    async fn events_emitted_before_task_poll_are_received() {
        let wallet_id = [3u8; 32];
        let (tx, rx) = broadcast::channel::<WalletEvent>(16);
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
        let (tx, rx) = broadcast::channel::<WalletEvent>(64);
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
        let (tx, rx) = broadcast::channel::<WalletEvent>(64);
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

    /// (g) SAFETY INVARIANT under folding: once the fault latch is set, a
    /// batch that merges a record-bearing event together with a watermark
    /// event still persists the records but must NOT carry the merged
    /// `synced_height`.
    ///
    /// This is the property that makes batching safe. The freeze is
    /// applied after the fold, so a `synced_height` that entered the
    /// changeset via `Merge` is stripped just like a standalone one —
    /// otherwise folding would smuggle the watermark past the guard and
    /// reintroduce dashpay/platform#4069 (durable watermark outrunning the
    /// rows it implies).
    #[tokio::test]
    async fn merged_watermark_is_still_stripped_after_a_fault() {
        let wallet_id = [9u8; 32];
        // Capacity 2 with 4 sends → the first recv() reports Lagged(2),
        // which latches the global fault before anything is stored.
        let (tx, rx) = broadcast::channel::<WalletEvent>(2);
        for h in 1..=4u32 {
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

        // Wait until the lag has been observed and latched, so the events
        // below are guaranteed to be evaluated under the fault.
        while !sync_fault.load(Ordering::Relaxed) {
            tokio::task::yield_now().await;
        }

        // A record-bearing event and a watermark event that will fold
        // into ONE changeset for this wallet.
        tx.send(block_processed_event(wallet_id, 60)).unwrap();
        tx.send(sync_height_event(wallet_id, 900)).unwrap();

        let observed = obs_rx
            .recv()
            .await
            .expect("the record-bearing half of the batch must still persist");
        assert_eq!(observed.wallet_id, wallet_id);
        assert_eq!(
            observed.synced_height, None,
            "a merged watermark must still be stripped while the wallet is faulted"
        );
        assert_eq!(
            observed.last_processed_height,
            Some(60),
            "record-bearing fields must survive the freeze"
        );

        cancel.cancel();
        drop(tx);
        handle.await.unwrap();
    }
}
