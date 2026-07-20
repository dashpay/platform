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
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, AddressState};
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
        assert!(entry.info.is_used());

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
        assert!(entry.info.is_used());
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
