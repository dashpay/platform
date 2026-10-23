//! Revalidate persisted peer spend claims before rebuilding spendable funds.

use std::collections::{HashMap, HashSet, VecDeque};

use dashcore::{hashes::Hash, OutPoint, ScriptBuf, TxOut, Txid};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::transaction_checking::TransactionContext;
use platform_wallet::changeset::{is_owned, CoreChangeSet};
use platform_wallet::wallet::core::spend_authorization::verify_input_authorization;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Transaction};

use super::{asset_locks, blob, dashpay};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast::i64_to_u64;

/// Remove unauthorized spend bodies and their descendants atomically, retaining final claims.
pub(crate) fn repair(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    state: &CoreChangeSet,
) -> Result<bool, WalletStorageError> {
    if !state
        .records
        .iter()
        .any(|record| matches!(record.context, TransactionContext::Mempool))
    {
        return Ok(false);
    }
    let mut coins = HashMap::new();
    let mut stmt = tx.prepare_cached(
        "SELECT length(outpoint), outpoint, value, length(script), script \
         FROM core_utxos WHERE wallet_id = ?1 AND is_sweep_placeholder = 0",
    )?;
    let mut rows = stmt.query([wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        let raw: Vec<u8> = row.get(1)?;
        blob::check_size(row.get(3)?)?;
        let script: Vec<u8> = row.get(4)?;
        let value = i64_to_u64("core_utxos.value", row.get(2)?)?;
        coins.insert(
            blob::decode_outpoint(&raw)?,
            TxOut {
                value,
                script_pubkey: ScriptBuf::from_bytes(script),
            },
        );
    }
    let authenticated = |record: &TransactionRecord| {
        !matches!(record.context, TransactionContext::Mempool)
            || state
                .instant_locks_for_non_final_records
                .get(&record.txid)
                .is_some_and(|lock| {
                    lock.txid == record.txid
                        && lock.inputs.len() == record.transaction.input.len()
                        && lock
                            .inputs
                            .iter()
                            .zip(&record.transaction.input)
                            .all(|(locked, input)| *locked == input.previous_output)
                })
    };
    let funding: HashMap<_, _> = state
        .records
        .iter()
        .map(|record| (record.txid, record))
        .collect();
    // Peer-supplied records can be large, so ownership and input details are
    // indexed once instead of scanned per input.
    let owned: HashSet<OutPoint> = funding
        .iter()
        .flat_map(|(txid, parent)| {
            parent
                .output_details
                .iter()
                .filter(|output| is_owned(output.role))
                .map(|output| OutPoint::new(*txid, output.index))
        })
        .collect();
    let mut rejected = HashSet::new();
    for record in &state.records {
        if authenticated(record) {
            continue;
        }
        let mut details = HashMap::with_capacity(record.input_details.len());
        for detail in &record.input_details {
            details.entry(detail.index as usize).or_insert(detail);
        }
        for (index, input) in record.transaction.input.iter().enumerate() {
            // Input details cover a previously owned coin whose materialized row is absent.
            let detail = details.get(&index).map(|detail| TxOut {
                value: detail.value,
                script_pubkey: detail.address.script_pubkey(),
            });
            let parent = funding.get(&input.previous_output.txid);
            let output_index = input.previous_output.vout;
            let owned_by_record = owned.contains(&input.previous_output);
            let remembered = coins.get(&input.previous_output).or(detail.as_ref());
            if remembered.is_none() && !owned_by_record {
                continue;
            }
            // A txid-checked funding body takes precedence over accounting annotations.
            let canonical =
                parent.and_then(|parent| parent.transaction.output.get(output_index as usize));
            let Some(prevout) = canonical.or(remembered) else {
                rejected.insert(record.txid);
                break;
            };
            if !verify_input_authorization(&record.transaction, index, prevout)? {
                rejected.insert(record.txid);
                break;
            }
        }
    }
    // Follow descendants once, even if persisted rows are in reverse dependency order.
    let mut children: HashMap<Txid, Vec<Txid>> = HashMap::new();
    for record in state.records.iter().filter(|record| !authenticated(record)) {
        for input in &record.transaction.input {
            children
                .entry(input.previous_output.txid)
                .or_default()
                .push(record.txid);
        }
    }
    let mut pending: VecDeque<_> = rejected.iter().copied().collect();
    while let Some(parent) = pending.pop_front() {
        if let Some(descendants) = children.get(&parent) {
            for child in descendants {
                if rejected.insert(*child) {
                    pending.push_back(*child);
                }
            }
        }
    }
    if rejected.is_empty() {
        return Ok(false);
    }
    let survivors: HashMap<OutPoint, Txid> = state
        .records
        .iter()
        .filter(|record| !rejected.contains(&record.txid))
        .flat_map(|record| {
            record
                .transaction
                .input
                .iter()
                .map(move |input| (input.previous_output, record.txid))
        })
        .fold(HashMap::new(), |mut claims, (outpoint, txid)| {
            claims
                .entry(outpoint)
                .and_modify(|existing| *existing = (*existing).min(txid))
                .or_insert(txid);
            claims
        });
    // Release before deleting bodies: their cleanup trigger clears claimant IDs.
    // Never reinterpret an anonymous or recordless final claim as a rejected peer claim.
    let mut claims = Vec::new();
    let mut stmt = tx.prepare_cached("SELECT length(outpoint), outpoint, length(spent_in_txid), spent_in_txid, is_sweep_placeholder FROM core_utxos WHERE wallet_id = ?1 AND spent = 1 AND spent_in_txid IS NOT NULL")?;
    let mut rows = stmt.query([wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        let raw: Vec<u8> = row.get(1)?;
        blob::check_fixed_width(row.get(2)?, 32, "core_utxos.spent_in_txid")?;
        let claimant: Vec<u8> = row.get(3)?;
        if rejected.contains(&Txid::from_slice(&claimant)?) {
            claims.push((raw, row.get::<_, bool>(4)?));
        }
    }
    for (raw, placeholder) in claims {
        let op = blob::decode_outpoint(&raw)?;
        if let Some(survivor) = survivors.get(&op) {
            tx.execute(
                "UPDATE core_utxos SET spent_in_txid = ?3 WHERE wallet_id = ?1 AND outpoint = ?2",
                params![
                    wallet_id.as_slice(),
                    raw,
                    survivor.as_byte_array().as_slice()
                ],
            )?;
        } else if placeholder {
            tx.execute(
                "DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![wallet_id.as_slice(), raw],
            )?;
        } else {
            tx.execute("UPDATE core_utxos SET spent = 0, spent_in_txid = NULL, winner_mined_height = NULL WHERE wallet_id = ?1 AND outpoint = ?2", params![wallet_id.as_slice(), raw])?;
        }
    }
    for txid in &rejected {
        let key = params![wallet_id.as_slice(), txid.as_byte_array().as_slice()];
        tx.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            key,
        )?;
        tx.execute(
            "DELETE FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
            key,
        )?;
        // `encode_outpoint_txid_occupies_bytes_two_to_thirty_three` pins this prefix.
        // SQLite plans a DELETE through an expression index only when named
        // (V020 explains the index); naming it also fails loudly if it goes.
        tx.execute(
            "DELETE FROM core_utxos INDEXED BY idx_core_utxos_funding \
             WHERE wallet_id = ?1 AND substr(outpoint, 2, 32) = ?2",
            key,
        )?;
    }
    asset_locks::remove_swept(tx, wallet_id, &rejected)?;
    dashpay::settle_swept_payments(tx, wallet_id, &rejected)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use dashcore::address::Payload;
    use dashcore::{Address, BlockHash, Network, PubkeyHash, Transaction, TxIn};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        OutputDetail, OutputRole, TransactionDirection,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionType};

    use super::*;
    use crate::sqlite::migrations::{self, wallet_db};

    fn record(transaction: Transaction, context: TransactionContext) -> TransactionRecord {
        let account = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let (kind, direction) = (TransactionType::Standard, TransactionDirection::Incoming);
        TransactionRecord::new(
            transaction,
            account,
            context,
            kind,
            direction,
            vec![],
            vec![],
            0,
        )
    }

    /// A peer can relay a pending record whose inputs name vouts its stored
    /// parent lacks: they are no wallet coins, and checking them must not
    /// rescan the parent's outputs once per input.
    #[test]
    fn should_check_inputs_naming_absent_parent_vouts_in_linear_time() {
        const OUTPUTS: u32 = 100_000;
        const ABSENT: u32 = 100_000;
        let wallet_id = [0xC5u8; 32];
        let mut conn = wallet_db(&wallet_id);
        let hash = PubkeyHash::from_byte_array([0x31; 20]);
        let address = Address::new(Network::Testnet, Payload::PubkeyHash(hash));
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn::default()],
            output: (0..OUTPUTS)
                .map(|_| TxOut {
                    value: 1_000,
                    script_pubkey: address.script_pubkey(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let block = BlockInfo::new(100, BlockHash::all_zeros(), 100);
        let mut parent = record(funding, TransactionContext::InBlock(block));
        parent.output_details = (0..OUTPUTS)
            .map(|index| OutputDetail {
                index,
                role: OutputRole::Received,
                address: Some(address.clone()),
                value: 1_000,
            })
            .collect();
        // The last, unsigned input names an owned output, so the check still
        // reaches ownership after every absent vout.
        let input = (OUTPUTS..OUTPUTS + ABSENT)
            .chain([0])
            .map(|vout| TxIn {
                previous_output: OutPoint::new(parent.txid, vout),
                ..Default::default()
            })
            .collect();
        let spend = Transaction {
            version: 1,
            lock_time: 0,
            input,
            output: vec![],
            special_transaction_payload: None,
        };
        let state = CoreChangeSet {
            records: vec![parent, record(spend, TransactionContext::Mempool)],
            ..Default::default()
        };
        let tx = conn.transaction().unwrap();
        let started = Instant::now();
        assert!(
            repair(&tx, &wallet_id, &state).unwrap(),
            "the unsigned spend of an owned output is rejected"
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "ownership checks must not scale with inputs times parent outputs: {elapsed:?}"
        );
    }

    /// Removing a rejected spend deletes its outputs and, through the cleanup
    /// trigger, clears its claims. Neither may visit every stored coin once
    /// per rejected spend, so quadrupling both at once roughly quadruples
    /// the repair's SQL work; quadratic work would grow sixteenfold. The bound
    /// sits between, with headroom both ways.
    #[test]
    fn should_remove_many_rejected_spends_in_work_linear_in_stored_coins() {
        let hash = PubkeyHash::from_byte_array([0x32; 20]);
        let script = Address::new(Network::Testnet, Payload::PubkeyHash(hash)).script_pubkey();
        let ops = |n: u32| {
            let wallet_id = [0xC6u8; 32];
            let mut conn = wallet_db(&wallet_id);
            let coin_row =
                |conn: &rusqlite::Connection, outpoint: &OutPoint, claimant: Option<&Txid>| {
                    conn.execute(
                        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent, \
                     spent_in_txid) VALUES (?1, ?2, 1000, ?3, ?4, ?5)",
                        params![
                            wallet_id.as_slice(),
                            blob::encode_outpoint(outpoint).unwrap(),
                            script.as_bytes(),
                            claimant.is_some(),
                            claimant.map(|txid| txid.as_byte_array().to_vec())
                        ],
                    )
                    .unwrap();
                };
            let mut records = Vec::new();
            for index in 0..n {
                let mut funding = [0x41u8; 32];
                funding[..4].copy_from_slice(&index.to_le_bytes());
                let coin = OutPoint::new(Txid::from_byte_array(funding), 0);
                // Unsigned, so the stored coin's script rejects it.
                let spend = Transaction {
                    version: 1,
                    lock_time: 0,
                    input: vec![TxIn {
                        previous_output: coin,
                        ..Default::default()
                    }],
                    output: vec![TxOut {
                        value: 500,
                        script_pubkey: script.clone(),
                    }],
                    special_transaction_payload: None,
                };
                let spend = record(spend, TransactionContext::Mempool);
                conn.execute(
                    "INSERT INTO core_transactions (wallet_id, txid, finalized) VALUES (?1, ?2, 0)",
                    params![wallet_id.as_slice(), spend.txid.as_byte_array().as_slice()],
                )
                .unwrap();
                coin_row(&conn, &coin, Some(&spend.txid));
                coin_row(&conn, &OutPoint::new(spend.txid, 0), None);
                records.push(spend);
            }
            let state = CoreChangeSet {
                records,
                ..Default::default()
            };
            let tx = conn.transaction().unwrap();
            let ops = migrations::count_vm_ops(&tx);
            assert!(repair(&tx, &wallet_id, &state).unwrap());
            let left: i64 = tx
                .query_row("SELECT COUNT(*) FROM core_utxos WHERE spent = 0", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(left, i64::from(n), "each released coin stays, no output");
            ops.load(std::sync::atomic::Ordering::Relaxed)
        };
        let (small, large) = (ops(200), ops(800));
        assert!(
            large < 8 * small,
            "repair work must grow linearly, not with stored coins times rejected spends: \
             {small} VM ops for 200 of each, {large} for 800"
        );
    }
}
