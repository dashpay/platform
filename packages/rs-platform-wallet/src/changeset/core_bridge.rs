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

        // Durable-watermark guard (dashpay/platform#4069).
        //
        // The upstream `WalletManager` publishes `WalletEvent`s onto a
        // *bounded* `tokio::broadcast` ring (capacity
        // `DEFAULT_WALLET_EVENT_CAPACITY`, 1000) via fire-and-forget
        // `let _ = event_sender.send(..)`. During a historical SPV
        // catch-up the manager processes blocks far faster than this
        // single-threaded adapter can drain them through the (slow,
        // JNI + Room) persister, so the ring overflows and `recv()`
        // returns `RecvError::Lagged(n)` — the `n` dropped events are
        // gone for good. Those dropped events are exactly the
        // `TransactionDetected` / `BlockProcessed` records that carry
        // the new UTXOs and the spent-outpoint markers. Meanwhile the
        // separate `SyncHeightAdvanced` event (a bare height watermark,
        // the ONLY event whose height reaches the host persister's
        // `syncedHeight` — see `WalletChangeSetFFI::from_changeset`)
        // keeps flowing and eventually lands, advancing the persisted
        // watermark past blocks whose rows never made it to disk.
        //
        // Symptoms (all one root cause): rows dropped entirely while the
        // watermark advances (fresh scan persists nothing yet reports
        // "scanned"); spent-markers lost so consumed outputs rehydrate
        // as spendable (inflated balance); and — because the wallet's
        // own `synced_height` is what gates a rescan — the wallet
        // believes it is fully scanned and never re-matches, so the
        // corruption is unrecoverable without deleting + recreating the
        // wallet.
        //
        // Fix: once persistence has faulted this session (a broadcast
        // lag OR a `store()` rejection), never advance the persisted
        // sync watermark again. We strip `synced_height` from every
        // subsequent changeset, freezing the durable watermark at the
        // last height whose rows were fully committed. On the next
        // process launch the wallet restores that (lower) watermark and
        // the SPV scan resumes from it, re-emitting the dropped
        // `BlockProcessed` records; the persister's upserts are
        // idempotent on the outpoint key, so re-applying them restores
        // the missing rows AND re-marks the lost spends — turning the
        // integrator's current manual "clear + recreate the wallet"
        // recovery into automatic self-healing across a restart. This is
        // deliberately conservative (freeze for the rest of the session
        // after the first fault); the worst case is one extra rescan on
        // the next launch, never lost or inflated funds.
        let mut persistence_faulted = false;

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
                            let mut core = build_core_changeset(&wallet_manager, &event).await;
                            // Hold the durable watermark at the last
                            // fully-persisted height once persistence has
                            // faulted (see the guard doc above). Records/UTXOs
                            // in this changeset are still persisted — only the
                            // height advance is suppressed.
                            freeze_synced_height_if_faulted(&mut core, persistence_faulted);
                            if core.is_empty_no_records() {
                                // SyncHeightAdvanced for an unknown wallet,
                                // empty BlockProcessed, a watermark-only event
                                // stripped by the fault guard above, etc. —
                                // nothing to persist. Skip the round-trip.
                                continue;
                            }
                            let cs = PlatformWalletChangeSet {
                                core: Some(core),
                                ..PlatformWalletChangeSet::default()
                            };
                            if let Err(e) = persister.store(wallet_id, cs) {
                                // A rejected changeset means these rows are not
                                // on disk. Fault the watermark so it can't
                                // outrun them; the next scan re-emits and the
                                // idempotent upserts recover the state.
                                persistence_faulted = true;
                                tracing::error!(
                                    wallet_id = %hex::encode(wallet_id),
                                    error = %e,
                                    "Persister rejected core changeset; freezing sync watermark so the next scan re-persists the missing rows (dashpay/platform#4069)"
                                );
                            }
                        }
                        Err(RecvError::Closed) if cancel.is_cancelled() => break,
                        Err(RecvError::Closed) => {
                            tracing::error!("WalletEvent broadcast closed unexpectedly");
                            break;
                        }
                        Err(RecvError::Lagged(n)) => {
                            // The `n` dropped events carried record/UTXO/spend
                            // rows we will never see again this session. Fault
                            // the watermark so it stays at the last
                            // fully-persisted height and the next scan
                            // re-emits the lost blocks (dashpay/platform#4069).
                            persistence_faulted = true;
                            tracing::error!(
                                missed = n,
                                "wallet-event adapter lagged on broadcast channel; {n} persistence events dropped — freezing sync watermark so the next scan re-persists them (dashpay/platform#4069)"
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

/// Durable-watermark guard for dashpay/platform#4069.
///
/// When the persister has faulted this session (a broadcast lag dropped
/// record-bearing events, or a `store()` was rejected), the persisted
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
}
