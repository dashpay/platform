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

use std::collections::BTreeSet;
use std::sync::Arc;

use dashcore::blockdata::transaction::{txout::TxOut, OutPoint};
use dashcore::ScriptBuf;
use key_wallet::account::AccountType;
use key_wallet::managed_account::transaction_record::{OutputRole, TransactionRecord};
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::Utxo;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::changeset::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet,
};
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
                            let cs =
                                build_platform_changeset(&wallet_manager, &wallet_id, core).await;
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

/// Wrap a [`CoreChangeSet`] in a [`PlatformWalletChangeSet`], attaching a
/// full pool snapshot in-band when the delta derived new addresses.
///
/// `addresses_derived` is a delta with no `used` flag, so it can't be the
/// manifest. When non-empty the in-memory pool just changed, so we read
/// the whole current pool. It must ride the same changeset because the
/// persister applies `account_address_pools` before the core UTXO delta
/// in one tx, making any newly-derived address resolvable when this
/// changeset's UTXOs are written — closing the gap-limit race in-band.
///
/// A `used` flip with no derivation leaves the delta empty and is
/// intentionally not snapshot here: `used` doesn't affect
/// address→account resolution, and emitting on every wallet-touching
/// block would be pure write amplification.
async fn build_platform_changeset(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    core: CoreChangeSet,
) -> PlatformWalletChangeSet {
    let mut account_address_pools = Vec::new();
    if !core.addresses_derived.is_empty() {
        let guard = wallet_manager.read().await;
        let affected: BTreeSet<AccountType> = core
            .addresses_derived
            .iter()
            .map(|d| d.account_type)
            .collect();
        for account_type in &affected {
            account_address_pools.extend(snapshot_account_pools(&guard, wallet_id, account_type));
        }
    }
    PlatformWalletChangeSet {
        core: Some(core),
        account_address_pools,
        ..PlatformWalletChangeSet::default()
    }
}

/// Snapshot one account's non-empty pools straight from the live
/// `WalletManager`, mirroring the enumeration the registration path uses
/// (`account_address_pools_blocking` is on a different type, so the shared
/// walk lives here). Empty vec when the wallet/account is absent.
fn snapshot_account_pools(
    guard: &WalletManager<PlatformWalletInfo>,
    wallet_id: &WalletId,
    account_type: &AccountType,
) -> Vec<AccountAddressPoolEntry> {
    let Some(info) = guard.get_wallet_info(wallet_id) else {
        return Vec::new();
    };
    let accounts = info.core_wallet.accounts.all_accounts();
    let Some(account) = accounts
        .iter()
        .find(|a| &a.managed_account_type().to_account_type() == account_type)
    else {
        return Vec::new();
    };
    account
        .managed_account_type()
        .address_pools()
        .iter()
        .filter(|pool| !pool.addresses.is_empty())
        .map(|pool| AccountAddressPoolEntry {
            account_type: *account_type,
            pool_type: pool.pool_type,
            addresses: pool.addresses.values().cloned().collect(),
        })
        .collect()
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
    use std::collections::BTreeMap;

    use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;
    use key_wallet_manager::DerivedAddress;

    use super::*;
    use crate::wallet::core::WalletBalance;
    use crate::wallet::identity::IdentityManager;

    /// Register a seeded wallet (default account creation derives the
    /// gap-limit pools) into a fresh manager.
    fn manager_with_wallet() -> (Arc<RwLock<WalletManager<PlatformWalletInfo>>>, WalletId) {
        let wallet = Wallet::from_seed_bytes(
            [0x42; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .expect("wallet from seed");
        let info = PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(&wallet, 0),
            balance: Arc::new(WalletBalance::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
        };
        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let wallet_id = wm.insert_wallet(wallet, info).expect("insert wallet");
        (Arc::new(RwLock::new(wm)), wallet_id)
    }

    /// First funds account's type plus the index count of its external
    /// pool, and a `DerivedAddress` forged from a real address in it (the
    /// emitter keys only off `account_type`).
    fn funds_account_and_derived(
        guard: &WalletManager<PlatformWalletInfo>,
        wallet_id: &WalletId,
    ) -> (AccountType, usize, DerivedAddress) {
        let info = guard.get_wallet_info(wallet_id).expect("wallet present");
        for account in &info.core_wallet.accounts.all_accounts() {
            let account_type = account.managed_account_type().to_account_type();
            for pool in account.managed_account_type().address_pools() {
                if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                    continue;
                }
                let addr = pool.addresses.values().next().expect("address present");
                let Some(PublicKeyType::ECDSA(bytes)) = addr.public_key.as_ref() else {
                    continue;
                };
                let derived = DerivedAddress {
                    account_type,
                    pool_type: pool.pool_type,
                    derivation_index: addr.index,
                    address: addr.address.clone(),
                    public_key: dashcore::PublicKey::from_slice(bytes).expect("valid key"),
                };
                return (account_type, pool.addresses.len(), derived);
            }
        }
        panic!("no funds account with a populated ECDSA external pool");
    }

    fn core_with_derived(derived: Vec<DerivedAddress>) -> CoreChangeSet {
        CoreChangeSet {
            last_processed_height: Some(100),
            addresses_derived: derived,
            ..CoreChangeSet::default()
        }
    }

    /// A derivation delta yields the FULL pool (every index), not just the
    /// new one, and it rides the SAME changeset as the core delta.
    #[tokio::test]
    async fn extension_snapshots_full_pool_in_band() {
        let (wm, wallet_id) = manager_with_wallet();
        let (account_type, full_len, derived) = {
            let guard = wm.read().await;
            funds_account_and_derived(&guard, &wallet_id)
        };
        assert!(full_len > 1, "external pool populated to the gap limit");

        let cs = build_platform_changeset(&wm, &wallet_id, core_with_derived(vec![derived])).await;

        assert!(cs.core.is_some(), "core delta rides the same changeset");
        let entry = cs
            .account_address_pools
            .iter()
            .find(|e| e.account_type == account_type && e.pool_type == AddressPoolType::External)
            .expect("external pool snapshot for the affected account");
        assert_eq!(entry.addresses.len(), full_len, "FULL pool, not the delta");
    }

    /// An empty delta emits no pool snapshot — no write amplification on
    /// blocks that derive nothing.
    #[tokio::test]
    async fn empty_derivation_emits_no_snapshot() {
        let (wm, wallet_id) = manager_with_wallet();
        let cs = build_platform_changeset(&wm, &wallet_id, core_with_derived(Vec::new())).await;
        assert!(cs.account_address_pools.is_empty());
        assert!(cs.core.is_some(), "core delta still carried");
    }

    /// The emitter snapshot matches the registration-path enumeration for
    /// the same account (same pools, same addresses per pool), so
    /// registration semantics are preserved. `AccountAddressPoolEntry`
    /// isn't `PartialEq`, so compare on a stable projection.
    #[tokio::test]
    async fn emitter_matches_registration_shape() {
        let (wm, wallet_id) = manager_with_wallet();
        let guard = wm.read().await;
        let (account_type, _, _) = funds_account_and_derived(&guard, &wallet_id);

        let project = |entries: &[AccountAddressPoolEntry]| -> Vec<(AddressPoolType, Vec<String>)> {
            let mut out: Vec<_> = entries
                .iter()
                .map(|e| {
                    let mut addrs: Vec<String> =
                        e.addresses.iter().map(|a| a.address.to_string()).collect();
                    addrs.sort();
                    (e.pool_type, addrs)
                })
                .collect();
            out.sort_by_key(|(pt, _)| *pt);
            out
        };

        let emitted = snapshot_account_pools(&guard, &wallet_id, &account_type);

        let info = guard.get_wallet_info(&wallet_id).expect("wallet present");
        let account = info
            .core_wallet
            .accounts
            .all_accounts()
            .into_iter()
            .find(|a| a.managed_account_type().to_account_type() == account_type)
            .expect("account present");
        let registration_shape: Vec<AccountAddressPoolEntry> = account
            .managed_account_type()
            .address_pools()
            .iter()
            .filter(|p| !p.addresses.is_empty())
            .map(|p| AccountAddressPoolEntry {
                account_type,
                pool_type: p.pool_type,
                addresses: p.addresses.values().cloned().collect(),
            })
            .collect();

        assert_eq!(project(&emitted), project(&registration_shape));
        assert!(emitted.iter().all(|e| e.account_type == account_type));
        assert!(!emitted.is_empty());
    }

    /// Unknown wallet → empty snapshot, not a panic.
    #[tokio::test]
    async fn snapshot_unknown_wallet_is_empty() {
        let (wm, wallet_id) = manager_with_wallet();
        let guard = wm.read().await;
        let (account_type, _, _) = funds_account_and_derived(&guard, &wallet_id);
        assert!(snapshot_account_pools(&guard, &[0xAB; 32], &account_type).is_empty());
    }
}
