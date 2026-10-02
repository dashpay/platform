//! Load-time replay of stored transaction history.
//!
//! Every persister hands back its stored records as a [`RecordedHistory`];
//! [`replay_recorded_history`] runs them through the wallet checker so the
//! in-memory spend state (`spent_outpoints`, `observed_spent_outpoints`,
//! InstantSend upgrades) matches what a live process held. Without it a
//! redelivered funding transaction re-credits an output a confirmed spend
//! already consumed.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::{OutPoint, TxOut, Txid};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;

use crate::changeset::{RecordedHistory, StoredTransaction};

/// Restore spend reservations and finality guards without re-crediting outputs excluded by persistence.
///
/// The persisted UTXO set already restored onto `wallet_info` stays
/// authoritative for credits: an output the replay credits but persistence
/// did not hold as unspent is dropped again, so the replay only ever removes.
///
/// Unconfirmed (mempool / InstantSend) spends are replayed too: without them a
/// redelivered funding transaction would re-credit an output they reserve.
/// A persisted lock upgrades its mempool record to InstantSend, because a lock
/// that arrived after its transaction was stored never rewrote the record.
///
/// Raw restored records are replayed too: their presence alone does not
/// establish spend guards. Their labels, fees and proof-lookup records survive
/// replay unless a final conflicting transaction sweeps them.
///
/// Returns how many records were replayed.
pub async fn replay_recorded_history(
    wallet_info: &mut ManagedWalletInfo,
    wallet: &mut Wallet,
    history: RecordedHistory,
) -> usize {
    let RecordedHistory {
        transactions,
        instant_locks,
    } = history;
    if transactions.is_empty() {
        return 0;
    }
    // Where the load projection parked each unspent outpoint; its keys are
    // the outputs persistence still considers unspent.
    let placed: HashMap<OutPoint, AccountType> = wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| {
            let owner = funds_account_type(account);
            account.utxos.keys().map(move |outpoint| (*outpoint, owner))
        })
        .collect();
    let replay_txids: HashSet<Txid> = transactions.iter().map(|stored| stored.txid).collect();
    let mut held: HashMap<AccountType, Vec<TransactionRecord>> = HashMap::new();
    // Detach raw records so the checker rebuilds reservations even for known mempool txids.
    for mut account in wallet_info.accounts.all_accounts_mut() {
        let owner = account.managed_account_type().to_account_type();
        account.transactions_mut().retain(|txid, record| {
            if replay_txids.contains(txid) {
                held.entry(owner).or_default().push(record.clone());
                false
            } else {
                true
            }
        });
    }
    // TODO(bound-load-history-replay): every stored record is replayed on each
    // load; bounding it to records above the last chain lock needs care so
    // finality and spend guards for older records are not lost.
    // TODO(expire-unconfirmed-spend-reservations): mempool records are replayed
    // on every load with no expiry, so a forged or never-mined spend of a wallet
    // outpoint keeps its reservation across load and rescan; only a conflicting
    // IS-locked or confirmed spend releases it. Sibling of
    // TODO(release-repair-spends-after-reorg) in the SQLite `core_history`.
    let replay = replay_order(transactions);
    stage_recorded_spent_inputs(wallet_info, &replay, &placed, &replay_txids);

    let mut replayed = 0usize;
    let mut swept = HashSet::new();
    let mut final_transactions = Vec::new();
    let mut instant_send_winners = Vec::new();
    for stored in replay {
        // The lock set already holds this txid (load marked the restored
        // UTXOs), so a later lock event is deduplicated: the InstantSend
        // context, and the conflict sweep it runs, must come from here.
        let lock = instant_locks
            .get(&stored.txid)
            .filter(|lock| lock_matches_record(lock, &stored));
        let context = match (stored.context, lock) {
            (TransactionContext::Mempool, Some(lock)) => {
                TransactionContext::InstantSend(lock.clone())
            }
            (context, _) => context,
        };
        let result = wallet_info
            .check_core_transaction(&stored.transaction, context.clone(), wallet, true, false)
            .await;
        swept.extend(result.swept_transactions);
        if matches!(context, TransactionContext::InstantSend(_)) {
            instant_send_winners.push((stored.transaction.clone(), context.clone()));
        }
        if !held.is_empty() && !matches!(context, TransactionContext::Mempool) {
            final_transactions.push((stored.transaction, context));
        }
        replayed += 1;
    }
    // A lock's sweep only reaches conflicts already replayed; siblings replay
    // by txid, so settle the ones that came after their winner here. Their
    // held raw records must not come back as fallbacks either.
    for (transaction, context) in &instant_send_winners {
        swept.extend(wallet_info.sweep_conflicts(transaction, context).txids);
    }

    let spent: HashSet<_> = wallet_info
        .observed_spent_outpoints()
        .keys()
        .copied()
        .collect();
    // Replay credits an output to the account whose pool derives it. When
    // that differs from the load-time fallback, the fallback copy is a
    // duplicate: drop it so each outpoint lives in exactly one account.
    let misplaced: HashSet<(OutPoint, AccountType)> = wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| {
            let owner = funds_account_type(account);
            let placed = &placed;
            account.utxos.keys().filter_map(move |outpoint| {
                placed
                    .get(outpoint)
                    .filter(|parked| **parked != owner)
                    .map(|parked| (*outpoint, *parked))
            })
        })
        .collect();
    for account in wallet_info.accounts.all_funding_accounts_mut() {
        let owner = funds_account_type(account);
        account.utxos.retain(|outpoint, _| {
            placed.contains_key(outpoint)
                && !spent.contains(outpoint)
                && !misplaced.contains(&(*outpoint, owner))
        });
    }
    drop_swept_descendants(&mut swept, &held, &instant_locks);
    let mut restored_fallback = false;
    for mut account in wallet_info.accounts.all_accounts_mut() {
        let owner = account.managed_account_type().to_account_type();
        for original in held.remove(&owner).into_iter().flatten() {
            if swept.contains(&original.txid) {
                continue;
            }
            if let Some(replayed) = account.transactions_mut().get_mut(&original.txid) {
                replayed.label = original.label;
                replayed.fee = original.fee.or(replayed.fee);
            } else {
                // Proof lookup can require a record with no currently attributable inputs or outputs.
                account.transactions_mut().insert(original.txid, original);
                restored_fallback = true;
            }
        }
    }
    if restored_fallback {
        // Unattributable raw records were absent from the checker's conflict sweeps.
        for (transaction, context) in final_transactions {
            wallet_info.sweep_conflicts(&transaction, &context);
        }
    }
    // Finalize replayed records before a sync checkpoint can prune their spend guards.
    if let Some(chain_lock) = wallet_info.metadata.last_applied_chain_lock.clone() {
        wallet_info.apply_chain_lock(chain_lock);
    }
    wallet_info.update_balance();
    replayed
}

/// Extend `swept` with every held unconfirmed record that descends from a swept one.
///
/// A record the checker could not attribute is restored as a raw fallback,
/// and the final conflict sweeps reach its descendants only through a parent
/// that is still recorded. Once a sweep already removed that parent (it was
/// attributable, e.g. through a staged input), the orphaned child would come
/// back for good. Mirrors the checker's own rule: only unconfirmed, unlocked
/// descendants follow a swept parent.
fn drop_swept_descendants(
    swept: &mut HashSet<Txid>,
    held: &HashMap<AccountType, Vec<TransactionRecord>>,
    instant_locks: &BTreeMap<Txid, InstantLock>,
) {
    let followable: Vec<&TransactionRecord> = held
        .values()
        .flatten()
        .filter(|record| {
            matches!(record.context, TransactionContext::Mempool)
                && !instant_locks.contains_key(&record.txid)
        })
        .collect();
    loop {
        let before = swept.len();
        for record in &followable {
            if !swept.contains(&record.txid)
                && record
                    .transaction
                    .input
                    .iter()
                    .any(|input| swept.contains(&input.previous_output.txid))
            {
                swept.insert(record.txid);
            }
        }
        if swept.len() == before {
            return;
        }
    }
}

/// Park every owned input a record spends whose funding no replayed record credits.
///
/// Persistence excludes spent outputs from the load projection, so without
/// this a spender of a height-only funding row replays with no owned input and
/// rebuilds no spent mark; a redelivered funding transaction would then
/// re-credit the coin once finality prunes the observed spend. The stored
/// [`StoredTransaction::owned_inputs`] are the evidence. Staged coins are
/// never in `placed`, so the retention pass drops whatever replay leaves
/// behind.
fn stage_recorded_spent_inputs(
    wallet_info: &mut ManagedWalletInfo,
    records: &[StoredTransaction],
    placed: &HashMap<OutPoint, AccountType>,
    replay_txids: &HashSet<Txid>,
) {
    let mut accounts = wallet_info.accounts.all_funding_accounts_mut();
    for record in records {
        for detail in &record.owned_inputs {
            let Some(input) = record.transaction.input.get(detail.index as usize) else {
                continue;
            };
            let outpoint = input.previous_output;
            if placed.contains_key(&outpoint) || replay_txids.contains(&outpoint.txid) {
                continue;
            }
            let Some(account) = accounts
                .iter_mut()
                .find(|account| account.contains_address(&detail.address))
            else {
                continue;
            };
            account.utxos.entry(outpoint).or_insert_with(|| {
                let txout = TxOut {
                    value: detail.value,
                    script_pubkey: detail.address.script_pubkey(),
                };
                Utxo::new(outpoint, txout, detail.address.clone(), 0, false)
            });
        }
    }
}

/// Whether `lock` really locks `stored`, so upgrading its context is safe.
///
/// The lock map is keyed by the stored txid and a record's txid is stored
/// beside its transaction, so neither is proof on its own. A mismatch keeps
/// the stored context: the lock's conflict sweep must not drop history on the
/// strength of a lock that belongs to another transaction.
fn lock_matches_record(lock: &InstantLock, stored: &StoredTransaction) -> bool {
    let transaction_txid = stored.transaction.txid();
    let matches = lock.txid == stored.txid && transaction_txid == stored.txid;
    if !matches {
        tracing::warn!(
            record_txid = %stored.txid,
            transaction_txid = %transaction_txid,
            lock_txid = %lock.txid,
            "persisted InstantSend lock does not match its transaction record; replaying without it"
        );
    }
    matches
}

/// Order records as the chain would deliver them: every in-set parent ahead of
/// its children, otherwise confirmed by block position, then unconfirmed.
///
/// Dependencies span both partitions: a parent's stored record can still say
/// mempool after it confirmed (a height-only confirmation never rewrites an
/// existing record), while its child's record is already confirmed.
fn replay_order(records: Vec<StoredTransaction>) -> Vec<StoredTransaction> {
    let mut records = records;
    records.sort_by_key(|record| {
        let block = record.context.block_info();
        (
            block.is_none(),
            block.map(|block| (block.height(), block.position())),
            record.txid,
        )
    });
    let index: HashMap<Txid, usize> = records
        .iter()
        .enumerate()
        .map(|(position, record)| (record.txid, position))
        .collect();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); records.len()];
    let mut waiting_on: Vec<usize> = vec![0; records.len()];
    for (child, record) in records.iter().enumerate() {
        let parents: BTreeSet<usize> = record
            .transaction
            .input
            .iter()
            .filter_map(|input| index.get(&input.previous_output.txid).copied())
            .filter(|parent| *parent != child)
            .collect();
        waiting_on[child] = parents.len();
        for parent in parents {
            children[parent].push(child);
        }
    }
    // Sorted positions, so the smallest ready one is always next in chain order.
    let mut ready: BTreeSet<usize> = (0..records.len())
        .filter(|position| waiting_on[*position] == 0)
        .collect();
    let mut emitted = vec![false; records.len()];
    let mut order = Vec::with_capacity(records.len());
    while let Some(position) = ready.pop_first() {
        emitted[position] = true;
        order.push(position);
        for &child in &children[position] {
            waiting_on[child] -= 1;
            if waiting_on[child] == 0 {
                ready.insert(child);
            }
        }
    }
    // Unreachable for real transactions (txids cannot form a cycle); keep the
    // rest in chain order rather than drop a reservation.
    order.extend((0..records.len()).filter(|position| !emitted[*position]));
    let mut slots: Vec<Option<StoredTransaction>> = records.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|position| slots[position].take())
        .collect()
}

/// Account identity of a funds account, stable across replay mutations.
fn funds_account_type(account: &ManagedCoreFundsAccount) -> AccountType {
    account.managed_account_type().to_account_type()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use key_wallet::managed_account::transaction_record::TransactionRecord;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    /// Stored history built from checker-produced records.
    fn history(
        records: Vec<TransactionRecord>,
        instant_locks: BTreeMap<Txid, InstantLock>,
    ) -> RecordedHistory {
        RecordedHistory {
            transactions: records.into_iter().map(StoredTransaction::from).collect(),
            instant_locks,
        }
    }

    /// A fresh random wallet and its first BIP44 receive address.
    fn wallet_with_receive_address() -> (Wallet, ManagedWalletInfo, dashcore::Address) {
        let wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let address = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        (wallet, info, address)
    }

    /// A confirmed funding with two outputs and a confirmed spend of output 0,
    /// as checker-produced records, plus the persisted projection that holds
    /// only the unspent output 1.
    async fn confirmed_spend_fixture() -> (
        Wallet,
        ManagedWalletInfo,
        dashcore::Transaction,
        Vec<TransactionRecord>,
        OutPoint,
    ) {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::transaction_checking::BlockInfo;
        use key_wallet::Utxo;

        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([31; 32]), 0),
                ..Default::default()
            }],
            output: [100_000, 20_000]
                .map(|value| TxOut {
                    value,
                    script_pubkey: address.script_pubkey(),
                })
                .to_vec(),
            special_transaction_payload: None,
        };
        let spent = OutPoint::new(funding.txid(), 0);
        let available = OutPoint::new(funding.txid(), 1);
        let spending = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: spent,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 99_000,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let block = |height| {
            TransactionContext::InBlock(BlockInfo::new(
                height,
                BlockHash::from_byte_array([height as u8; 32]),
                height,
            ))
        };
        let mut records = info
            .check_core_transaction(&funding, block(100), &mut wallet, true, true)
            .await
            .new_records;
        records.extend(
            info.check_core_transaction(&spending, block(101), &mut wallet, true, true)
                .await
                .new_records,
        );
        assert_eq!(records.len(), 2);

        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
        restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .utxos
            .insert(
                available,
                Utxo {
                    outpoint: available,
                    txout: funding.output[1].clone(),
                    address,
                    height: 100,
                    is_coinbase: false,
                    is_confirmed: true,
                    is_instantlocked: false,
                    is_locked: false,
                    is_trusted: false,
                },
            );
        restored.update_balance();
        (wallet, restored, funding, records, spent)
    }

    /// The bug the replay exists for: a funding transaction redelivered after
    /// load (rescan, gap-limit rediscovery) must not resurrect an output a
    /// confirmed spend already consumed. The control half pins that the same
    /// redelivery does resurrect it when no history is replayed.
    #[tokio::test]
    async fn should_not_resurrect_confirmed_spent_output_on_funding_redelivery() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, restored, funding, records, spent) = confirmed_spend_fixture().await;
        let redelivery = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([100; 32]),
            100,
        ));

        let mut replayed = restored.clone();
        let count = replay_recorded_history(
            &mut replayed,
            &mut wallet,
            history(records, BTreeMap::new()),
        )
        .await;
        assert_eq!(count, 2);
        replayed
            .check_core_transaction(&funding, redelivery.clone(), &mut wallet, true, true)
            .await;
        assert!(
            !replayed.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&spent),
            "a redelivered funding must not re-credit a confirmed-spent output"
        );
        assert_eq!(replayed.balance.total(), 20_000);

        let mut unguarded = restored;
        unguarded
            .check_core_transaction(&funding, redelivery, &mut wallet, true, true)
            .await;
        assert!(
            unguarded.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&spent),
            "control: without the replay the redelivery resurrects the output"
        );
    }

    /// A spend whose funding survives only as a height row (no record to
    /// replay, spent output excluded from the projection) still rebuilds its
    /// spent mark from the stored owned inputs, and the staged coin does not
    /// outlive the replay. Finality then prunes the observed spend, so only
    /// the account spent mark guards the redelivery. The control half pins the
    /// gap a persister without per-input ownership leaves: the redelivery
    /// resurrects the output.
    #[tokio::test]
    async fn should_rebuild_spent_mark_for_height_only_funding_from_owned_inputs() {
        use dashcore::bls_sig_utils::BLSSignature;
        use dashcore::ephemerealdata::chain_lock::ChainLock;
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, restored, funding, records, spent) = confirmed_spend_fixture().await;
        let spender: Vec<StoredTransaction> = records
            .into_iter()
            .filter(|record| record.txid != funding.txid())
            .map(StoredTransaction::from)
            .collect();
        assert_eq!(spender.len(), 1);
        assert_eq!(spender[0].owned_inputs.len(), 1);
        let redelivery = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([100; 32]),
            100,
        ));
        let replay = |transactions| RecordedHistory {
            transactions,
            instant_locks: BTreeMap::new(),
        };
        let finalize = |info: &mut ManagedWalletInfo| {
            info.apply_chain_lock(ChainLock {
                block_height: 300,
                block_hash: BlockHash::from_byte_array([30; 32]),
                signature: BLSSignature::from([0; 96]),
            });
            info.update_synced_height(300);
            assert!(info.observed_spent_outpoints().is_empty());
        };

        let mut replayed = restored.clone();
        replay_recorded_history(&mut replayed, &mut wallet, replay(spender.clone())).await;
        assert!(
            !replayed.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&spent),
            "the staged input must not survive the replay"
        );
        assert_eq!(replayed.balance.total(), 20_000);
        finalize(&mut replayed);
        replayed
            .check_core_transaction(&funding, redelivery.clone(), &mut wallet, true, true)
            .await;
        assert!(
            !replayed.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&spent),
            "a redelivered height-only funding must not re-credit a spent output"
        );
        assert_eq!(replayed.balance.total(), 20_000);

        let mut unguarded = restored;
        let without_ownership = spender
            .into_iter()
            .map(|stored| StoredTransaction {
                owned_inputs: Vec::new(),
                ..stored
            })
            .collect();
        replay_recorded_history(&mut unguarded, &mut wallet, replay(without_ownership)).await;
        finalize(&mut unguarded);
        unguarded
            .check_core_transaction(&funding, redelivery, &mut wallet, true, true)
            .await;
        assert!(
            unguarded.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&spent),
            "control: without owned inputs the redelivery resurrects the output"
        );
    }

    #[tokio::test]
    async fn should_rebuild_guards_for_raw_restored_records() {
        for mempool in [false, true] {
            let (mut wallet, mut restored, funding, mut records, spent) =
                confirmed_spend_fixture().await;
            if mempool {
                records[1].context = TransactionContext::Mempool;
            }
            let mut held = records[1].clone();
            held.label = "retained label".into();
            held.fee = Some(1_000);
            let txid = held.txid;
            restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .transactions_mut()
                .insert(txid, held);

            let count = replay_recorded_history(
                &mut restored,
                &mut wallet,
                history(records.clone(), BTreeMap::new()),
            )
            .await;
            restored
                .check_core_transaction(
                    &funding,
                    records[0].context.clone(),
                    &mut wallet,
                    true,
                    true,
                )
                .await;
            let account = &restored.accounts.standard_bip44_accounts[&0];
            assert!(!account.utxos.contains_key(&spent), "mempool={mempool}");
            assert_eq!(restored.balance.total(), 20_000);
            assert_eq!(count, 2);
            let held = &account.transactions()[&txid];
            assert_eq!(held.label, "retained label");
            assert_eq!(held.fee, Some(1_000));
        }
    }

    #[tokio::test]
    async fn should_retain_raw_record_without_matching_utxos() {
        let (mut wallet, mut restored, _, records, _) = confirmed_spend_fixture().await;
        let mut held = records[1].clone();
        held.context = TransactionContext::Mempool;
        held.label = "proof lookup".into();
        let txid = held.txid;
        restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .transactions_mut()
            .insert(txid, held.clone());
        replay_recorded_history(
            &mut restored,
            &mut wallet,
            history(vec![held], BTreeMap::new()),
        )
        .await;
        assert_eq!(
            restored.accounts.standard_bip44_accounts[&0].transactions()[&txid].label,
            "proof lookup"
        );
    }

    #[tokio::test]
    async fn should_drop_raw_conflict_without_matching_utxos() {
        let (mut wallet, restored, _, records, _) = confirmed_spend_fixture().await;
        for instant in [false, true] {
            let mut info = restored.clone();
            let mut loser = records[1].clone();
            loser.context = TransactionContext::Mempool;
            let mut child = loser.clone();
            child.transaction.input[0].previous_output = OutPoint::new(loser.txid, 0);
            child.txid = child.transaction.txid();
            let mut winner = loser.clone();
            winner.transaction.output[0].value -= 1;
            winner.txid = winner.transaction.txid();
            winner.context = if instant {
                TransactionContext::InstantSend(InstantLock {
                    txid: winner.txid,
                    inputs: winner
                        .transaction
                        .input
                        .iter()
                        .map(|input| input.previous_output)
                        .collect(),
                    ..Default::default()
                })
            } else {
                records[1].context.clone()
            };
            info.accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .transactions_mut()
                .insert(loser.txid, loser.clone());
            info.accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .transactions_mut()
                .insert(child.txid, child.clone());
            replay_recorded_history(
                &mut info,
                &mut wallet,
                history(vec![loser.clone(), child.clone(), winner], BTreeMap::new()),
            )
            .await;
            assert!(
                !info.accounts.standard_bip44_accounts[&0]
                    .transactions()
                    .contains_key(&loser.txid),
                "instant={instant}"
            );
            assert!(
                !info.accounts.standard_bip44_accounts[&0]
                    .transactions()
                    .contains_key(&child.txid),
                "instant={instant}"
            );
            assert_eq!(info.balance.total(), 20_000);
        }
    }

    /// A held, unattributable child of a conflict loser must go with its
    /// parent even when the loser was attributable (its spent input staged
    /// from the stored owned inputs) and a sweep already removed it before
    /// the fallback restore. Covers both sibling replay orders.
    #[tokio::test]
    async fn should_drop_orphaned_raw_descendant_of_swept_conflict_in_either_order() {
        let (mut wallet, restored, _, records, _) = confirmed_spend_fixture().await;
        let mut loser = records[1].clone();
        loser.context = TransactionContext::Mempool;
        let mut child = loser.clone();
        child.transaction.input[0].previous_output = OutPoint::new(loser.txid, 0);
        child.txid = child.transaction.txid();
        child.input_details.clear();
        let winner_with = |delta: u64| {
            let mut winner = loser.clone();
            winner.transaction.output[0].value -= delta;
            winner.txid = winner.transaction.txid();
            winner.context = TransactionContext::InstantSend(InstantLock {
                txid: winner.txid,
                inputs: winner
                    .transaction
                    .input
                    .iter()
                    .map(|input| input.previous_output)
                    .collect(),
                ..Default::default()
            });
            winner
        };
        for winner_first in [true, false] {
            let winner = (1..)
                .map(winner_with)
                .find(|winner| (winner.txid < loser.txid) == winner_first)
                .unwrap();
            let mut info = restored.clone();
            let account = info.accounts.standard_bip44_accounts.get_mut(&0).unwrap();
            for raw in [&loser, &child] {
                account.transactions_mut().insert(raw.txid, raw.clone());
            }
            replay_recorded_history(
                &mut info,
                &mut wallet,
                history(vec![loser.clone(), child.clone(), winner], BTreeMap::new()),
            )
            .await;
            let transactions = info.accounts.standard_bip44_accounts[&0].transactions();
            assert!(
                !transactions.contains_key(&loser.txid),
                "winner_first={winner_first}: the loser must be swept"
            );
            assert!(
                !transactions.contains_key(&child.txid),
                "winner_first={winner_first}: the loser's raw child must not come back"
            );
            assert_eq!(info.balance.total(), 20_000, "winner_first={winner_first}");
        }
    }

    /// An empty history leaves the restored projection untouched.
    #[tokio::test]
    async fn should_leave_projection_untouched_without_history() {
        let (mut wallet, restored, _, _, _) = confirmed_spend_fixture().await;
        let mut info = restored.clone();
        let count =
            replay_recorded_history(&mut info, &mut wallet, RecordedHistory::default()).await;
        assert_eq!(count, 0);
        assert_eq!(info.balance.total(), restored.balance.total());
        assert_eq!(
            info.accounts.standard_bip44_accounts[&0].utxos.len(),
            restored.accounts.standard_bip44_accounts[&0].utxos.len()
        );
    }

    /// Same-block funding and spend, with and without in-block positions: an
    /// output the load projection still parks as unspent must end up excluded,
    /// and recorded as observed spent, whichever of the two is stored first.
    #[tokio::test]
    async fn should_exclude_spent_output_for_either_same_height_replay_order() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::transaction_checking::BlockInfo;
        use key_wallet::Utxo;

        for (spend_first, positioned) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let case = format!("spend_first={spend_first} positioned={positioned}");
            let (mut wallet, mut info, address) = wallet_with_receive_address();
            let funding = Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(Txid::from_byte_array([15; 32]), 0),
                    ..Default::default()
                }],
                output: [100_000, 20_000]
                    .map(|value| TxOut {
                        value,
                        script_pubkey: address.script_pubkey(),
                    })
                    .to_vec(),
                special_transaction_payload: None,
            };
            let (spent, available) = (
                OutPoint::new(funding.txid(), 0),
                OutPoint::new(funding.txid(), 1),
            );
            let spending = Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: spent,
                    ..Default::default()
                }],
                output: vec![TxOut {
                    value: 99_000,
                    script_pubkey: dashcore::ScriptBuf::new(),
                }],
                special_transaction_payload: None,
            };
            let context = |position: u32| {
                let block = BlockInfo::new(100, BlockHash::from_byte_array([7; 32]), 100);
                TransactionContext::InBlock(if positioned {
                    block.with_position(position)
                } else {
                    block
                })
            };
            let mut records = info
                .check_core_transaction(&funding, context(1), &mut wallet, true, true)
                .await
                .new_records;
            records.extend(
                info.check_core_transaction(&spending, context(2), &mut wallet, true, true)
                    .await
                    .new_records,
            );
            assert_eq!(records.len(), 2);
            assert!(records.iter().all(|r| r
                .block_info()
                .is_some_and(|b| b.position().is_some() == positioned)));
            if spend_first {
                records.reverse();
            }

            // A stale projection that still parks the spent output as unspent.
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            let account = restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap();
            for outpoint in [spent, available] {
                account.utxos.insert(
                    outpoint,
                    Utxo {
                        outpoint,
                        txout: funding.output[outpoint.vout as usize].clone(),
                        address: address.clone(),
                        height: 100,
                        is_coinbase: false,
                        is_confirmed: true,
                        is_instantlocked: false,
                        is_locked: false,
                        is_trusted: false,
                    },
                );
            }
            replay_recorded_history(
                &mut restored,
                &mut wallet,
                history(records, BTreeMap::new()),
            )
            .await;

            let coins = &restored.accounts.standard_bip44_accounts[&0].utxos;
            assert!(!coins.contains_key(&spent), "{case}");
            assert!(coins.contains_key(&available), "{case}");
            assert!(
                restored.observed_spent_outpoints().contains_key(&spent),
                "{case}"
            );
            assert_eq!(restored.balance.total(), 20_000, "{case}");
        }
    }

    /// A lock that arrived after its transaction was stored lives only in
    /// `core_instant_locks`; the stored record still says mempool. Replay must
    /// restore the InstantSend context and run its conflict sweep, since the
    /// already-marked lock deduplicates any later lock event.
    #[tokio::test]
    async fn should_replay_mempool_record_with_persisted_lock_as_instant_send() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([21; 32]), 0),
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 100_000,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let spend = |value| Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(funding.txid(), 0),
                ..Default::default()
            }],
            output: vec![TxOut {
                value,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let block = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([8; 32]),
            100,
        ));
        let mut records = info
            .check_core_transaction(&funding, block, &mut wallet, true, true)
            .await
            .new_records;
        // Both double spends as stored: unconfirmed, recorded independently.
        let (mut winner, mut loser) = (spend(99_000), spend(98_000));
        for tx in [&winner, &loser] {
            let mut scratch = info.clone();
            records.extend(
                scratch
                    .check_core_transaction(
                        tx,
                        TransactionContext::Mempool,
                        &mut wallet,
                        true,
                        true,
                    )
                    .await
                    .new_records,
            );
        }
        assert_eq!(records.len(), 3);
        // Unconfirmed siblings replay by txid: lock the later one so the
        // loser is already recorded when the winner's sweep runs.
        if winner.txid() < loser.txid() {
            std::mem::swap(&mut winner, &mut loser);
        }
        let lock = InstantLock {
            inputs: vec![OutPoint::new(funding.txid(), 0)],
            txid: winner.txid(),
            ..Default::default()
        };
        let locks: BTreeMap<Txid, InstantLock> = [(winner.txid(), lock)].into_iter().collect();

        let record_of = |txid: Txid| records.iter().find(|r| r.txid == txid).unwrap().clone();

        // Alone, the locked spend comes back InstantSend.
        let mut alone = ManagedWalletInfo::from_wallet(&wallet, 0);
        let pair = vec![record_of(funding.txid()), record_of(winner.txid())];
        replay_recorded_history(&mut alone, &mut wallet, history(pair, locks.clone())).await;
        assert!(
            alone.accounts.standard_bip44_accounts[&0]
                .transactions()
                .get(&winner.txid())
                .is_some_and(|record| matches!(record.context, TransactionContext::InstantSend(_))),
            "the locked record must come back InstantSend, not mempool"
        );

        // Replayed after a conflicting spend, its lock sweeps that spend.
        let mut contested = ManagedWalletInfo::from_wallet(&wallet, 0);
        contested
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .transactions_mut()
            .insert(loser.txid(), record_of(loser.txid()));
        contested
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .transactions_mut()
            .insert(winner.txid(), record_of(winner.txid()));
        replay_recorded_history(
            &mut contested,
            &mut wallet,
            history(records.clone(), locks.clone()),
        )
        .await;
        assert!(
            !contested.accounts.standard_bip44_accounts[&0]
                .transactions()
                .contains_key(&loser.txid()),
            "the lock's conflict sweep must drop the competing spend"
        );
    }

    /// An InstantSend winner must sweep its conflicting mempool sibling whichever
    /// of the two replays first, so the loser's wallet-owned change, still
    /// persisted as unspent, does not come back selectable, nor its raw
    /// restored record as an unattributable fallback.
    #[tokio::test]
    async fn should_sweep_conflicting_spend_for_either_sibling_replay_order() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::transaction_checking::BlockInfo;
        use key_wallet::Utxo;

        for (lock_later_txid, raw_restored) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let case = format!("lock_later_txid={lock_later_txid} raw_restored={raw_restored}");
            let (mut wallet, mut info, address) = wallet_with_receive_address();
            let funding = Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(Txid::from_byte_array([23; 32]), 0),
                    ..Default::default()
                }],
                output: vec![TxOut {
                    value: 100_000,
                    script_pubkey: address.script_pubkey(),
                }],
                special_transaction_payload: None,
            };
            let spend = |change| Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(funding.txid(), 0),
                    ..Default::default()
                }],
                output: vec![
                    TxOut {
                        value: 99_000 - change,
                        script_pubkey: dashcore::ScriptBuf::new(),
                    },
                    TxOut {
                        value: change,
                        script_pubkey: address.script_pubkey(),
                    },
                ],
                special_transaction_payload: None,
            };
            let block = TransactionContext::InBlock(BlockInfo::new(
                100,
                BlockHash::from_byte_array([9; 32]),
                100,
            ));
            let mut records = info
                .check_core_transaction(&funding, block, &mut wallet, true, true)
                .await
                .new_records;
            let (mut winner, mut loser) = (spend(40_000), spend(30_000));
            if (winner.txid() > loser.txid()) != lock_later_txid {
                std::mem::swap(&mut winner, &mut loser);
            }
            // Both siblings as stored: unconfirmed, each credited its change.
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            for tx in [&winner, &loser] {
                let mut scratch = info.clone();
                records.extend(
                    scratch
                        .check_core_transaction(
                            tx,
                            TransactionContext::Mempool,
                            &mut wallet,
                            true,
                            true,
                        )
                        .await
                        .new_records,
                );
                let change = OutPoint::new(tx.txid(), 1);
                restored
                    .accounts
                    .standard_bip44_accounts
                    .get_mut(&0)
                    .unwrap()
                    .utxos
                    .insert(
                        change,
                        Utxo::new(change, tx.output[1].clone(), address.clone(), 0, false),
                    );
            }
            assert_eq!(records.len(), 3, "{case}");
            if raw_restored {
                let account = restored
                    .accounts
                    .standard_bip44_accounts
                    .get_mut(&0)
                    .unwrap();
                for record in &records {
                    account
                        .transactions_mut()
                        .insert(record.txid, record.clone());
                }
            }
            let lock = InstantLock {
                inputs: vec![OutPoint::new(funding.txid(), 0)],
                txid: winner.txid(),
                ..Default::default()
            };
            let locks: BTreeMap<Txid, InstantLock> = [(winner.txid(), lock)].into_iter().collect();

            replay_recorded_history(&mut restored, &mut wallet, history(records, locks)).await;

            let account = &restored.accounts.standard_bip44_accounts[&0];
            assert!(
                !account.transactions().contains_key(&loser.txid()),
                "{case}: the lock must sweep the competing spend"
            );
            assert!(
                !account.utxos.contains_key(&OutPoint::new(loser.txid(), 1)),
                "{case}: the swept spend's change must not stay selectable"
            );
            assert!(
                account.utxos.contains_key(&OutPoint::new(winner.txid(), 1)),
                "{case}: the winner's change stays"
            );
            assert_eq!(restored.balance.total(), winner.output[1].value, "{case}");
        }
    }

    /// A persisted lock is trusted only when it names the record it is keyed
    /// under and that record's transaction really has that txid; otherwise the
    /// record replays in its stored mempool context and no sweep runs.
    #[tokio::test]
    async fn should_not_upgrade_record_to_instant_send_with_mismatched_lock() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([23; 32]), 0),
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 100_000,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let spend = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(funding.txid(), 0),
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 99_000,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let block = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([8; 32]),
            100,
        ));
        let mut records = info
            .check_core_transaction(&funding, block, &mut wallet, true, true)
            .await
            .new_records;
        records.extend(
            info.check_core_transaction(
                &spend,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await
            .new_records,
        );
        assert_eq!(records.len(), 2);
        let foreign = Txid::from_byte_array([24; 32]);
        let lock_for = |txid| InstantLock {
            inputs: vec![OutPoint::new(funding.txid(), 0)],
            txid,
            ..Default::default()
        };
        let mut forged_record = records.clone();
        forged_record
            .iter_mut()
            .find(|record| record.txid == spend.txid())
            .unwrap()
            .txid = foreign;
        let cases = [
            // The lock row is keyed under the record but locks another txid.
            (
                "lock txid",
                records.clone(),
                spend.txid(),
                lock_for(foreign),
            ),
            // Record and lock agree, but the record's transaction is another one.
            ("record txid", forged_record, foreign, lock_for(foreign)),
        ];
        for (case, records, key, lock) in cases {
            let locks: BTreeMap<Txid, InstantLock> = [(key, lock)].into_iter().collect();
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            replay_recorded_history(&mut restored, &mut wallet, history(records, locks.clone()))
                .await;
            let transactions = restored.accounts.standard_bip44_accounts[&0].transactions();
            assert!(
                !transactions
                    .values()
                    .any(|record| matches!(record.context, TransactionContext::InstantSend(_))),
                "{case}: a mismatched lock must not upgrade any record"
            );
            assert!(
                transactions.contains_key(&spend.txid()),
                "{case}: the spend must still replay in its stored context"
            );
        }
    }

    /// A record spending `parents` (output 0 of each); `value` keeps txids distinct.
    fn replay_record(
        parents: &[Txid],
        value: u64,
        context: TransactionContext,
    ) -> TransactionRecord {
        use dashcore::{Transaction, TxIn, TxOut};
        use key_wallet::account::StandardAccountType;
        use key_wallet::managed_account::transaction_record::TransactionDirection;
        use key_wallet::transaction_checking::TransactionType;

        let transaction = Transaction {
            version: 1,
            lock_time: 0,
            input: parents
                .iter()
                .map(|parent| TxIn {
                    previous_output: OutPoint::new(*parent, 0),
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn in_block(height: u32, position: Option<u32>) -> TransactionContext {
        use dashcore::hashes::Hash;
        use key_wallet::transaction_checking::BlockInfo;

        let block = BlockInfo::new(height, dashcore::BlockHash::all_zeros(), height);
        TransactionContext::InBlock(position.map_or(block, |p| block.with_position(p)))
    }

    fn replayed_txids(records: Vec<TransactionRecord>) -> Vec<Txid> {
        replay_order(records.into_iter().map(Into::into).collect())
            .into_iter()
            .map(|r| r.txid)
            .collect()
    }

    /// An unconfirmed child whose txid sorts ahead of its parent's still
    /// replays after it, so its input reserves the parent's output.
    #[test]
    fn should_replay_unconfirmed_parent_before_its_child() {
        use dashcore::hashes::Hash;

        let parent = replay_record(
            &[Txid::from_byte_array([1; 32])],
            1_000,
            TransactionContext::Mempool,
        );
        let child = (0..)
            .map(|value| replay_record(&[parent.txid], value, TransactionContext::Mempool))
            .find(|child| child.txid < parent.txid)
            .unwrap();
        let expected = vec![parent.txid, child.txid];

        assert_eq!(replayed_txids(vec![child, parent]), expected);
    }

    /// A parent whose stored record is still mempool replays ahead of a child
    /// already recorded as confirmed.
    #[test]
    fn should_replay_mempool_parent_before_its_confirmed_child() {
        use dashcore::hashes::Hash;

        let parent = replay_record(
            &[Txid::from_byte_array([2; 32])],
            1_000,
            TransactionContext::Mempool,
        );
        let child = replay_record(&[parent.txid], 900, in_block(50, Some(3)));
        let unrelated = replay_record(&[Txid::from_byte_array([3; 32])], 700, in_block(40, None));
        let expected = vec![unrelated.txid, parent.txid, child.txid];

        assert_eq!(replayed_txids(vec![child, unrelated, parent]), expected);
    }

    /// Independent records follow chain order: height, then in-block
    /// position, then unconfirmed.
    #[test]
    fn should_order_independent_records_by_height_then_block_position() {
        use dashcore::hashes::Hash;

        let funding = |marker| [Txid::from_byte_array([marker; 32])];
        let pending = replay_record(&funding(4), 1, TransactionContext::Mempool);
        let late_second = replay_record(&funding(5), 2, in_block(10, Some(2)));
        let late_first = replay_record(&funding(6), 3, in_block(10, Some(1)));
        let early = replay_record(&funding(7), 4, in_block(9, Some(5)));
        let expected = vec![early.txid, late_first.txid, late_second.txid, pending.txid];

        assert_eq!(
            replayed_txids(vec![pending, late_second, late_first, early]),
            expected
        );
    }

    /// Within one block, in-block position decides: a spend follows the
    /// funding transaction it spends, and unrelated transactions keep their
    /// place around the pair.
    #[test]
    fn should_keep_block_position_order_for_same_block_spends() {
        use dashcore::hashes::Hash;

        let parent = replay_record(&[Txid::from_byte_array([9; 32])], 10, in_block(20, Some(1)));
        let child = replay_record(&[parent.txid], 9, in_block(20, Some(2)));
        let before = replay_record(&[Txid::from_byte_array([10; 32])], 8, in_block(20, Some(0)));
        let after = replay_record(&[Txid::from_byte_array([11; 32])], 7, in_block(20, Some(3)));
        let expected = vec![before.txid, parent.txid, child.txid, after.txid];

        assert_eq!(replayed_txids(vec![after, child, before, parent]), expected);
    }

    /// Records that name each other as parents (impossible for real txids)
    /// still all replay, after everything that is ready.
    #[test]
    fn should_keep_every_record_of_a_dependency_cycle() {
        use dashcore::hashes::Hash;

        let (a, b) = (
            Txid::from_byte_array([0xAA; 32]),
            Txid::from_byte_array([0xBB; 32]),
        );
        let mut first = replay_record(&[b], 1, TransactionContext::Mempool);
        first.txid = a;
        let mut second = replay_record(&[a], 2, TransactionContext::Mempool);
        second.txid = b;
        let ready = replay_record(
            &[Txid::from_byte_array([8; 32])],
            3,
            TransactionContext::Mempool,
        );
        let expected = vec![ready.txid, a, b];

        assert_eq!(replayed_txids(vec![second, first, ready]), expected);
    }
}
