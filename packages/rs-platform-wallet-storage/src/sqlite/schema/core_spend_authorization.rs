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
    let mut rejected = HashSet::new();
    for record in &state.records {
        if authenticated(record) {
            continue;
        }
        for (index, input) in record.transaction.input.iter().enumerate() {
            // Input details cover a previously owned coin whose materialized row is absent.
            let detail = record
                .input_details
                .iter()
                .find(|detail| detail.index as usize == index)
                .map(|detail| TxOut {
                    value: detail.value,
                    script_pubkey: detail.address.script_pubkey(),
                });
            let parent = funding.get(&input.previous_output.txid);
            let output_index = input.previous_output.vout;
            let owned_by_record = parent.is_some_and(|parent| {
                parent
                    .output_details
                    .iter()
                    .any(|output| output.index == output_index && is_owned(output.role))
            });
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
        tx.execute(
            "DELETE FROM core_utxos WHERE wallet_id = ?1 AND substr(outpoint, 2, 32) = ?2",
            key,
        )?;
    }
    asset_locks::remove_swept(tx, wallet_id, &rejected)?;
    dashpay::settle_swept_payments(tx, wallet_id, &rejected)?;
    Ok(true)
}
