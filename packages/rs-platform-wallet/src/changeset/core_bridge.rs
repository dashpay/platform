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

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::transaction_router::AccountTypeToCheck;
use key_wallet::transaction_checking::{DerivedAddressInfo, TransactionContext};
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{CoreChangeSet, HighestUsedIndexes, PlatformWalletChangeSet};
use crate::changeset::traits::PlatformWalletPersistence;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Spawn the wallet-event subscriber task.
///
/// Subscribes to `wallet_manager.subscribe_events()` from inside the
/// spawned task (so the call-site doesn't need to be on a tokio
/// runtime), then loops dispatching events to the persister via
/// [`PlatformWalletPersistence::store`]. Exits when `cancel` fires
/// or the upstream broadcast channel closes.
///
/// Generic over `P` so the spawned task gets static-dispatch on
/// every `persister.store(...)` call. Pass the manager's own
/// `Arc<P>` (not the `Arc<dyn PlatformWalletPersistence>`
/// coercion) to actually realize the static-dispatch win.
pub fn spawn_wallet_event_adapter<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<P>,
    cancel: CancellationToken,
) -> JoinHandle<()>
where
    P: PlatformWalletPersistence + 'static,
{
    tokio::spawn(async move {
        let mut receiver = {
            let guard = wallet_manager.read().await;
            guard.subscribe_events()
        };
        tracing::debug!("wallet-event adapter task started");

        loop {
            tokio::select! {
                recv = receiver.recv() => {
                    match recv {
                        Ok(event) => {
                            let wallet_id = event.wallet_id();
                            // For events that need to consult per-wallet
                            // state (today only `TransactionInstantLocked`,
                            // which checks finality before recording the IS
                            // lock), grab a brief read lock on the manager.
                            let core = build_core_changeset(&wallet_manager, &event).await;
                            if core.is_empty_no_records() {
                                // SyncHeightAdvanced for an unknown wallet,
                                // empty BlockProcessed, etc. — nothing to
                                // persist. Skip the round-trip.
                                continue;
                            }
                            let cs = PlatformWalletChangeSet {
                                core: Some(core),
                                ..PlatformWalletChangeSet::default()
                            };
                            if let Err(e) = persister.store(wallet_id, cs) {
                                tracing::warn!(
                                    wallet_id = %hex::encode(wallet_id),
                                    error = %e,
                                    "Persister rejected core changeset; state will be re-emitted on next sync round"
                                );
                            }
                        }
                        Err(RecvError::Closed) if cancel.is_cancelled() => break,
                        Err(RecvError::Closed) => {
                            tracing::error!("WalletEvent broadcast closed unexpectedly");
                            break;
                        }
                        Err(RecvError::Lagged(n)) => {
                            tracing::warn!(
                                missed = n,
                                "wallet-event adapter lagged on broadcast channel; some events were dropped"
                            );
                        }
                    }
                }
                _ = cancel.cancelled() => break,
            }
        }
        tracing::debug!("wallet-event adapter task exiting");
    })
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
/// For each record this re-runs the same **read-only** matcher the
/// wallet used during processing
/// ([`ManagedAccountCollection::check_transaction`]), scoped to the
/// record's own account type, then resolves every involved address
/// back to its owning pool for the authoritative post-mark
/// [`AddressInfo`](key_wallet::managed_account::address_pool::AddressInfo)
/// and the pool's `highest_used` watermark. `used` is forced `true`
/// on the emitted entry — involvement in a recorded transaction is
/// the definition of "used", independent of snapshot timing.
///
/// Returns empty deltas when the wallet is unknown (raced a removal)
/// — the next sync round re-emits.
async fn collect_usage_deltas(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    records: Vec<&TransactionRecord>,
) -> (
    Vec<DerivedAddressInfo>,
    BTreeMap<AccountType, HighestUsedIndexes>,
) {
    let mut marked_used: Vec<DerivedAddressInfo> = Vec::new();
    let mut highest_used: BTreeMap<AccountType, HighestUsedIndexes> = BTreeMap::new();
    if records.is_empty() {
        return (marked_used, highest_used);
    }

    let guard = wallet_manager.read().await;
    let Some(info) = guard.get_wallet_info(wallet_id) else {
        return (marked_used, highest_used);
    };
    let accounts = &info.core_wallet.accounts;
    let account_refs = accounts.all_accounts();

    let mut seen: HashSet<(AccountType, AddressPoolType, u32)> = HashSet::new();
    // A block can carry several records for the same account type
    // (and `check_transaction` is per-tx anyway), so dedup the
    // (txid, type-to-check) pairs to avoid re-matching the same
    // transaction against the same accounts. `AccountTypeToCheck`
    // isn't `Hash` upstream; a linear scan over this per-batch-sized
    // vec is cheaper than hashing anyway.
    let mut checked: Vec<(dashcore::Txid, AccountTypeToCheck)> = Vec::new();

    for record in records {
        // The matcher wants the runtime `AccountTypeToCheck`
        // discriminant; resolve it through the live managed account so
        // the conversion stays upstream-owned. A record whose account
        // vanished from the collection (removed mid-flight) is skipped
        // — nothing to flip in a pool that no longer exists.
        let Some(account) = account_refs
            .iter()
            .find(|a| a.managed_account_type().to_account_type() == record.account_type)
        else {
            continue;
        };
        let Ok(type_to_check) = AccountTypeToCheck::try_from(account.managed_account_type()) else {
            continue;
        };
        if checked.contains(&(record.txid, type_to_check)) {
            continue;
        }
        checked.push((record.txid, type_to_check));

        let result = accounts.check_transaction(&record.transaction, &[type_to_check]);
        for account_match in result.affected_accounts {
            for involved in account_match.account_type_match.all_involved_addresses() {
                // Resolve the involved address back to its owning
                // account + pool. This recovers the `pool_type` (which
                // the match doesn't carry) and the authoritative
                // post-mark `AddressInfo`; any account monitoring the
                // address is a genuine usage site, matching upstream's
                // per-matched-account `mark_address_used` sweep.
                for owner in &account_refs {
                    let owner_type = owner.managed_account_type().to_account_type();
                    for pool in owner.managed_account_type().address_pools() {
                        let Some(pool_info) = pool.address_info(&involved.address) else {
                            continue;
                        };
                        if seen.insert((owner_type, pool.pool_type, pool_info.index)) {
                            let mut info = pool_info.clone();
                            info.used = true;
                            marked_used.push(DerivedAddressInfo {
                                account_type: owner_type,
                                pool_type: pool.pool_type,
                                info,
                            });
                        }
                        // Snapshot the owning account's highest-used
                        // watermarks. Standard accounts map External /
                        // Internal onto the two persisted slots;
                        // single-pool accounts (provider keys, identity
                        // funding) surface theirs as `external`.
                        let slot = highest_used.entry(owner_type).or_default();
                        let mut snapshot = HighestUsedIndexes::default();
                        for p in owner.managed_account_type().address_pools() {
                            if p.is_internal() {
                                snapshot.internal = p.highest_used;
                            } else {
                                snapshot.external = match (snapshot.external, p.highest_used) {
                                    (Some(a), Some(b)) => Some(a.max(b)),
                                    (a, b) => a.or(b),
                                };
                            }
                        }
                        slot.merge_max(snapshot);
                    }
                }
            }
        }
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
