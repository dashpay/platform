//! Wallet accounting repaired from historical owned outputs, independent of live UTXOs.

use std::collections::{BTreeMap, HashSet};

use dashcore::hashes::Hash;
use dashcore::{Address, OutPoint, ScriptBuf, Txid};
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::{TransactionContext, TransactionType};
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};

use super::{blob, core_state, wallets};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::LoadCtx;
use crate::sqlite::util::safe_cast::i64_to_u64;

/// Preserve proven ownership when a partial account snapshot replaces the wallet record.
pub(super) fn preserve_known_details(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    incoming: &TransactionRecord,
) -> Result<TransactionRecord, WalletStorageError> {
    let mut merged = incoming.clone();
    let Some(previous) =
        core_state::get_tx_record(tx, wallet_id, &incoming.txid, &LoadCtx::strict())?
    else {
        return Ok(merged);
    };
    if previous.transaction != incoming.transaction {
        return Err(WalletStorageError::blob_decode(
            "same transaction id has different raw transaction bodies",
        ));
    }
    let mut inputs: BTreeMap<_, _> = previous
        .input_details
        .into_iter()
        .map(|d| (d.index, d))
        .collect();
    for detail in &incoming.input_details {
        inputs.insert(detail.index, detail.clone());
    }
    let mut outputs: BTreeMap<_, _> = previous
        .output_details
        .into_iter()
        .map(|d| (d.index, d))
        .collect();
    for detail in &incoming.output_details {
        let keep_previous = outputs
            .get(&detail.index)
            .is_some_and(|old| matches!(old.role, OutputRole::Received | OutputRole::Change))
            && !matches!(detail.role, OutputRole::Received | OutputRole::Change);
        if !keep_previous {
            outputs.insert(detail.index, detail.clone());
        }
    }
    merged.input_details = inputs.into_values().collect();
    merged.output_details = outputs.into_values().collect();
    Ok(merged)
}

/// Index raw inputs independently of when their ownership becomes known.
pub(super) fn index_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare_cached(
        "INSERT OR IGNORE INTO core_transaction_inputs (wallet_id, txid, outpoint) VALUES (?1, ?2, ?3)",
    )?;
    for input in &record.transaction.input {
        stmt.execute(params![
            wallet_id.as_slice(),
            record.txid.as_byte_array().as_slice(),
            blob::encode_outpoint(&input.previous_output)?,
        ])?;
    }
    Ok(())
}

/// Repair changed records and consumers of newly materialized historical outputs.
pub(super) fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), WalletStorageError> {
    let mut affected: HashSet<Txid> = cs.records.iter().map(|r| r.txid).collect();
    let mut consumers = tx.prepare_cached(
        "SELECT txid FROM core_transaction_inputs WHERE wallet_id = ?1 AND outpoint = ?2",
    )?;
    for utxo in cs.new_utxos.iter().chain(&cs.spent_utxos) {
        affected.insert(utxo.outpoint.txid);
        let mut rows = consumers.query(params![
            wallet_id.as_slice(),
            blob::encode_outpoint(&utxo.outpoint)?
        ])?;
        while let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            affected.insert(Txid::from_slice(&bytes)?);
        }
    }
    if affected.is_empty() {
        return Ok(());
    }
    let network = network(tx, wallet_id)?;
    for txid in affected {
        repair_record(tx, wallet_id, &txid, network)?;
    }
    Ok(())
}

fn network(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
) -> Result<dashcore::Network, WalletStorageError> {
    let label: String = tx.query_row(
        "SELECT network FROM wallets WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |r| r.get(0),
    )?;
    wallets::parse_network(&label)
        .ok_or_else(|| WalletStorageError::blob_decode("wallets.network is unknown"))
}

fn owned_output(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    outpoint: &OutPoint,
    network: dashcore::Network,
) -> Result<Option<(u64, Address)>, WalletStorageError> {
    let mut stmt = tx.prepare_cached("SELECT value, length(script), script FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 0")?;
    let mut rows = stmt.query(params![
        wallet_id.as_slice(),
        blob::encode_outpoint(outpoint)?
    ])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let value = i64_to_u64("core_utxos.value", row.get(0)?)?;
    blob::check_size(row.get(1)?)?;
    let script: Vec<u8> = row.get(2)?;
    if contact_only_script(tx, wallet_id, &script)? {
        return Ok(None);
    }
    let address = Address::from_script(&ScriptBuf::from_bytes(script), network)?;
    Ok(Some((value, address)))
}

/// Whether `script` is tracked only by a contact's watch-only (DashPay external) chain.
pub(crate) fn contact_only_script(
    conn: &Connection,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<bool, WalletStorageError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM core_address_pool WHERE wallet_id = ?1 AND script = ?2) AND NOT EXISTS(SELECT 1 FROM core_address_pool WHERE wallet_id = ?1 AND script = ?2 AND account_type != 'dashpay_external')",
        params![wallet_id.as_slice(), script], |r| r.get(0))?)
}

fn repair_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    txid: &Txid,
    network: dashcore::Network,
) -> Result<(), WalletStorageError> {
    let Some(mut record) = core_state::get_tx_record(tx, wallet_id, txid, &LoadCtx::strict())?
    else {
        return Ok(());
    };
    let original = blob::encode(&record)?;
    let mut inputs = BTreeMap::new();
    for detail in record.input_details.drain(..) {
        if !contact_only_script(tx, wallet_id, detail.address.script_pubkey().as_bytes())? {
            inputs.insert(detail.index, detail);
        }
    }
    for (index, input) in record.transaction.input.iter().enumerate() {
        if let Some((value, address)) =
            owned_output(tx, wallet_id, &input.previous_output, network)?
        {
            inputs.insert(
                index as u32,
                InputDetail {
                    index: index as u32,
                    value,
                    address,
                },
            );
            // Stale mempool rows cannot overrule a later sweep's release.
            if !matches!(record.context, TransactionContext::Mempool) {
                tx.execute(
                    "UPDATE core_utxos SET spent = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
                    params![
                        wallet_id.as_slice(),
                        blob::encode_outpoint(&input.previous_output)?
                    ],
                )?;
            }
        }
    }
    let mut outputs = BTreeMap::new();
    for mut detail in record.output_details.drain(..) {
        if let Some(address) = &detail.address {
            if contact_only_script(tx, wallet_id, address.script_pubkey().as_bytes())? {
                detail.role = OutputRole::Sent;
            }
        }
        outputs.insert(detail.index, detail);
    }
    for (index, output) in record.transaction.output.iter().enumerate() {
        let index = index as u32;
        if let Some((_, address)) = owned_output(
            tx,
            wallet_id,
            &OutPoint {
                txid: *txid,
                vout: index,
            },
            network,
        )? {
            let role = outputs.get(&index).map_or(OutputRole::Received, |d| {
                if d.role == OutputRole::Change {
                    OutputRole::Change
                } else {
                    OutputRole::Received
                }
            });
            outputs.insert(
                index,
                OutputDetail {
                    index,
                    role,
                    address: Some(address),
                    value: output.value,
                },
            );
        }
    }
    // Empty metadata is not accounting evidence (e.g. confirmation-only placeholders).
    if inputs.is_empty() && outputs.is_empty() {
        return Ok(());
    }
    let received: i128 = outputs
        .values()
        .filter(|d| matches!(d.role, OutputRole::Received | OutputRole::Change))
        .map(|d| i128::from(d.value))
        .sum();
    let spent: i128 = inputs.values().map(|d| i128::from(d.value)).sum();
    record.net_amount = i64::try_from(received - spent).map_err(|_| {
        WalletStorageError::blob_decode("wallet transaction net amount exceeds i64")
    })?;
    let has_ours = outputs
        .values()
        .any(|d| matches!(d.role, OutputRole::Received | OutputRole::Change));
    let has_external = record
        .transaction
        .output
        .iter()
        .enumerate()
        .any(|(i, output)| {
            !output.script_pubkey.is_op_return()
                && !outputs.get(&(i as u32)).is_some_and(|d| {
                    matches!(
                        d.role,
                        OutputRole::Received | OutputRole::Change | OutputRole::Unspendable
                    )
                })
        });
    record.direction = if record.transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if inputs.is_empty() {
        TransactionDirection::Incoming
    } else if !has_external && (has_ours || record.transaction_type == TransactionType::AssetLock) {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    };
    record.input_details = inputs.into_values().collect();
    record.output_details = outputs.into_values().collect();
    let repaired = blob::encode(&record)?;
    if repaired != original {
        tx.execute(
            "UPDATE core_transactions SET record_blob = ?1 WHERE wallet_id = ?2 AND txid = ?3",
            params![
                repaired,
                wallet_id.as_slice(),
                txid.as_byte_array().as_slice()
            ],
        )?;
    }
    Ok(())
}

/// Backfill the input index and correct existing history in the migration transaction.
pub(crate) fn migrate(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare_cached("SELECT length(wallet_id), wallet_id, length(txid), txid FROM core_transactions WHERE record_blob IS NOT NULL")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        blob::check_fixed_width(row.get(0)?, 32, "core_transactions.wallet_id")?;
        let wallet_id: Vec<u8> = row.get(1)?;
        let wallet_id = super::id32("core_transactions.wallet_id", &wallet_id)?;
        blob::check_fixed_width(row.get(2)?, 32, "core_transactions.txid")?;
        let txid: Vec<u8> = row.get(3)?;
        let txid = Txid::from_slice(&txid)?;
        if let Some(record) = core_state::get_tx_record(tx, &wallet_id, &txid, &LoadCtx::strict())?
        {
            index_record(tx, &wallet_id, &record)?;
            repair_record(tx, &wallet_id, &txid, network(tx, &wallet_id)?)?;
        }
    }
    Ok(())
}
