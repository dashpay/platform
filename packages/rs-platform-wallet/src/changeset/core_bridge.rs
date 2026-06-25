//! Adapter that turns upstream `WalletEvent`s into `PlatformWalletChangeSet`s.
//!
//! Upstream `key_wallet_manager::WalletManager` exposes a
//! `broadcast::Sender<WalletEvent>` and a `subscribe_events()` accessor
//! returning a `broadcast::Receiver<WalletEvent>`; consumers attach at
//! startup and drain the stream. [`wallet_event_adapter_loop`] is the
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
//! [`wallet_event_adapter_loop`] is the task body. The caller (typically
//! `PlatformWalletManager`) registers it on the shared `ThreadRegistry`
//! via `start_task`, which owns its [`JoinHandle`](tokio::task::JoinHandle)
//! and cancellation; on shutdown the registry fires the
//! [`CancellationToken`] to make the task exit cleanly and joins it.

use std::sync::Arc;

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{CoreChangeSet, PlatformWalletChangeSet};
use crate::changeset::traits::PlatformWalletPersistence;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// The wallet-event subscriber loop (the task body owned by the registry).
///
/// Subscribes to `wallet_manager.subscribe_events()` from inside the task
/// (so the call-site doesn't need to be on a tokio runtime), then loops
/// dispatching events to the persister via
/// [`PlatformWalletPersistence::store`]. Exits when `cancel` fires or the
/// upstream broadcast channel closes.
///
/// Generic over `P` so the task gets static-dispatch on every
/// `persister.store(...)` call. Pass the manager's own `Arc<P>` (not the
/// `Arc<dyn PlatformWalletPersistence>` coercion) to realize that win.
pub async fn wallet_event_adapter_loop<P>(
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<P>,
    cancel: CancellationToken,
) where
    P: PlatformWalletPersistence + 'static,
{
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
                        dispatch_event(&wallet_manager, persister.as_ref(), event).await;
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
            _ = cancel.cancelled() => {
                // Drain anything already queued in the receiver before
                // exit. Without this, events that the broadcast had
                // already delivered (but the select hadn't yet polled)
                // are dropped on cancellation — losing persistence work
                // the upstream already committed to. Same dispatch /
                // error handling as the live arm.
                drain_pending_events(&mut receiver, &wallet_manager, persister.as_ref()).await;
                break;
            }
        }
    }
    tracing::debug!("wallet-event adapter task exiting");
}

/// Drain every event already buffered in `receiver` synchronously,
/// dispatching each via [`dispatch_event`]. Used by the cancellation
/// arm of the adapter loop so events the broadcast delivered before
/// teardown are not dropped on exit. Lagged batches are logged and
/// skipped (matching the live-loop policy); a closed channel ends
/// the drain.
async fn drain_pending_events<P>(
    receiver: &mut Receiver<WalletEvent>,
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: &P,
) where
    P: PlatformWalletPersistence + 'static,
{
    loop {
        match receiver.try_recv() {
            Ok(event) => dispatch_event(wallet_manager, persister, event).await,
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
            Err(TryRecvError::Lagged(n)) => {
                tracing::warn!(
                    missed = n,
                    "wallet-event adapter lagged on broadcast channel during cancellation drain; some events were dropped"
                );
            }
        }
    }
}

/// Project a single [`WalletEvent`] into its [`CoreChangeSet`] and
/// forward to the persister. Extracted so the live-recv path and the
/// cancellation-drain path apply identical handling.
async fn dispatch_event<P>(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: &P,
    event: WalletEvent,
) where
    P: PlatformWalletPersistence + 'static,
{
    let wallet_id = event.wallet_id();
    // For events that need to consult per-wallet state (today only
    // `TransactionInstantLocked`, which checks finality before recording
    // the IS lock), grab a brief read lock on the manager.
    let core = build_core_changeset(wallet_manager, &event).await;
    if core.is_empty_no_records() {
        // SyncHeightAdvanced for an unknown wallet, empty
        // BlockProcessed, etc. — nothing to persist. Skip the
        // round-trip.
        return;
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
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AO};
    use tokio::sync::broadcast;

    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletChangeSet};

    struct CountingPersister(AtomicUsize);

    impl PlatformWalletPersistence for CountingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.0.fetch_add(1, AO::SeqCst);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// `drain_pending_events` must dispatch every event already queued
    /// in the receiver before returning. Guards against the
    /// cancellation-arm bug where events the broadcast had delivered but
    /// the `select!` hadn't yet polled were silently dropped at exit.
    #[tokio::test]
    async fn drain_pending_events_persists_queued_events() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(dashcore::Network::Testnet)));
        let persister = CountingPersister(AtomicUsize::new(0));

        let (tx, mut rx) = broadcast::channel::<WalletEvent>(16);
        let wallet_id: WalletId = [1u8; 32];
        // Queue three SyncHeightAdvanced events without polling rx;
        // each maps to a non-empty CoreChangeSet (synced_height = Some).
        for h in 100..103 {
            tx.send(WalletEvent::SyncHeightAdvanced {
                wallet_id,
                height: h,
            })
            .unwrap();
        }

        drain_pending_events(&mut rx, &wallet_manager, &persister).await;
        assert_eq!(
            persister.0.load(AO::SeqCst),
            3,
            "every queued event must reach the persister before the drain returns"
        );
    }

    /// Sanity check: an empty receiver returns immediately, no stores.
    #[tokio::test]
    async fn drain_pending_events_is_noop_on_empty_receiver() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(dashcore::Network::Testnet)));
        let persister = CountingPersister(AtomicUsize::new(0));
        let (_tx, mut rx) = broadcast::channel::<WalletEvent>(4);
        drain_pending_events(&mut rx, &wallet_manager, &persister).await;
        assert_eq!(persister.0.load(AO::SeqCst), 0);
    }

    /// End-to-end: events queued in the broadcast receiver at the moment
    /// `cancel` fires must be dispatched before the adapter loop exits.
    /// Cancels first, then pushes events through the WalletManager's
    /// broadcast sender — the loop's `select!` is already biased toward
    /// the cancel arm by then, so without the drain path every event
    /// here would be silently dropped on exit.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_loop_drains_queued_events_on_cancel() {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(dashcore::Network::Testnet)));
        let persister = Arc::new(CountingPersister(AtomicUsize::new(0)));
        let cancel = CancellationToken::new();

        let wm_for_task = Arc::clone(&wallet_manager);
        let persister_for_task = Arc::clone(&persister);
        let cancel_for_task = cancel.clone();
        let handle = tokio::spawn(async move {
            wallet_event_adapter_loop(wm_for_task, persister_for_task, cancel_for_task).await;
        });

        // Wait for the loop to subscribe (it does so before the first
        // recv()). A short poll is enough — the subscribe is sync inside
        // the task.
        for _ in 0..50 {
            if wallet_manager.read().await.event_sender().receiver_count() > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(
            wallet_manager.read().await.event_sender().receiver_count() > 0,
            "adapter loop should have subscribed by now"
        );

        // Cancel BEFORE sending. The next time the adapter polls, the
        // cancel arm wins (the events end up sitting in the broadcast
        // queue), so the drain path is what carries them through. The
        // sends happen synchronously into the broadcast buffer.
        cancel.cancel();
        let sender = wallet_manager.read().await.event_sender().clone();
        let wallet_id: WalletId = [7u8; 32];
        for h in 200..205 {
            sender
                .send(WalletEvent::SyncHeightAdvanced {
                    wallet_id,
                    height: h,
                })
                .unwrap();
        }

        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("adapter must exit promptly on cancel");

        assert_eq!(
            persister.0.load(AO::SeqCst),
            5,
            "drain path must dispatch every queued event before loop exit"
        );
    }
}
