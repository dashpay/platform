//! Load-time replay of stored transaction history.
//!
//! A persister hands back its stored records as a [`RecordedHistory`];
//! [`replay_recorded_history`] runs them through the wallet checker so the
//! in-memory spend state (`spent_outpoints`, `observed_spent_outpoints`,
//! InstantSend upgrades) matches what a live process held. Without it a
//! redelivered funding transaction re-credits an output a confirmed spend
//! already consumed.
//!
//! A persister that must repair its own rows while it replays does so inside
//! its `load` and hands back an empty history; the SQLite one does.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::transaction::TransactionPayload;
use dashcore::{OutPoint, Transaction, TxOut, Txid};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::{
    InputDetail, TransactionDirection, TransactionRecord,
};
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::transaction_checking::{
    TransactionContext, TransactionType, WalletTransactionChecker,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;

use crate::changeset::{RecordedHistory, StoredTransaction};

/// What [`replay_recorded_history`] made of the stored history.
#[derive(Debug, Default)]
pub struct HistoryReplay {
    /// How many records went through the wallet checker.
    pub replayed: usize,
    /// Stored transactions the replay removed or left out: the losers of a
    /// settled conflict with their descendants, and the records with unsafe
    /// amounts.
    pub retired: HashSet<Txid>,
}

impl HistoryReplay {
    /// The unconfirmed sends of `sends` still worth dispatching again.
    ///
    /// A dispatch hands the transaction back to the wallet as a mempool
    /// sighting: a retired loser would be recorded again, and a record with
    /// unsafe amounts would reach the checker the replay kept it from.
    pub fn resendable(&self, sends: Vec<Transaction>) -> Vec<Transaction> {
        sends
            .into_iter()
            .filter(|send| !self.retired.contains(&send.txid()))
            .collect()
    }
}

/// Restore spend reservations and finality guards without re-crediting outputs excluded by persistence.
///
/// The persisted UTXO set already restored onto `wallet_info` stays
/// authoritative for credits: an output the replay credits but persistence
/// did not hold as unspent is dropped again, so the replay only ever removes.
/// A coin it keeps also keeps its stored lock, InstantSend status and
/// confirmation.
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
/// A stored transaction that a settled one beats to an input is not replayed,
/// nor are its unconfirmed descendants, and their persisted outputs are dropped.
///
/// A record whose amounts would overflow the checker's signed arithmetic
/// ([`unsafe_replay_amounts`]) is left out rather than failing the load. No
/// valid transaction carries such amounts: an unconfirmed one is unverified
/// peer data, a settled one is store corruption, and this path has no recovery
/// mode to fall back on. The restored coins it spends stay reserved, and an
/// unconfirmed one stays on record, unchecked, so that a settled conflict
/// still sweeps it and releases them. A coin two such records spend is held
/// for one of them only and is released with that one.
///
/// Returns how many records were replayed and which stored transactions were
/// retired instead.
pub async fn replay_recorded_history(
    wallet_info: &mut ManagedWalletInfo,
    wallet: &mut Wallet,
    history: RecordedHistory,
) -> HistoryReplay {
    let RecordedHistory {
        transactions,
        instant_locks,
    } = history;
    if transactions.is_empty() {
        return HistoryReplay::default();
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
    // The same coins as persistence restored them; see `restore_persisted_coin_state`.
    let persisted: HashMap<OutPoint, Utxo> = wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.clone())
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
    let mut replay = replay_order(transactions);
    let chain_locked_height = wallet_info
        .metadata
        .last_applied_chain_lock
        .as_ref()
        .map(|lock| lock.block_height);
    // Settle each stored context: a block under the restored chain lock is
    // final, and a persisted lock must upgrade its record here, because the
    // lock set already holds that txid and deduplicates a later lock event.
    for stored in &mut replay {
        stored.context = match &stored.context {
            TransactionContext::InBlock(block)
                if chain_locked_height.is_some_and(|height| block.height() <= height) =>
            {
                TransactionContext::InChainLockedBlock(*block)
            }
            TransactionContext::Mempool => instant_locks
                .get(&stored.txid)
                .filter(|lock| lock_matches_record(lock, stored))
                .map(|lock| TransactionContext::InstantSend(lock.clone()))
                .unwrap_or(TransactionContext::Mempool),
            context => context.clone(),
        };
    }
    let operands: Vec<ReplayOperands<'_>> = replay
        .iter()
        .map(|stored| {
            (
                stored.txid,
                &stored.transaction,
                stored.owned_inputs.as_slice(),
            )
        })
        .collect();
    let unsafe_txids = unsafe_replay_amounts(wallet_info, &operands);
    // Losers leave the replay and the projection; `swept` also keeps their
    // held raw records from coming back as fallbacks.
    let plan = plan_replay_sweeps(wallet_info, &replay, &unsafe_txids);
    tracing::debug!(
        wallet_id = %hex::encode(wallet_info.wallet_id),
        sweeps = plan.sweeps,
        examined = plan.examined,
        removed = plan.removed.len(),
        "load: settled stored transaction conflicts before the history replay"
    );
    let mut swept = plan.removed;
    replay.retain(|stored| !swept.contains(&stored.txid));
    let (replay, set_aside): (Vec<_>, Vec<_>) = replay
        .into_iter()
        .partition(|stored| !unsafe_txids.contains(&stored.txid));
    if !set_aside.is_empty() {
        let settled = set_aside
            .iter()
            .filter(|stored| stored.context.block_info().is_some())
            .count();
        // Counts only: the txids of unconfirmed records are peer-supplied.
        tracing::warn!(
            wallet_id = %hex::encode(wallet_info.wallet_id),
            skipped = set_aside.len(),
            "stored transaction records with unsafe amounts left out of the shared load replay; the coins they spend stay reserved"
        );
        if settled > 0 {
            tracing::error!(
                wallet_id = %hex::encode(wallet_info.wallet_id),
                settled,
                "stored confirmed transaction records carry amounts no valid transaction can; the wallet store is corrupt"
            );
        }
    }
    // The restored coins the records left out spend: they leave the UTXO set
    // below and are claimed for those records once the replay is over.
    let reserved_coins: HashSet<OutPoint> = set_aside
        .iter()
        .flat_map(|stored| &stored.transaction.input)
        .map(|input| input.previous_output)
        .filter(|outpoint| placed.contains_key(outpoint))
        .collect();
    for account in wallet_info.accounts.all_funding_accounts_mut() {
        account
            .utxos
            .retain(|outpoint, _| !swept.contains(&outpoint.txid));
    }
    stage_recorded_spent_inputs(wallet_info, &replay, &placed);

    for stored in &replay {
        let result = wallet_info
            .check_core_transaction(
                &stored.transaction,
                stored.context.clone(),
                wallet,
                true,
                false,
            )
            .await;
        swept.extend(result.swept_transactions);
        drop_fallback_copies(wallet_info, stored, &placed);
    }

    let spent: HashSet<_> = wallet_info
        .observed_spent_outpoints()
        .keys()
        .copied()
        .collect();
    // Replay credits an output to the account whose pool derives it. A
    // fallback copy `drop_fallback_copies` did not see credited by then is
    // dropped here, so each outpoint lives in exactly one account.
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
                && !reserved_coins.contains(outpoint)
                && !misplaced.contains(&(*outpoint, owner))
        });
        for (outpoint, coin) in &mut account.utxos {
            if let Some(persisted) = persisted.get(outpoint) {
                restore_persisted_coin_state(coin, persisted);
            }
        }
    }
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
            }
        }
    }
    // An unconfirmed record left out stays on record, as stored, in every
    // account holding a coin it spends or pays: a settled conflict, stored or
    // still to come, must find it there to sweep it and release its coins.
    for stored in &set_aside {
        if stored.context.block_info().is_some() {
            continue;
        }
        let outputs = (0..stored.transaction.output.len() as u32)
            .map(|vout| OutPoint::new(stored.txid, vout));
        let owners: HashSet<AccountType> = stored
            .transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .chain(outputs)
            .filter_map(|outpoint| placed.get(&outpoint).copied())
            .collect();
        for account in wallet_info.accounts.all_funding_accounts_mut() {
            let owner = funds_account_type(account);
            if owners.contains(&owner) {
                account
                    .transactions_mut()
                    .entry(stored.txid)
                    .or_insert_with(|| raw_record(stored, owner));
            }
        }
    }
    // A settled spend's first sweep cannot reach competitors replayed later,
    // nor the raw records restored above. The upstream sweep preserves
    // ChainLock/InstantSend precedence; it runs only for a spend something
    // unconfirmed still contests, because each sweep scans every live record.
    let competitors = unconfirmed_spends(wallet_info);
    for stored in &replay {
        if !matches!(stored.context, TransactionContext::Mempool)
            && is_contested(stored, &competitors)
        {
            let sweep = wallet_info.sweep_conflicts(&stored.transaction, &stored.context);
            swept.extend(sweep.txids);
        }
    }
    // Finalize replayed records before a sync checkpoint can prune their spend guards.
    if let Some(chain_lock) = wallet_info.metadata.last_applied_chain_lock.clone() {
        wallet_info.apply_chain_lock(chain_lock);
    }
    // A claim guards a coin only once it has left the UTXO set, and a sweep
    // or an abandon of its claimant releases the coin. One claimant per coin,
    // the smallest txid still on record: the wallet turns two different
    // claimants into a claim nothing releases.
    let mut claims: BTreeMap<OutPoint, Txid> = BTreeMap::new();
    for stored in set_aside
        .iter()
        .filter(|stored| !swept.contains(&stored.txid))
    {
        for input in &stored.transaction.input {
            if reserved_coins.contains(&input.previous_output) {
                claims
                    .entry(input.previous_output)
                    .and_modify(|claimant| *claimant = stored.txid.min(*claimant))
                    .or_insert(stored.txid);
            }
        }
    }
    let claims: Vec<(OutPoint, Option<Txid>)> = claims
        .into_iter()
        .map(|(coin, claimant)| (coin, Some(claimant)))
        .collect();
    wallet_info.restore_spent_outpoints(&claims);
    wallet_info.update_balance();
    swept.extend(set_aside.iter().map(|stored| stored.txid));
    HistoryReplay {
        replayed: replay.len(),
        retired: swept,
    }
}

/// One stored transaction as [`unsafe_replay_amounts`] reads it: its txid, its
/// body and the wallet-owned inputs its store recorded.
pub type ReplayOperands<'a> = (Txid, &'a Transaction, &'a [InputDetail]);

/// The stored transactions whose amounts the wallet checker cannot take.
///
/// The checker casts each unsigned operand to `i64` before a signed
/// subtraction, so an amount above `i64::MAX`, alone or summed, overflows it:
/// a panic where overflow is checked, a wrapped net amount where it is not. A
/// fitting net does not make an overflowing operand safe. Every source of an
/// operand is covered: the coins restored onto `wallet_info`, the outputs of
/// the other `records` (asset-lock credit outputs included) and the recorded
/// owned inputs.
pub fn unsafe_replay_amounts(
    wallet_info: &ManagedWalletInfo,
    records: &[ReplayOperands<'_>],
) -> HashSet<Txid> {
    /// Whether every value and their running sum fit `i64`.
    fn fits(values: impl IntoIterator<Item = u64>) -> bool {
        let mut sum = 0_u64;
        values.into_iter().all(|value| {
            sum = sum.saturating_add(value);
            sum <= i64::MAX as u64
        })
    }

    let mut amounts: HashMap<OutPoint, u64> = HashMap::new();
    let mut remember = |outpoint: OutPoint, value: u64| {
        let known = amounts.entry(outpoint).or_insert(value);
        *known = (*known).max(value);
    };
    for account in wallet_info.accounts.all_funding_accounts() {
        for (outpoint, coin) in &account.utxos {
            remember(*outpoint, coin.txout.value);
        }
    }
    let mut unsafe_txids = HashSet::new();
    for (txid, transaction, owned_inputs) in records {
        let credit_outputs = match &transaction.special_transaction_payload {
            Some(TransactionPayload::AssetLockPayloadType(payload)) => {
                payload.credit_outputs.as_slice()
            }
            _ => &[],
        };
        let outputs = transaction.output.iter().chain(credit_outputs);
        if !fits(owned_inputs.iter().map(|detail| detail.value))
            || !fits(outputs.map(|output| output.value))
        {
            // A record left out credits nothing, so its outputs are no child's input amount.
            unsafe_txids.insert(*txid);
            continue;
        }
        for (vout, output) in transaction.output.iter().enumerate() {
            remember(OutPoint::new(*txid, vout as u32), output.value);
        }
        for detail in *owned_inputs {
            if let Some(input) = transaction.input.get(detail.index as usize) {
                remember(input.previous_output, detail.value);
            }
        }
    }
    for (txid, transaction, _) in records {
        let inputs = transaction
            .input
            .iter()
            .filter_map(|input| amounts.get(&input.previous_output).copied());
        if !fits(inputs) {
            unsafe_txids.insert(*txid);
        }
    }
    unsafe_txids
}

/// What [`plan_replay_sweeps`] decided, and what deciding it cost.
#[derive(Debug, Default)]
struct SweepPlan {
    /// Every transaction a settled stored one beats to an input, with its
    /// unconfirmed descendants.
    removed: HashSet<Txid>,
    /// Conflict sweeps run.
    sweeps: usize,
    /// Records those sweeps had to look through, summed over the sweeps.
    examined: usize,
}

/// Resolve the full history before a defeated lock can sweep another spender.
///
/// Planning ahead of the replay is what keeps the losers out: replayed after
/// its parent was swept, a descendant that pays the wallet would be recorded
/// again, and no later sweep reaches it without the parent.
///
/// The upstream sweep decides every conflict, so finality precedence stays
/// its own. It scans the account it runs in whole, so each settled record's
/// sweep is handed only what it could remove: the unconfirmed records still
/// spending one of its inputs, and their unconfirmed descendants. A locked
/// competitor a plain block cannot remove is handed again to each such block.
///
/// `unsafe_txids` are planned on their inputs alone, like every other record.
fn plan_replay_sweeps(
    wallet_info: &ManagedWalletInfo,
    records: &[StoredTransaction],
    unsafe_txids: &HashSet<Txid>,
) -> SweepPlan {
    // The scratch wallet lends the sweep its rules and nothing to scan: its
    // unconfirmed records move to the pool, the rest and its coins are dropped.
    let mut scratch = wallet_info.clone();
    let mut pool: HashMap<Txid, TransactionRecord> = HashMap::new();
    for mut account in scratch.accounts.all_accounts_mut() {
        for (txid, record) in std::mem::take(account.transactions_mut()) {
            if !record.is_confirmed() {
                pool.entry(txid).or_insert(record);
            }
        }
    }
    for account in scratch.accounts.all_funding_accounts_mut() {
        account.utxos.clear();
    }
    // One account for every sweep lets it follow descendants across accounts.
    let Some(owner) = sweep_account(&mut scratch).map(|account| funds_account_type(account)) else {
        return SweepPlan::default();
    };
    for stored in records {
        if stored.context.block_info().is_none() {
            pool.insert(stored.txid, raw_record(stored, owner));
        }
    }
    // Who spends each outpoint, and who spends an output of each transaction.
    let mut spenders: HashMap<OutPoint, Vec<Txid>> = HashMap::new();
    let mut children: HashMap<Txid, Vec<Txid>> = HashMap::new();
    for (txid, record) in &pool {
        for input in &record.transaction.input {
            let spent = input.previous_output;
            spenders.entry(spent).or_default().push(*txid);
            children.entry(spent.txid).or_default().push(*txid);
        }
    }
    let mut winners: Vec<&StoredTransaction> = records
        .iter()
        .filter(|stored| !matches!(stored.context, TransactionContext::Mempool))
        // A record with unsafe amounts can lose a conflict, never win one.
        .filter(|stored| !unsafe_txids.contains(&stored.txid))
        .collect();
    winners.sort_by_key(|stored| {
        let priority = match stored.context {
            TransactionContext::InChainLockedBlock(_) => 0,
            TransactionContext::InstantSend(_) => 1,
            _ => 2,
        };
        (priority, stored.txid)
    });
    let mut plan = SweepPlan::default();
    for stored in winners {
        if plan.removed.contains(&stored.txid) {
            continue;
        }
        // A record leaves the pool when handed over, so one already removed
        // or already handed is skipped.
        let mut handed = BTreeMap::new();
        let mut queue: Vec<Txid> = stored
            .transaction
            .input
            .iter()
            .filter_map(|input| spenders.get(&input.previous_output))
            .flatten()
            .copied()
            .collect();
        while let Some(txid) = queue.pop() {
            if txid == stored.txid {
                continue;
            }
            if let Some(record) = pool.remove(&txid) {
                handed.insert(txid, record);
                queue.extend(children.get(&txid).into_iter().flatten());
            }
        }
        let Some(account) = sweep_account(&mut scratch).filter(|_| !handed.is_empty()) else {
            continue;
        };
        plan.sweeps += 1;
        plan.examined += handed.len();
        *account.transactions_mut() = handed;
        let sweep = scratch.sweep_conflicts(&stored.transaction, &stored.context);
        plan.removed.extend(sweep.txids);
        // Whatever the sweep left standing can still lose to a later winner.
        if let Some(account) = sweep_account(&mut scratch) {
            pool.extend(std::mem::take(account.transactions_mut()));
        }
    }
    plan
}

/// The funds account of a scratch wallet that planned sweeps run in.
fn sweep_account(scratch: &mut ManagedWalletInfo) -> Option<&mut ManagedCoreFundsAccount> {
    scratch
        .accounts
        .all_funding_accounts_mut()
        .into_iter()
        .next()
}

/// A history record holding only what the conflict sweep reads: the context
/// and the inputs. The amounts and details are filler.
fn raw_record(stored: &StoredTransaction, owner: AccountType) -> TransactionRecord {
    TransactionRecord::new(
        stored.transaction.clone(),
        owner,
        stored.context.clone(),
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        Vec::new(),
        Vec::new(),
        0,
    )
}

/// The unconfirmed transactions of `wallet_info` spending each outpoint. One
/// recorded in several accounts is still one spender.
fn unconfirmed_spends(wallet_info: &ManagedWalletInfo) -> HashMap<OutPoint, Vec<Txid>> {
    let mut seen = HashSet::new();
    let mut spends: HashMap<OutPoint, Vec<Txid>> = HashMap::new();
    for record in wallet_info
        .accounts
        .all_accounts()
        .into_iter()
        .flat_map(|account| account.transactions().values())
        .filter(|record| !record.is_confirmed())
    {
        if !seen.insert(record.txid) {
            continue;
        }
        for input in &record.transaction.input {
            spends
                .entry(input.previous_output)
                .or_default()
                .push(record.txid);
        }
    }
    spends
}

/// Whether an unconfirmed transaction other than `stored` itself still spends
/// one of its inputs: only then can a conflict sweep for it remove anything.
fn is_contested(stored: &StoredTransaction, competitors: &HashMap<OutPoint, Vec<Txid>>) -> bool {
    stored.transaction.input.iter().any(|input| {
        competitors
            .get(&input.previous_output)
            .is_some_and(|spenders| spenders.iter().any(|txid| *txid != stored.txid))
    })
}

/// Drop the projection's fallback copy of every output of `stored` that its
/// replay just credited to the account deriving it.
///
/// Left until the end of the replay, the copy lets a later record spend the
/// coin in both accounts: the fallback account records a spend of a coin it
/// never owned, and the checker's wallet total counts the coin twice.
fn drop_fallback_copies(
    wallet_info: &mut ManagedWalletInfo,
    stored: &StoredTransaction,
    placed: &HashMap<OutPoint, AccountType>,
) {
    for vout in 0..stored.transaction.output.len() as u32 {
        let outpoint = OutPoint::new(stored.txid, vout);
        let Some(parked) = placed.get(&outpoint) else {
            continue;
        };
        let mut accounts = wallet_info.accounts.all_funding_accounts_mut();
        let credited_elsewhere = accounts.iter().any(|account| {
            funds_account_type(account) != *parked && account.utxos.contains_key(&outpoint)
        });
        if credited_elsewhere {
            for account in &mut accounts {
                if funds_account_type(account) == *parked {
                    account.utxos.remove(&outpoint);
                }
            }
        }
    }
}

/// Give a coin the replay kept the state persistence stored on it.
///
/// The replay rebuilds a coin from its stored funding record, and the record
/// can know less than the coin: it keeps saying mempool after a height-only
/// confirmation, and a coin credited to another account than the one it was
/// parked in starts without its flags. The spend lock, the InstantSend status
/// and the trusted mark are restored as stored.
///
/// A stored confirmation is restored, with its height, only for a coin the
/// replay left unconfirmed. Where the replay confirmed the coin, the record's
/// block stands: the record names the block, the coin holds a bare height.
/// Nothing is raised beyond what was stored.
fn restore_persisted_coin_state(coin: &mut Utxo, persisted: &Utxo) {
    coin.is_locked |= persisted.is_locked;
    coin.is_instantlocked |= persisted.is_instantlocked;
    coin.is_trusted |= persisted.is_trusted;
    if persisted.is_confirmed && !coin.is_confirmed {
        coin.is_confirmed = true;
        coin.height = persisted.height;
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
) {
    let replayed: HashSet<Txid> = records.iter().map(|record| record.txid).collect();
    let mut accounts = wallet_info.accounts.all_funding_accounts_mut();
    for record in records {
        for detail in &record.owned_inputs {
            let Some(input) = record.transaction.input.get(detail.index as usize) else {
                continue;
            };
            let outpoint = input.previous_output;
            if placed.contains_key(&outpoint) || replayed.contains(&outpoint.txid) {
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
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// The test wallet, built from a fixed mnemonic: every txid a test derives
    /// from its addresses is the same in each run, so no test depends on how
    /// random txids happen to sort.
    fn test_wallet(accounts: WalletAccountCreationOptions) -> Wallet {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).unwrap();
        Wallet::from_mnemonic(mnemonic, Network::Testnet, accounts).unwrap()
    }

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

    /// The test wallet and its first BIP44 receive address.
    fn wallet_with_receive_address() -> (Wallet, ManagedWalletInfo, dashcore::Address) {
        let wallet = test_wallet(WalletAccountCreationOptions::Default);
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
        .await
        .replayed;
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
            .await
            .replayed;
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

    /// A wallet-paying child of a conflict loser must go with its parent
    /// wherever the InstantSend winner falls in the replay order. Between the
    /// two, the loser is already swept when the child replays, and the checker
    /// would rebuild the child from the output it pays the wallet.
    #[tokio::test]
    async fn should_drop_attributable_descendant_of_swept_conflict_for_any_replay_order() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let pay = |previous_output: OutPoint, outputs: &[(u64, bool)]| Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output,
                ..Default::default()
            }],
            output: outputs
                .iter()
                .map(|(value, ours)| TxOut {
                    value: *value,
                    script_pubkey: if *ours {
                        address.script_pubkey()
                    } else {
                        dashcore::ScriptBuf::new()
                    },
                })
                .collect(),
            special_transaction_payload: None,
        };
        let funding = pay(
            OutPoint::new(Txid::from_byte_array([51; 32]), 0),
            &[(100_000, true)],
        );
        let coin = OutPoint::new(funding.txid(), 0);
        let loser = pay(coin, &[(59_000, false), (40_000, true)]);
        let loser_change = OutPoint::new(loser.txid(), 1);
        let block = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([9; 32]),
            100,
        ));
        let funded = info
            .check_core_transaction(&funding, block, &mut wallet, true, true)
            .await
            .new_records;

        for position in ["first", "between", "last"] {
            // Unconfirmed records replay by txid, a child after its parent.
            let (winner, child) = (1..64_u64)
                .flat_map(|w| (1..64_u64).map(move |c| (w, c)))
                .map(|(w, c)| {
                    (
                        pay(coin, &[(68_000 - w, false), (30_000, true)]),
                        pay(loser_change, &[(39_000 - c, true)]),
                    )
                })
                .find(|(winner, child)| {
                    let (winner, loser, child) = (winner.txid(), loser.txid(), child.txid());
                    match position {
                        "first" => winner < loser,
                        "between" => loser < winner && winner < child,
                        _ => loser < winner && child < winner,
                    }
                })
                .unwrap();
            let child_output = OutPoint::new(child.txid(), 0);
            let winner_change = OutPoint::new(winner.txid(), 1);

            // Every spend as stored: unconfirmed, the winner's lock kept apart.
            let mut records = funded.clone();
            let mut losing = info.clone();
            for tx in [&loser, &child] {
                records.extend(
                    losing
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
            records.extend(
                info.clone()
                    .check_core_transaction(
                        &winner,
                        TransactionContext::Mempool,
                        &mut wallet,
                        true,
                        true,
                    )
                    .await
                    .new_records,
            );
            assert_eq!(records.len(), 4, "{position}");
            // Persisted as unspent: the winner's change and the child's output.
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            let account = restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap();
            for (outpoint, txout) in [
                (winner_change, &winner.output[1]),
                (child_output, &child.output[0]),
            ] {
                account.utxos.insert(
                    outpoint,
                    Utxo::new(outpoint, txout.clone(), address.clone(), 0, false),
                );
            }
            let lock = InstantLock {
                inputs: vec![coin],
                txid: winner.txid(),
                ..Default::default()
            };
            let locks = [(winner.txid(), lock)].into_iter().collect();

            replay_recorded_history(&mut restored, &mut wallet, history(records, locks)).await;

            let account = &restored.accounts.standard_bip44_accounts[&0];
            for (name, txid) in [("loser", loser.txid()), ("child", child.txid())] {
                assert!(
                    !account.transactions().contains_key(&txid),
                    "{position}: the {name} must be swept"
                );
            }
            assert!(
                !account.utxos.contains_key(&child_output),
                "{position}: the swept child's output must not stay selectable"
            );
            assert!(
                account.utxos.contains_key(&winner_change),
                "{position}: the winner's change stays"
            );
            assert_eq!(restored.balance.total(), 30_000, "{position}");
        }
    }

    /// A confirmed winner must also take a loser's descendant that pays a
    /// different account: the per-account conflict sweep reaches a descendant
    /// only through a parent recorded in the same account.
    #[tokio::test]
    async fn should_remove_cross_account_descendant_of_conflicting_spend() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::transaction_checking::BlockInfo;

        let mut wallet = test_wallet(WalletAccountCreationOptions::BIP44AccountsOnly(
            [0, 1].into_iter().collect(),
        ));
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let addresses: Vec<_> = [0, 1]
            .into_iter()
            .map(|index| {
                let xpub = wallet.accounts.standard_bip44_accounts[&index].account_xpub;
                info.accounts
                    .standard_bip44_accounts
                    .get_mut(&index)
                    .unwrap()
                    .next_receive_address(Some(&xpub), true)
                    .unwrap()
            })
            .collect();
        let pay = |previous_output: OutPoint, value: u64, script_pubkey| Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output,
                ..Default::default()
            }],
            output: vec![TxOut {
                value,
                script_pubkey,
            }],
            special_transaction_payload: None,
        };
        let funding = pay(
            OutPoint::new(Txid::from_byte_array([41; 32]), 0),
            100_000,
            addresses[0].script_pubkey(),
        );
        let coin = OutPoint::new(funding.txid(), 0);
        let root = pay(coin, 99_000, addresses[0].script_pubkey());
        let child = pay(
            OutPoint::new(root.txid(), 0),
            98_000,
            addresses[1].script_pubkey(),
        );
        let winner = pay(coin, 97_000, dashcore::ScriptBuf::new());
        let block = |height| {
            TransactionContext::InBlock(BlockInfo::new(
                height,
                BlockHash::from_byte_array([42; 32]),
                height,
            ))
        };
        let mut records = info
            .check_core_transaction(&funding, block(100), &mut wallet, true, true)
            .await
            .new_records;
        let mut winning = info.clone();
        for tx in [&root, &child] {
            records.extend(
                info.check_core_transaction(
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
        records.extend(
            winning
                .check_core_transaction(&winner, block(101), &mut wallet, true, true)
                .await
                .new_records,
        );
        crate::test_support::fold_wallet_records(&mut records);
        assert_eq!(records.len(), 4);

        // Persisted as unspent: only the child's output, in the other account.
        let child_output = OutPoint::new(child.txid(), 0);
        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
        restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&1)
            .unwrap()
            .utxos
            .insert(
                child_output,
                Utxo::new(
                    child_output,
                    child.output[0].clone(),
                    addresses[1].clone(),
                    0,
                    false,
                ),
            );

        replay_recorded_history(
            &mut restored,
            &mut wallet,
            history(records, BTreeMap::new()),
        )
        .await;

        for (index, account) in &restored.accounts.standard_bip44_accounts {
            for (name, txid) in [("root", root.txid()), ("child", child.txid())] {
                assert!(
                    !account.transactions().contains_key(&txid),
                    "account {index}: the conflicting {name} must be swept"
                );
            }
            assert!(
                !account.utxos.contains_key(&child_output),
                "account {index}: a conflicting descendant must not stay selectable"
            );
        }
        assert_eq!(restored.balance.total(), 0);
    }

    /// Stored amounts no valid transaction can carry must not reach the
    /// checker's signed arithmetic: the record is left out of the replay, and
    /// a restored coin it spends stays reserved instead of coming back
    /// spendable. Only an unconfirmed record holding such a coin stays on
    /// record, for a later conflict to find.
    #[tokio::test]
    async fn should_keep_unsafe_amounts_from_the_checker_and_reserve_the_coins_they_spend() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::managed_account::transaction_record::InputDetail;
        use key_wallet::transaction_checking::BlockInfo;

        const HALF: u64 = 1 << 62;
        let block = |height| {
            TransactionContext::InBlock(BlockInfo::new(
                height,
                BlockHash::from_byte_array([height as u8; 32]),
                height,
            ))
        };
        for (case, outputs, staged, settled) in [
            ("single output", vec![2 * HALF], None, false),
            ("summed outputs", vec![HALF, HALF], None, false),
            ("staged input", vec![], Some(2 * HALF), false),
            ("settled record", vec![2 * HALF], None, true),
        ] {
            let (mut wallet, mut info, address) = wallet_with_receive_address();
            let pay = |previous_output: OutPoint, values: &[u64]| Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output,
                    ..Default::default()
                }],
                output: values
                    .iter()
                    .map(|value| TxOut {
                        value: *value,
                        script_pubkey: address.script_pubkey(),
                    })
                    .collect(),
                special_transaction_payload: None,
            };
            let funding = pay(OutPoint::new(Txid::from_byte_array([61; 32]), 0), &[1]);
            let coin = OutPoint::new(funding.txid(), 0);
            let funded = info
                .check_core_transaction(&funding, block(100), &mut wallet, true, true)
                .await
                .new_records;
            // A staged input stands for a funding the store holds no record of.
            let spent = if staged.is_some() {
                OutPoint::new(Txid::from_byte_array([62; 32]), 0)
            } else {
                coin
            };
            let unsafe_spend = pay(spent, &outputs);
            let mut stored: Vec<StoredTransaction> =
                funded.into_iter().map(StoredTransaction::from).collect();
            stored.push(StoredTransaction {
                txid: unsafe_spend.txid(),
                transaction: unsafe_spend.clone(),
                context: if settled {
                    block(101)
                } else {
                    TransactionContext::Mempool
                },
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: staged
                    .map(|value| InputDetail {
                        index: 0,
                        value,
                        address: address.clone(),
                    })
                    .into_iter()
                    .collect(),
            });
            // An unconfirmed spend never flips the coin's persisted spent flag.
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .utxos
                .insert(
                    coin,
                    Utxo::new(coin, funding.output[0].clone(), address.clone(), 100, false),
                );

            let replayed = replay_recorded_history(
                &mut restored,
                &mut wallet,
                RecordedHistory {
                    transactions: stored,
                    instant_locks: BTreeMap::new(),
                },
            )
            .await
            .replayed;
            restored
                .check_core_transaction(&funding, block(100), &mut wallet, true, true)
                .await;

            assert_eq!(replayed, 1, "{case}: only the funding is replayed");
            let account = &restored.accounts.standard_bip44_accounts[&0];
            assert_eq!(
                account.transactions().contains_key(&unsafe_spend.txid()),
                !settled && staged.is_none(),
                "{case}: only an unconfirmed record reserving a restored coin stays on record"
            );
            assert_eq!(
                account.utxos.contains_key(&coin),
                staged.is_some(),
                "{case}: a coin the unsafe record spends stays reserved"
            );
            assert_eq!(
                restored.balance.total(),
                u64::from(staged.is_some()),
                "{case}"
            );
        }
    }

    /// An unconfirmed record with unsafe amounts that spends coins A and B,
    /// the confirmed winner that beats it on A alone, and B's funding, which
    /// the store holds no record of. The projection still holds both coins.
    async fn unsafe_loser_fixture() -> (
        Wallet,
        ManagedWalletInfo,
        Vec<StoredTransaction>,
        StoredTransaction,
        dashcore::Transaction,
        [OutPoint; 2],
    ) {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let spend = |spends: &[OutPoint], value: u64, script_pubkey| Transaction {
            version: 1,
            lock_time: 0,
            input: spends
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value,
                script_pubkey,
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
        let fund = |byte, value| {
            let unknown = OutPoint::new(Txid::from_byte_array([byte; 32]), 0);
            spend(&[unknown], value, address.script_pubkey())
        };
        let (funding_a, funding_b) = (fund(71, 10_000), fund(72, 20_000));
        let coins = [funding_a.txid(), funding_b.txid()].map(|txid| OutPoint::new(txid, 0));
        let stored = |transaction: Transaction, context| StoredTransaction {
            txid: transaction.txid(),
            transaction,
            context,
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: Vec::new(),
        };
        let mut history: Vec<StoredTransaction> = info
            .check_core_transaction(&funding_a, block(100), &mut wallet, true, true)
            .await
            .new_records
            .into_iter()
            .map(StoredTransaction::from)
            .collect();
        history.push(stored(
            spend(&coins, 1 << 63, address.script_pubkey()),
            TransactionContext::Mempool,
        ));
        let winner = stored(
            spend(&coins[..1], 9_000, dashcore::ScriptBuf::new()),
            block(101),
        );

        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
        let account = restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap();
        for (coin, funding) in coins.iter().zip([&funding_a, &funding_b]) {
            account.utxos.insert(
                *coin,
                Utxo::new(*coin, funding.output[0].clone(), address.clone(), 99, false),
            );
        }
        (wallet, restored, history, winner, funding_b, coins)
    }

    /// A stored confirmed winner settles an unsafe unconfirmed loser like any
    /// other: the loser is gone and the coin only the loser spent is free.
    #[tokio::test]
    async fn should_release_other_input_of_unsafe_loser_beaten_by_stored_winner() {
        let (mut wallet, mut restored, mut history, winner, _, [coin_a, coin_b]) =
            unsafe_loser_fixture().await;
        let loser = history[1].txid;
        history.push(winner);

        replay_recorded_history(
            &mut restored,
            &mut wallet,
            RecordedHistory {
                transactions: history,
                instant_locks: BTreeMap::new(),
            },
        )
        .await;

        let account = &restored.accounts.standard_bip44_accounts[&0];
        assert!(!account.transactions().contains_key(&loser));
        assert!(
            !account.utxos.contains_key(&coin_a),
            "the winner spent this coin"
        );
        assert!(
            account.utxos.contains_key(&coin_b),
            "the coin only the swept loser spent must be spendable again"
        );
        assert_eq!(restored.balance.total(), 20_000);
    }

    /// With no winner stored, the unsafe unconfirmed record keeps both coins
    /// reserved and stays on record, so a winner delivered after the load
    /// sweeps it and releases the coin it does not spend itself.
    #[tokio::test]
    async fn should_release_other_input_of_unsafe_loser_beaten_after_load() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut restored, history, winner, funding_b, [coin_a, coin_b]) =
            unsafe_loser_fixture().await;
        let loser = history[1].txid;
        let funded_b = TransactionContext::InBlock(BlockInfo::new(
            99,
            BlockHash::from_byte_array([99; 32]),
            99,
        ));

        replay_recorded_history(
            &mut restored,
            &mut wallet,
            RecordedHistory {
                transactions: history,
                instant_locks: BTreeMap::new(),
            },
        )
        .await;

        let account = &restored.accounts.standard_bip44_accounts[&0];
        assert!(account.transactions().contains_key(&loser));
        assert!(!account.utxos.contains_key(&coin_a) && !account.utxos.contains_key(&coin_b));
        assert_eq!(restored.balance.total(), 0);
        // Control: while the record stands, its reservation holds.
        let mut reserved = restored.clone();
        reserved
            .check_core_transaction(&funding_b, funded_b.clone(), &mut wallet, true, true)
            .await;
        assert!(
            !reserved.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&coin_b),
            "a coin the unsafe record spends must stay reserved"
        );

        let settled = restored
            .check_core_transaction(
                &winner.transaction,
                winner.context.clone(),
                &mut wallet,
                true,
                true,
            )
            .await;
        assert_eq!(settled.swept_transactions, vec![loser]);
        assert_eq!(settled.released_outpoints, vec![coin_b]);
        restored
            .check_core_transaction(&funding_b, funded_b, &mut wallet, true, true)
            .await;

        let account = &restored.accounts.standard_bip44_accounts[&0];
        assert!(!account.transactions().contains_key(&loser));
        assert!(
            !account.utxos.contains_key(&coin_a),
            "the winner spent this coin"
        );
        assert!(
            account.utxos.contains_key(&coin_b),
            "the released coin must be credited when its funding is seen"
        );
        assert_eq!(restored.balance.total(), 20_000);
    }

    /// Planning must not run one whole-history sweep per settled record that
    /// merely shares an input with an unconfirmed one: a sweep is worth
    /// running only while a competitor it could remove is still there, and it
    /// looks only at the records it could remove.
    #[test]
    fn should_plan_sweeps_only_while_a_competitor_remains() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        const SPENDERS: u32 = 200;
        let (_wallet, info, address) = wallet_with_receive_address();
        let outpoint = |index: u32| {
            let mut bytes = [7_u8; 32];
            bytes[..4].copy_from_slice(&index.to_le_bytes());
            OutPoint::new(Txid::from_byte_array(bytes), 0)
        };
        let stored = |spends: Vec<u32>, lock_time, context| {
            let transaction = Transaction {
                version: 1,
                lock_time,
                input: spends
                    .into_iter()
                    .map(|index| TxIn {
                        previous_output: outpoint(index),
                        ..Default::default()
                    })
                    .collect(),
                output: vec![TxOut {
                    value: 1_000,
                    script_pubkey: address.script_pubkey(),
                }],
                special_transaction_payload: None,
            };
            StoredTransaction {
                txid: transaction.txid(),
                transaction,
                context,
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: Vec::new(),
            }
        };
        let confirmed: Vec<StoredTransaction> = (0..SPENDERS)
            .map(|index| {
                let block = BlockInfo::new(100 + index, BlockHash::from_byte_array([1; 32]), 0);
                stored(vec![index], index, TransactionContext::InBlock(block))
            })
            .collect();

        // One unconfirmed record contests every confirmed spender at once.
        let greedy = stored((0..SPENDERS).collect(), 0, TransactionContext::Mempool);
        let mut records = confirmed.clone();
        records.push(greedy.clone());
        let plan = plan_replay_sweeps(&info, &records, &HashSet::new());
        assert_eq!(plan.removed, HashSet::from([greedy.txid]));
        assert_eq!(
            plan.sweeps, 1,
            "the competitor is gone after the first sweep"
        );

        // Each confirmed spender has a competitor of its own, next to
        // unconfirmed records nothing contests.
        let competitors: Vec<StoredTransaction> = (0..SPENDERS)
            .map(|index| stored(vec![index], SPENDERS + index, TransactionContext::Mempool))
            .collect();
        let bystanders: Vec<StoredTransaction> = (0..SPENDERS)
            .map(|index| {
                stored(
                    vec![SPENDERS + index],
                    2 * SPENDERS + index,
                    TransactionContext::Mempool,
                )
            })
            .collect();
        let records: Vec<StoredTransaction> =
            [&confirmed[..], &competitors[..], &bystanders[..]].concat();
        let plan = plan_replay_sweeps(&info, &records, &HashSet::new());
        assert_eq!(
            plan.removed,
            competitors.iter().map(|stored| stored.txid).collect()
        );
        assert_eq!(plan.sweeps, SPENDERS as usize);
        assert_eq!(
            plan.examined, SPENDERS as usize,
            "each sweep looks at its own competitor, never at the bystanders"
        );
    }

    /// The host lists its unconfirmed sends for re-dispatch from its own rows,
    /// so the list can name transactions the replay retired. Those must not
    /// go out again; every other send must, stored or not.
    #[tokio::test]
    async fn should_withhold_retired_sends_from_re_dispatch() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, _, address) = wallet_with_receive_address();
        let coin = |byte: u8| OutPoint::new(Txid::from_byte_array([byte; 32]), 0);
        let spend = |spends: &[OutPoint], value: u64| Transaction {
            version: 1,
            lock_time: 0,
            input: spends
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        // The winner took coin 1 in a block; the loser wanted coins 1 and 2.
        let winner = spend(&[coin(1)], 9_000);
        let loser = spend(&[coin(1), coin(2)], 29_000);
        let descendant = spend(&[OutPoint::new(loser.txid(), 0)], 28_000);
        let survivor = spend(&[coin(3)], 39_000);
        let unsafe_send = spend(&[coin(4)], 1 << 63);
        let unrecorded = spend(&[coin(5)], 59_000);
        let stored = |transaction: &Transaction, context| StoredTransaction {
            txid: transaction.txid(),
            transaction: transaction.clone(),
            context,
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: Vec::new(),
        };
        let block = BlockInfo::new(101, BlockHash::from_byte_array([101; 32]), 101);
        let history = RecordedHistory {
            transactions: vec![
                stored(&winner, TransactionContext::InBlock(block)),
                stored(&loser, TransactionContext::Mempool),
                stored(&descendant, TransactionContext::Mempool),
                stored(&survivor, TransactionContext::Mempool),
                stored(&unsafe_send, TransactionContext::Mempool),
            ],
            instant_locks: BTreeMap::new(),
        };
        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
        let account = restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap();
        // Persisted as unspent: every coin but the one the block spent.
        for (byte, value) in [(2, 20_000), (3, 40_000), (4, 50_000), (5, 60_000)] {
            let txout = TxOut {
                value,
                script_pubkey: address.script_pubkey(),
            };
            account.utxos.insert(
                coin(byte),
                Utxo::new(coin(byte), txout, address.clone(), 99, false),
            );
        }

        let outcome = replay_recorded_history(&mut restored, &mut wallet, history).await;

        for (retired, why) in [
            (&loser, "the loser of a settled conflict"),
            (&descendant, "a descendant of that loser"),
        ] {
            assert!(outcome.retired.contains(&retired.txid()), "{why}");
        }
        assert!(
            outcome.retired.contains(&unsafe_send.txid()),
            "a record with unsafe amounts must not reach the checker by re-dispatch either"
        );
        let sends = vec![
            loser.clone(),
            descendant.clone(),
            survivor.clone(),
            unsafe_send.clone(),
            unrecorded.clone(),
        ];
        let resendable: Vec<Txid> = outcome
            .resendable(sends)
            .iter()
            .map(Transaction::txid)
            .collect();
        assert_eq!(
            resendable,
            vec![survivor.txid(), unrecorded.txid()],
            "a surviving stored send and one the history never held still go out"
        );
    }

    /// Planning by whole-history sweeps, kept as the reference the narrowed
    /// planner must agree with: every unconfirmed stored record sits in the
    /// scratch account, and every settled record sweeps all of it.
    fn plan_by_whole_history_sweeps(
        wallet_info: &ManagedWalletInfo,
        records: &[StoredTransaction],
        unsafe_txids: &HashSet<Txid>,
    ) -> HashSet<Txid> {
        let mut conflicts = wallet_info.clone();
        let account = conflicts
            .accounts
            .all_funding_accounts_mut()
            .into_iter()
            .next()
            .unwrap();
        let owner = funds_account_type(account);
        account.transactions_mut().extend(
            records
                .iter()
                .filter(|stored| stored.context.block_info().is_none())
                .map(|stored| (stored.txid, raw_record(stored, owner))),
        );
        let mut winners: Vec<&StoredTransaction> = records
            .iter()
            .filter(|stored| !matches!(stored.context, TransactionContext::Mempool))
            .filter(|stored| !unsafe_txids.contains(&stored.txid))
            .collect();
        winners.sort_by_key(|stored| {
            let priority = match stored.context {
                TransactionContext::InChainLockedBlock(_) => 0,
                TransactionContext::InstantSend(_) => 1,
                _ => 2,
            };
            (priority, stored.txid)
        });
        let mut removed = HashSet::new();
        for stored in winners {
            if !removed.contains(&stored.txid) {
                let sweep = conflicts.sweep_conflicts(&stored.transaction, &stored.context);
                removed.extend(sweep.txids);
            }
        }
        removed
    }

    /// Handing each sweep only the records it could remove must not change
    /// what planning decides. Compared against whole-history sweeps on a
    /// fixed InstantSend shape and on generated histories mixing winners,
    /// competitors, descendants, bystanders and locked competitors.
    #[test]
    fn should_plan_the_same_removals_as_whole_history_sweeps() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let (_wallet, info, address) = wallet_with_receive_address();
        let coin = |index: u8| OutPoint::new(Txid::from_byte_array([index; 32]), 0);
        let block = |height| BlockInfo::new(height, BlockHash::from_byte_array([1; 32]), 0);
        let stored = |spends: Vec<OutPoint>, lock_time: u32, settlement: u8| {
            let transaction = Transaction {
                version: 1,
                lock_time,
                input: spends
                    .iter()
                    .map(|previous_output| TxIn {
                        previous_output: *previous_output,
                        ..Default::default()
                    })
                    .collect(),
                output: vec![TxOut {
                    value: 1_000,
                    script_pubkey: address.script_pubkey(),
                }],
                special_transaction_payload: None,
            };
            let txid = transaction.txid();
            let context = match settlement {
                0 => TransactionContext::Mempool,
                1 => TransactionContext::InstantSend(InstantLock {
                    txid,
                    inputs: spends,
                    ..Default::default()
                }),
                2 => TransactionContext::InBlock(block(100 + lock_time)),
                _ => TransactionContext::InChainLockedBlock(block(100 + lock_time)),
            };
            StoredTransaction {
                txid,
                transaction,
                context,
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: Vec::new(),
            }
        };

        // A plain block cannot overrule a lock; a chain lock can, and takes
        // the locked competitor's unconfirmed child with it.
        let competitor = stored(vec![coin(1)], 1, 1);
        let child = stored(vec![OutPoint::new(competitor.txid, 0)], 2, 0);
        for (settlement, expected) in [
            (2, HashSet::new()),
            (3, HashSet::from([competitor.txid, child.txid])),
        ] {
            let records = [
                stored(vec![coin(1)], 0, settlement),
                competitor.clone(),
                child.clone(),
            ];
            let plan = plan_replay_sweeps(&info, &records, &HashSet::new());
            assert_eq!(plan.removed, expected, "winner settlement {settlement}");
            assert_eq!(
                plan_by_whole_history_sweeps(&info, &records, &HashSet::new()),
                expected
            );
        }

        // How many generated histories held each shape, so none goes untested.
        let (mut swept, mut swept_descendant, mut locked_survivor, mut bystander) = (0, 0, 0, 0);
        for seed in 0..500 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut records: Vec<StoredTransaction> = Vec::new();
            for index in 0..rng.gen_range(2..=14) {
                let spends: BTreeSet<OutPoint> = (0..rng.gen_range(1..=2))
                    .map(|_| {
                        if !records.is_empty() && rng.gen_bool(0.35) {
                            OutPoint::new(records[rng.gen_range(0..records.len())].txid, 0)
                        } else {
                            coin(rng.gen_range(0..6))
                        }
                    })
                    .collect();
                let settlement = match rng.gen_range(0..10) {
                    0..=4 => 0,
                    5..=6 => 1,
                    7..=8 => 2,
                    _ => 3,
                };
                records.push(stored(spends.into_iter().collect(), index, settlement));
            }
            let unsafe_txids: HashSet<Txid> = records
                .iter()
                .filter(|_| rng.gen_bool(0.1))
                .map(|stored| stored.txid)
                .collect();

            let expected = plan_by_whole_history_sweeps(&info, &records, &unsafe_txids);
            let plan = plan_replay_sweeps(&info, &records, &unsafe_txids);
            assert_eq!(plan.removed, expected, "seed {seed}");

            let shares_input = |a: &StoredTransaction, b: &StoredTransaction| {
                a.txid != b.txid
                    && a.transaction.input.iter().any(|input| {
                        b.transaction
                            .input
                            .iter()
                            .any(|other| other.previous_output == input.previous_output)
                    })
            };
            swept += usize::from(!expected.is_empty());
            swept_descendant += usize::from(records.iter().any(|stored| {
                expected.contains(&stored.txid)
                    && stored
                        .transaction
                        .input
                        .iter()
                        .any(|input| expected.contains(&input.previous_output.txid))
            }));
            locked_survivor += usize::from(records.iter().any(|locked| {
                matches!(locked.context, TransactionContext::InstantSend(_))
                    && !expected.contains(&locked.txid)
                    && records.iter().any(|winner| {
                        matches!(winner.context, TransactionContext::InBlock(_))
                            && !unsafe_txids.contains(&winner.txid)
                            && shares_input(winner, locked)
                    })
            }));
            bystander += usize::from(records.iter().any(|stored| {
                stored.context.block_info().is_none()
                    && !expected.contains(&stored.txid)
                    && !records.iter().any(|other| shares_input(other, stored))
            }));
        }
        for (shape, histories) in [
            ("a swept competitor", swept),
            ("a swept descendant", swept_descendant),
            (
                "a locked competitor surviving a plain block",
                locked_survivor,
            ),
            ("an uncontested bystander", bystander),
        ] {
            assert!(histories >= 25, "only {histories} histories held {shape}");
        }
    }

    /// A wallet with BIP44 accounts 0 and 1 and a receive address of each.
    fn two_account_wallet() -> (Wallet, ManagedWalletInfo, [dashcore::Address; 2]) {
        let wallet = test_wallet(WalletAccountCreationOptions::BIP44AccountsOnly(
            [0, 1].into_iter().collect(),
        ));
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let addresses = [0, 1].map(|index| {
            let xpub = wallet.accounts.standard_bip44_accounts[&index].account_xpub;
            info.accounts
                .standard_bip44_accounts
                .get_mut(&index)
                .unwrap()
                .next_receive_address(Some(&xpub), true)
                .unwrap()
        });
        (wallet, info, addresses)
    }

    /// The projection can park a coin in a fallback account while its funding,
    /// replayed, credits it to the account that derives it. A stored spend of
    /// that coin must still count it once: summed over both copies, the
    /// checker's wallet total doubles and a large coin overflows it.
    #[tokio::test]
    async fn should_spend_coin_parked_in_fallback_account_once() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        for value in [1_u64 << 62, 100_000] {
            let (mut wallet, restored, addresses) = two_account_wallet();
            let pay = |previous_output: OutPoint, value: u64| Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output,
                    ..Default::default()
                }],
                output: vec![TxOut {
                    value,
                    script_pubkey: addresses[1].script_pubkey(),
                }],
                special_transaction_payload: None,
            };
            let funding = pay(OutPoint::new(Txid::from_byte_array([81; 32]), 0), value);
            let coin = OutPoint::new(funding.txid(), 0);
            let child = pay(coin, 1);
            let stored = |transaction: Transaction, context| StoredTransaction {
                txid: transaction.txid(),
                transaction,
                context,
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: Vec::new(),
            };
            let block = BlockInfo::new(100, BlockHash::from_byte_array([82; 32]), 100);
            let history = RecordedHistory {
                transactions: vec![
                    stored(funding.clone(), TransactionContext::InBlock(block)),
                    stored(child.clone(), TransactionContext::Mempool),
                ],
                instant_locks: BTreeMap::new(),
            };
            let mut restored = restored;
            // Parked in account 0, though account 1 derives its address.
            restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .utxos
                .insert(
                    coin,
                    Utxo::new(
                        coin,
                        funding.output[0].clone(),
                        addresses[1].clone(),
                        100,
                        false,
                    ),
                );

            replay_recorded_history(&mut restored, &mut wallet, history).await;

            let [fallback, owner] =
                [0, 1].map(|index| &restored.accounts.standard_bip44_accounts[&index]);
            assert!(
                !fallback.utxos.contains_key(&coin) && !owner.utxos.contains_key(&coin),
                "{value}: the stored spend took the coin"
            );
            assert!(
                !fallback.transactions().contains_key(&child.txid()),
                "{value}: the fallback account never owned the coin the spend took"
            );
            assert_eq!(
                owner.transactions()[&child.txid()].net_amount,
                1 - value as i64,
                "{value}: the spend counts the coin once"
            );
        }
    }

    /// Two unconfirmed records with unsafe amounts that both spend one shared
    /// coin, each next to a coin of its own, replayed onto a projection
    /// holding all three coins.
    ///
    /// Returns the wallet after the replay, the two records ordered by txid,
    /// the shared coin with its funding, and each record's own coin.
    async fn overlapping_unsafe_spends_fixture() -> (
        Wallet,
        ManagedWalletInfo,
        [dashcore::Transaction; 2],
        (OutPoint, dashcore::Transaction),
        [OutPoint; 2],
    ) {
        use dashcore::hashes::Hash;
        use dashcore::{Transaction, TxIn};

        let (mut wallet, _, address) = wallet_with_receive_address();
        let spend = |spends: &[OutPoint], value: u64| Transaction {
            version: 1,
            lock_time: 0,
            input: spends
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value,
                script_pubkey: address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let fundings = [(85, 10_000), (86, 20_000), (87, 30_000)].map(|(byte, value)| {
            spend(
                &[OutPoint::new(Txid::from_byte_array([byte; 32]), 0)],
                value,
            )
        });
        let [shared, one, other] = [0, 1, 2].map(|index| OutPoint::new(fundings[index].txid(), 0));
        let mut spends = [
            (spend(&[shared, one], 1 << 63), one),
            (spend(&[shared, other], 1 << 63), other),
        ];
        spends.sort_by_key(|(spend, _)| spend.txid());
        let own = [spends[0].1, spends[1].1];
        let unsafe_spends = spends.map(|(spend, _)| spend);

        let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
        let account = restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap();
        for (coin, funding) in [shared, one, other].iter().zip(&fundings) {
            account.utxos.insert(
                *coin,
                Utxo::new(*coin, funding.output[0].clone(), address.clone(), 99, false),
            );
        }
        let history = RecordedHistory {
            transactions: unsafe_spends
                .iter()
                .map(|transaction| StoredTransaction {
                    txid: transaction.txid(),
                    transaction: transaction.clone(),
                    context: TransactionContext::Mempool,
                    stored_net_amount: None,
                    stored_direction: None,
                    owned_inputs: Vec::new(),
                })
                .collect(),
            instant_locks: BTreeMap::new(),
        };
        replay_recorded_history(&mut restored, &mut wallet, history).await;
        assert_eq!(
            restored.balance.total(),
            0,
            "every coin an unsafe record spends is reserved"
        );
        let [funding, _, _] = fundings;
        (wallet, restored, unsafe_spends, (shared, funding), own)
    }

    /// A confirmed spend of `coins`, delivered to `info` after the load.
    async fn deliver_confirmed_spend(
        info: &mut ManagedWalletInfo,
        wallet: &mut Wallet,
        coins: &[OutPoint],
    ) -> key_wallet::transaction_checking::TransactionCheckResult {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        let winner = Transaction {
            version: 1,
            lock_time: 0,
            input: coins
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value: 1_000,
                script_pubkey: dashcore::ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let block = BlockInfo::new(101, BlockHash::from_byte_array([101; 32]), 101);
        info.check_core_transaction(
            &winner,
            TransactionContext::InBlock(block),
            wallet,
            true,
            true,
        )
        .await
    }

    /// Whether a redelivery of `funding` credits `coin` to the wallet.
    async fn redelivery_credits(
        info: &ManagedWalletInfo,
        wallet: &mut Wallet,
        funding: &dashcore::Transaction,
        coin: &OutPoint,
    ) -> bool {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::transaction_checking::BlockInfo;

        let mut info = info.clone();
        let block = BlockInfo::new(99, BlockHash::from_byte_array([99; 32]), 99);
        info.check_core_transaction(
            funding,
            TransactionContext::InBlock(block),
            wallet,
            true,
            true,
        )
        .await;
        info.accounts.standard_bip44_accounts[&0]
            .utxos
            .contains_key(coin)
    }

    /// Two records left out for unsafe amounts can spend the same restored
    /// coin. Once a settled transaction has swept both, nothing spends the
    /// coin any more: it must be released and credited when its funding is
    /// seen again.
    #[tokio::test]
    async fn should_release_coin_shared_by_two_unsafe_records_once_both_are_swept() {
        let (mut wallet, mut restored, unsafe_spends, (shared, funding), own) =
            overlapping_unsafe_spends_fixture().await;
        assert!(!redelivery_credits(&restored, &mut wallet, &funding, &shared).await);

        let settled = deliver_confirmed_spend(&mut restored, &mut wallet, &own).await;

        let mut swept = settled.swept_transactions.clone();
        swept.sort_unstable();
        assert_eq!(swept, unsafe_spends.map(|spend| spend.txid()));
        assert_eq!(
            settled.released_outpoints,
            vec![shared],
            "the coin only the swept records spent must be released"
        );
        assert!(
            redelivery_credits(&restored, &mut wallet, &funding, &shared).await,
            "the released coin must be credited when its funding is seen"
        );
    }

    /// The wallet can name one claimant per reserved coin, so a shared coin
    /// is held for the record with the smaller txid. Accepted consequence:
    /// sweeping that record alone drops the guard although the other record
    /// still names the coin, and sweeping the other one alone drops nothing.
    /// Neither is reported as a release while a record on file spends the
    /// coin. A record with unsafe amounts can never confirm, so neither
    /// costs funds.
    #[tokio::test]
    async fn should_hold_shared_coin_for_the_unsafe_record_with_the_smaller_txid() {
        for (swept, drops_guard) in [(0, true), (1, false)] {
            let (mut wallet, mut restored, unsafe_spends, (shared, funding), own) =
                overlapping_unsafe_spends_fixture().await;

            let settled =
                deliver_confirmed_spend(&mut restored, &mut wallet, &own[swept..=swept]).await;

            assert_eq!(
                settled.swept_transactions,
                vec![unsafe_spends[swept].txid()]
            );
            assert_eq!(
                settled.released_outpoints,
                vec![],
                "record {swept} swept alone: the other record still spends the coin"
            );
            assert_eq!(
                redelivery_credits(&restored, &mut wallet, &funding, &shared).await,
                drops_guard,
                "record {swept} swept alone"
            );
        }
    }

    /// A coin the replay moves from its fallback account to the account that
    /// derives it keeps what persistence stored on it: the spend lock and the
    /// InstantSend status.
    #[tokio::test]
    async fn should_keep_restored_flags_of_coin_moved_from_fallback_account() {
        use dashcore::hashes::Hash;
        use dashcore::{Transaction, TxIn};

        let (mut wallet, mut restored, addresses) = two_account_wallet();
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([88; 32]), 0),
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 100_000,
                script_pubkey: addresses[1].script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let coin = OutPoint::new(funding.txid(), 0);
        let mut parked = Utxo::new(
            coin,
            funding.output[0].clone(),
            addresses[1].clone(),
            0,
            false,
        );
        parked.is_locked = true;
        parked.is_instantlocked = true;
        // Parked in account 0, though account 1 derives its address.
        restored
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .utxos
            .insert(coin, parked);
        let history = RecordedHistory {
            transactions: vec![StoredTransaction {
                txid: funding.txid(),
                transaction: funding,
                context: TransactionContext::Mempool,
                stored_net_amount: None,
                stored_direction: None,
                owned_inputs: Vec::new(),
            }],
            instant_locks: BTreeMap::new(),
        };

        replay_recorded_history(&mut restored, &mut wallet, history).await;

        let [fallback, owner] =
            [0, 1].map(|index| &restored.accounts.standard_bip44_accounts[&index]);
        assert!(!fallback.utxos.contains_key(&coin));
        let moved = &owner.utxos[&coin];
        assert!(moved.is_locked, "a locked coin must stay locked");
        assert!(
            moved.is_instantlocked,
            "an InstantSend-locked coin must stay so"
        );
    }

    /// The replay rebuilds a coin from its stored funding record, which can
    /// be older than the coin: a height-only confirmation never rewrites the
    /// record. The confirmation persistence stored on the coin must survive,
    /// nothing may be promoted beyond it, and where the replay itself
    /// confirmed the coin, the record's block stands.
    #[tokio::test]
    async fn should_restore_only_the_confirmation_persistence_stored_on_a_coin() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        let in_block = |height| {
            TransactionContext::InBlock(BlockInfo::new(
                height,
                BlockHash::from_byte_array([height as u8; 32]),
                height,
            ))
        };
        // (case, persisted confirmation, stored record, expected confirmation)
        for (case, persisted, recorded, expected) in [
            (
                "stale mempool record",
                Some(99),
                TransactionContext::Mempool,
                Some(99),
            ),
            ("unconfirmed coin", None, TransactionContext::Mempool, None),
            (
                "record names another block",
                Some(90),
                in_block(100),
                Some(100),
            ),
        ] {
            let (mut wallet, _, address) = wallet_with_receive_address();
            let funding = Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(Txid::from_byte_array([89; 32]), 0),
                    ..Default::default()
                }],
                output: vec![TxOut {
                    value: 100_000,
                    script_pubkey: address.script_pubkey(),
                }],
                special_transaction_payload: None,
            };
            let coin = OutPoint::new(funding.txid(), 0);
            let mut stored_coin = Utxo::new(
                coin,
                funding.output[0].clone(),
                address.clone(),
                persisted.unwrap_or(0),
                false,
            );
            stored_coin.is_confirmed = persisted.is_some();
            let mut restored = ManagedWalletInfo::from_wallet(&wallet, 0);
            restored
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap()
                .utxos
                .insert(coin, stored_coin);
            let history = RecordedHistory {
                transactions: vec![StoredTransaction {
                    txid: funding.txid(),
                    transaction: funding,
                    context: recorded,
                    stored_net_amount: None,
                    stored_direction: None,
                    owned_inputs: Vec::new(),
                }],
                instant_locks: BTreeMap::new(),
            };

            replay_recorded_history(&mut restored, &mut wallet, history).await;

            let kept = &restored.accounts.standard_bip44_accounts[&0].utxos[&coin];
            assert_eq!(kept.is_confirmed.then_some(kept.height), expected, "{case}");
            assert!(!kept.is_locked && !kept.is_instantlocked, "{case}");
        }
    }

    /// A transaction recorded in two accounts is one spender of its inputs,
    /// not a competitor of itself: with no other spender it is uncontested.
    #[tokio::test]
    async fn should_not_count_multi_account_record_as_its_own_competitor() {
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn};
        use key_wallet::transaction_checking::BlockInfo;

        let (mut wallet, mut info, addresses) = two_account_wallet();
        let pay = |previous_output: OutPoint, value: u64, to: usize| Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output,
                ..Default::default()
            }],
            output: vec![TxOut {
                value,
                script_pubkey: addresses[to].script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let funding = pay(
            OutPoint::new(Txid::from_byte_array([83; 32]), 0),
            100_000,
            0,
        );
        let coin = OutPoint::new(funding.txid(), 0);
        let transfer = pay(coin, 99_000, 1);
        let locked = TransactionContext::InstantSend(InstantLock {
            txid: transfer.txid(),
            inputs: vec![coin],
            ..Default::default()
        });
        let block = BlockInfo::new(100, BlockHash::from_byte_array([84; 32]), 100);
        info.check_core_transaction(
            &funding,
            TransactionContext::InBlock(block),
            &mut wallet,
            true,
            true,
        )
        .await;
        info.check_core_transaction(&transfer, locked.clone(), &mut wallet, true, true)
            .await;
        for index in [0, 1] {
            assert!(
                info.accounts.standard_bip44_accounts[&index]
                    .transactions()
                    .contains_key(&transfer.txid()),
                "the transfer must be on record in account {index}"
            );
        }
        let stored = StoredTransaction {
            txid: transfer.txid(),
            transaction: transfer,
            context: locked,
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: Vec::new(),
        };

        assert!(!is_contested(&stored, &unconfirmed_spends(&info)));
    }

    /// An empty history leaves the restored projection untouched.
    #[tokio::test]
    async fn should_leave_projection_untouched_without_history() {
        let (mut wallet, restored, _, _, _) = confirmed_spend_fixture().await;
        let mut info = restored.clone();
        let count = replay_recorded_history(&mut info, &mut wallet, RecordedHistory::default())
            .await
            .replayed;
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

    /// Settled spends must sweep persisted competing change after all siblings
    /// replay, and a raw restored loser must not come back as a fallback. Only a
    /// chainlock overrules a competitor that holds its own InstantSend lock.
    #[tokio::test]
    async fn should_sweep_conflicting_spend_for_either_sibling_replay_order() {
        use dashcore::ephemerealdata::chain_lock::ChainLock;
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, Transaction, TxIn, TxOut};
        use key_wallet::transaction_checking::BlockInfo;

        let settlements = [
            ("instant_send", false),
            ("block", false),
            ("chainlock", false),
            ("block", true),
            ("chainlock", true),
            ("persisted_chainlock", true),
            ("persisted_chainlock_below", true),
        ];
        let orders = [(false, false), (true, false), (false, true), (true, true)];
        for (settlement, locked_competitor) in settlements {
            for (winner_later_txid, raw_restored) in orders {
                let case = format!(
                    "{settlement}, locked_competitor={locked_competitor}, \
                     winner_later_txid={winner_later_txid}, raw_restored={raw_restored}"
                );
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
                if (winner.txid() > loser.txid()) != winner_later_txid {
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
                let lock_of = |txid| InstantLock {
                    inputs: vec![OutPoint::new(funding.txid(), 0)],
                    txid,
                    ..Default::default()
                };
                let mut locks = BTreeMap::new();
                if settlement == "instant_send" {
                    locks.insert(winner.txid(), lock_of(winner.txid()));
                } else {
                    let block = BlockInfo::new(101, BlockHash::from_byte_array([10; 32]), 101);
                    let record = records
                        .iter_mut()
                        .find(|record| record.txid == winner.txid())
                        .unwrap();
                    record.context = if settlement == "chainlock" {
                        TransactionContext::InChainLockedBlock(block)
                    } else {
                        TransactionContext::InBlock(block)
                    };
                }
                if settlement.starts_with("persisted_chainlock") {
                    restored.metadata.last_applied_chain_lock = Some(ChainLock {
                        block_height: if settlement == "persisted_chainlock" {
                            101
                        } else {
                            100
                        },
                        block_hash: BlockHash::from_byte_array([10; 32]),
                        signature: [0; 96].into(),
                    });
                }
                if locked_competitor {
                    locks.insert(loser.txid(), lock_of(loser.txid()));
                }
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

                replay_recorded_history(&mut restored, &mut wallet, history(records, locks)).await;

                let account = &restored.accounts.standard_bip44_accounts[&0];
                let keep_competitor = matches!(settlement, "block" | "persisted_chainlock_below")
                    && locked_competitor;
                assert_eq!(
                    account.transactions().contains_key(&loser.txid()),
                    keep_competitor,
                    "{case}: only a chainlock can overrule an InstantSend lock"
                );
                assert_eq!(
                    account.utxos.contains_key(&OutPoint::new(loser.txid(), 1)),
                    keep_competitor,
                    "{case}: the swept spend's change must not stay selectable"
                );
                assert!(
                    account.utxos.contains_key(&OutPoint::new(winner.txid(), 1)),
                    "{case}: the winner's change stays"
                );
                let expected_balance = winner.output[1].value
                    + if keep_competitor {
                        loser.output[1].value
                    } else {
                        0
                    };
                assert_eq!(restored.balance.total(), expected_balance, "{case}");
            }
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
