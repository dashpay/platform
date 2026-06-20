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

use std::sync::Arc;

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{CoreChangeSet, PlatformWalletChangeSet};
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
                            // DashPay payment hooks for every transaction
                            // record this event carries: record incoming
                            // payments (outputs paying a DashpayReceivingFunds
                            // address) and advance a matching sent payment
                            // `Pending → Confirmed` once its transaction
                            // confirms. Runs after the core store so the
                            // tx/UTXO rows land first.
                            if carries_payment_records(&event) {
                                let wallet_persister =
                                    crate::wallet::persister::WalletPersister::new(
                                        wallet_id,
                                        Arc::clone(&persister)
                                            as Arc<dyn PlatformWalletPersistence>,
                                    );
                                run_dashpay_payment_hooks(
                                    &wallet_manager,
                                    &wallet_id,
                                    &wallet_persister,
                                    &event,
                                )
                                .await;
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

/// Transaction records carried by `event` that should drive the DashPay
/// payment hooks (live incoming-record recording + sent-payment confirm).
///
/// [`WalletEvent::TransactionDetected`] is the first off-chain sighting of
/// a transaction — mempool, or a direct InstantSend lock — so its
/// `record.context` is not yet block-confirmed.
/// [`WalletEvent::BlockProcessed`] carries the records a block changed:
/// `inserted` (first stored in this block) and `updated`
/// (previously-known records that this block confirmed). A wallet sees its
/// *own* broadcast in the mempool first, so that transaction reaches a
/// confirmed context only via `BlockProcessed.updated` — routing solely
/// `TransactionDetected` is the gap that left sent payments stuck
/// `Pending`: the confirm hook early-returns on the unconfirmed mempool
/// sighting and never sees the confirming block. `matured` is
/// coinbase-maturity only — never a DashPay payment — so it is excluded.
fn dashpay_payment_records(event: &WalletEvent) -> Vec<&TransactionRecord> {
    match event {
        WalletEvent::TransactionDetected { record, .. } => vec![record.as_ref()],
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => inserted.iter().chain(updated.iter()).collect(),
        _ => Vec::new(),
    }
}

/// Cheap predicate so the adapter skips constructing a `WalletPersister`
/// for events that carry no transaction records.
fn carries_payment_records(event: &WalletEvent) -> bool {
    !dashpay_payment_records(event).is_empty()
}

/// Run the DashPay payment hooks for every transaction record carried by
/// `event`: record any incoming DashPay payment, then advance a matching
/// sent payment from `Pending` to `Confirmed` once its transaction
/// confirms. Both hooks are idempotent per txid, so re-detections and
/// repeated block-processing rounds converge without duplicating entries.
pub(crate) async fn run_dashpay_payment_hooks(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    event: &WalletEvent,
) {
    for record in dashpay_payment_records(event) {
        crate::wallet::identity::network::record_incoming_dashpay_payments(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
        crate::wallet::identity::network::confirm_sent_dashpay_payment(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
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
            record,
            addresses_derived,
            ..
        } => {
            // Derive UTXO deltas before moving the record into `records`
            // so the per-record borrows are still live.
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
            // `last_applied_chain_lock` advances (dashpay/rust-dashcore#769),
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

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::transaction::Transaction;
    use dashcore::TxIn;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};
    use key_wallet::WalletCoreBalance;

    /// A `TransactionRecord` whose txid is uniquely seeded by `seed` (via a
    /// distinct input outpoint). Context is irrelevant to the routing under
    /// test, so it stays `Mempool`.
    fn record(seed: u8) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(dashcore::Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn block_processed(
        inserted: Vec<TransactionRecord>,
        updated: Vec<TransactionRecord>,
        matured: Vec<TransactionRecord>,
    ) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id: [0u8; 32],
            height: 1,
            chain_lock: None,
            inserted,
            updated,
            matured,
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    /// `BlockProcessed` is the path by which a wallet's own broadcast
    /// confirms (`updated`), and the path by which a payment first seen in a
    /// block lands (`inserted`); both must drive the DashPay payment hooks.
    /// `matured` is coinbase-maturity only and carries no DashPay payment, so
    /// it is excluded. A regression that re-narrows routing to
    /// `TransactionDetected` — the original sent-payment-stuck-`Pending` bug —
    /// drops the `updated` record and fails this test.
    #[test]
    fn dashpay_payment_records_covers_block_processed_inserted_and_updated() {
        let event = block_processed(vec![record(0x01)], vec![record(0x02)], vec![record(0x03)]);
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert!(
            txids.contains(&record(0x01).txid),
            "inserted record must drive the payment hooks"
        );
        assert!(
            txids.contains(&record(0x02).txid),
            "updated (just-confirmed) record must drive the payment hooks — \
             this is how a sent payment flips Pending → Confirmed"
        );
        assert!(
            !txids.contains(&record(0x03).txid),
            "matured coinbase is not a DashPay payment and must be excluded"
        );
        assert_eq!(txids.len(), 2, "exactly inserted ∪ updated");
    }

    /// The first mempool sighting still routes its single record (incoming
    /// recording + the early-returning confirm probe).
    #[test]
    fn dashpay_payment_records_covers_transaction_detected() {
        let event = WalletEvent::TransactionDetected {
            wallet_id: [0u8; 32],
            record: Box::new(record(0x07)),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert_eq!(txids, vec![record(0x07).txid]);
    }

    /// Events with no transaction records contribute nothing.
    #[test]
    fn dashpay_payment_records_empty_for_non_record_events() {
        let event = WalletEvent::SyncHeightAdvanced {
            wallet_id: [0u8; 32],
            height: 42,
        };
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!carries_payment_records(&event));
    }
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
    }
}
