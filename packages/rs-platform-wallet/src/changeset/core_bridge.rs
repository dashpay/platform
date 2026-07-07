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
use crate::changeset::Merge;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Bound on the on-cancel drain — caps how many buffered events the
/// adapter persists after cancellation before exiting. Sized well above
/// the upstream broadcast capacity (currently 256) so a normal teardown
/// drains everything, while a pathological flood can't stall shutdown.
const CANCEL_DRAIN_BUDGET: usize = 4096;

/// Single-account observation. The storage writer hardcodes
/// `core_utxos.account_index = 0` (the product uses only the default
/// account, and that column drives only cosmetic per-account grouping). A
/// UTXO-bearing record owned by a non-default funds account is STILL
/// persisted under index 0 — never skipped, because skipping it would
/// undercount the wallet balance and lose funds. We only `warn!` so the
/// approximate grouping is visible. Identity/provider account types carry
/// no funds index (`AccountType::index() == None`) and never emit
/// `Received`/`Change` UTXOs, so they never warn.
fn warn_if_non_default_account(record: &TransactionRecord) {
    if let Some(index) = record.account_type.index() {
        if index != 0 {
            tracing::warn!(
                account_index = index,
                txid = %record.txid,
                "non-default account UTXO persisted under account_index 0; \
                 per-account grouping is approximate"
            );
        }
    }
}

/// The wallet-event subscriber loop (the task body owned by the registry).
///
/// Subscribes to `wallet_manager.subscribe_events()` from inside the task
/// (so the call-site doesn't need to be on a tokio runtime), then loops
/// dispatching events to the persister via
/// [`PlatformWalletPersistence::store`]. Exits when `cancel` fires or the
/// upstream broadcast channel closes.
///
/// On cancellation the adapter drains any events already buffered on the
/// receiver before exiting (bounded by [`CANCEL_DRAIN_BUDGET`]). Without
/// that drain, a `TransactionInstantLocked` (which P2P does not replay)
/// emitted just before stop would be lost — the next sync subscribes
/// fresh and only sees future events. Other event kinds (block-driven)
/// are re-emitted on the next SPV resync from `last_processed_height`,
/// but the drain treats them uniformly.
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
                        process_event(&wallet_manager, persister.as_ref(), event).await;
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
                // Drain buffered events before exiting: P2P does not
                // replay IS-locks, so an event emitted just before
                // cancel would be silently lost without this drain.
                let drained = drain_buffered_events(
                    &mut receiver,
                    &wallet_manager,
                    persister.as_ref(),
                    CANCEL_DRAIN_BUDGET,
                )
                .await;
                if drained > 0 {
                    tracing::debug!(
                        drained,
                        "wallet-event adapter drained buffered events on cancel",
                    );
                }
                break;
            }
        }
    }
    tracing::debug!("wallet-event adapter task exiting");
}

/// Project a single event into the persister. Shared by the live loop
/// and the on-cancel drain so they cannot drift in behaviour.
async fn process_event<P>(
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
    if core.is_empty() {
        // SyncHeightAdvanced for an unknown wallet, empty BlockProcessed,
        // etc. — nothing to persist. Skip the round-trip.
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

/// Drain at most `budget` buffered events from `receiver` through
/// [`process_event`]. Returns the count drained. Stops on empty, closed
/// channel, or budget exhaustion; logs a warning on `Lagged`.
//
// TODO: add a unit test covering this path — synthesise WalletEvents on
// the broadcast, fire `cancel`, assert the drained count. Blocked on
// the absence of an existing test scaffold for `wallet_event_adapter_loop`.
async fn drain_buffered_events<P>(
    receiver: &mut Receiver<WalletEvent>,
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: &P,
    budget: usize,
) -> usize
where
    P: PlatformWalletPersistence + 'static,
{
    let mut drained = 0;
    let mut attempts = 0;
    // `attempts` (not `drained`) bounds the loop so a sustained `Lagged`
    // stream — which logs but produces no persisted event — still hits
    // the cap and exits, preserving the bounded-teardown guarantee.
    while attempts < budget {
        attempts += 1;
        match receiver.try_recv() {
            Ok(event) => {
                process_event(wallet_manager, persister, event).await;
                drained += 1;
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
            Err(TryRecvError::Lagged(n)) => {
                tracing::warn!(
                    missed = n,
                    "wallet-event adapter lagged during cancel drain; some events were dropped"
                );
            }
        }
    }
    if attempts == budget {
        tracing::warn!(
            budget,
            drained,
            "wallet-event adapter cancel-drain hit budget; further buffered events dropped"
        );
    }
    drained
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
            // Persist regardless of account; warn on a non-default account.
            warn_if_non_default_account(record);
            // Derive UTXO deltas before moving the record into `records`
            // so the per-record borrows are still live.
            CoreChangeSet {
                new_utxos: derive_new_utxos(record),
                spent_utxos: derive_spent_utxos(record),
                records: vec![(**record).clone()],
                // Forward the upstream-emitted derived addresses to the
                // persister; the FFI layer feeds the iOS address registry
                // from this delta. See `CoreChangeSet.addresses_derived`.
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
            // Inserted records bring fresh UTXOs and may consume previous
            // ones — warn on a non-default account, but always project.
            for r in inserted {
                warn_if_non_default_account(r);
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
            // so a quiescent wallet's boundary advance still reaches
            // this bridge even when no record was promoted.
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

// merge: keep #3953's projection tests; is_empty_no_records dropped by our
// refactor in favour of Merge::is_empty() (the adapter call site uses it).
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dashcore::blockdata::transaction::Transaction;
    use dashcore::hashes::Hash;
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        OutputDetail, TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use key_wallet::WalletCoreBalance;

    use super::*;

    fn standard(index: u32) -> AccountType {
        AccountType::Standard {
            index,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    /// A throwaway testnet P2PKH address keyed off `seed`.
    fn p2pkh(seed: u8) -> dashcore::Address {
        use dashcore::address::Payload;
        use dashcore::PubkeyHash;
        dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([seed; 20])),
        )
    }

    /// A confirmed `TransactionRecord` owned by `account_type` carrying a
    /// single `Received` output worth `value` at `addr`, so
    /// `derive_new_utxos` yields exactly one UTXO.
    fn record_with_received_output(
        account_type: AccountType,
        addr: &dashcore::Address,
        value: u64,
    ) -> TransactionRecord {
        let tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![dashcore::TxOut {
                value,
                script_pubkey: addr.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            account_type,
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                42,
                dashcore::BlockHash::from_byte_array([3u8; 32]),
                1_735_689_600,
            )),
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            vec![OutputDetail {
                index: 0,
                role: OutputRole::Received,
                address: Some(addr.clone()),
                value,
            }],
            value as i64,
        )
    }

    /// Project a `TransactionDetected` for `record` through the real bridge
    /// path. `balance`/`account_balances` are unused by the projection.
    async fn changeset_for(record: TransactionRecord) -> CoreChangeSet {
        let wm = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            key_wallet::Network::Testnet,
        )));
        let event = WalletEvent::TransactionDetected {
            wallet_id: [0u8; 32],
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        build_core_changeset(&wm, &event).await
    }

    /// A default-account (index 0) UTXO is projected into the changeset.
    #[tokio::test]
    async fn default_account_utxo_persists() {
        let addr = p2pkh(0x11);
        let cs = changeset_for(record_with_received_output(standard(0), &addr, 500_000)).await;
        assert_eq!(
            cs.new_utxos.len(),
            1,
            "the default-account UTXO must be projected"
        );
        assert_eq!(cs.new_utxos[0].value(), 500_000);
    }

    /// REGRESSION (fund-loss): a non-default-account (index != 0) UTXO is
    /// STILL projected — never dropped. Storage persists it under
    /// `account_index 0`; the only cost is approximate per-account grouping
    /// (a `warn!` is logged). Dropping it would undercount the balance.
    #[tokio::test]
    async fn non_default_account_utxo_persists_under_zero() {
        let addr = p2pkh(0x22);
        let cs = changeset_for(record_with_received_output(standard(7), &addr, 900_000)).await;
        assert_eq!(
            cs.new_utxos.len(),
            1,
            "a non-default-account UTXO must NOT be dropped"
        );
        assert_eq!(cs.new_utxos[0].value(), 900_000, "funds preserved");
    }
}
