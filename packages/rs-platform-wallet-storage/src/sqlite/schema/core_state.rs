//! Writers + readers for the `core_*` tables.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Apply a `CoreChangeSet` inside a transaction.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), WalletStorageError> {
    for record in &cs.records {
        upsert_tx_record(tx, wallet_id, record)?;
    }
    for utxo in &cs.new_utxos {
        upsert_utxo(tx, wallet_id, utxo, false)?;
    }
    for utxo in &cs.spent_utxos {
        let op = blob::encode_outpoint(&utxo.outpoint);
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![wallet_id.as_slice(), &op[..]],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if exists {
            tx.execute(
                "UPDATE core_utxos SET spent = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
                params![wallet_id.as_slice(), &op[..]],
            )?;
        } else {
            upsert_utxo(tx, wallet_id, utxo, true)?;
        }
    }
    for (txid, islock) in &cs.instant_locks_for_non_final_records {
        let payload = blob::encode(islock)?;
        tx.execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET islock_blob = excluded.islock_blob",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid), payload],
        )?;
    }
    if cs.last_processed_height.is_some() || cs.synced_height.is_some() {
        upsert_sync_state(tx, wallet_id, cs.last_processed_height, cs.synced_height)?;
    }
    for da in &cs.addresses_derived {
        // `account_type` and `pool_type` are stored Debug-rendered for
        // disambiguation across pools sharing the same address space.
        let account_type = format!("{:?}", da.account_type);
        let address = da.address.to_string();
        let path = format!("{:?}/{}", da.pool_type, da.derivation_index);
        tx.execute(
            "INSERT INTO core_derived_addresses (wallet_id, account_type, address, derivation_path, used) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, account_type, address) DO UPDATE SET \
                derivation_path = excluded.derivation_path",
            params![wallet_id.as_slice(), account_type, address, path, false],
        )?;
    }
    Ok(())
}

fn upsert_tx_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let block_info = record.block_info();
    let height = block_info.map(|b| i64::from(b.height()));
    let block_hash = block_info.map(|b| AsRef::<[u8]>::as_ref(&b.block_hash()).to_vec());
    let block_time = block_info.map(|b| i64::from(b.timestamp()));
    let finalized = block_info.is_some();
    let payload = blob::encode(record)?;
    tx.execute(
        "INSERT INTO core_transactions \
            (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT(wallet_id, txid) DO UPDATE SET \
            height = excluded.height, \
            block_hash = excluded.block_hash, \
            block_time = excluded.block_time, \
            finalized = excluded.finalized, \
            record_blob = excluded.record_blob",
        params![
            wallet_id.as_slice(),
            AsRef::<[u8]>::as_ref(&record.txid),
            height,
            block_hash,
            block_time,
            finalized,
            payload,
        ],
    )?;
    Ok(())
}

fn upsert_utxo(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    spent: bool,
) -> Result<(), WalletStorageError> {
    let op = blob::encode_outpoint(&utxo.outpoint);
    tx.execute(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL) \
         ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
            value = excluded.value, \
            script = excluded.script, \
            height = excluded.height, \
            account_index = excluded.account_index, \
            spent = excluded.spent",
        params![
            wallet_id.as_slice(),
            &op[..],
            crate::sqlite::util::safe_cast::u64_to_i64("core_utxos.value", utxo.value())?,
            utxo.txout.script_pubkey.as_bytes(),
            i64::from(utxo.height),
            0i64, // Utxo does not carry account_index; populated by derived-address lookup later.
            spent,
        ],
    )?;
    Ok(())
}

fn upsert_sync_state(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    last_processed: Option<u32>,
    synced: Option<u32>,
) -> Result<(), WalletStorageError> {
    // Monotonic-max semantics — keep the larger of (current, new).
    let current = tx
        .query_row(
            "SELECT last_processed_height, synced_height FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| {
                let lp: Option<i64> = row.get(0)?;
                let sy: Option<i64> = row.get(1)?;
                Ok((lp.map(|x| x as u32), sy.map(|x| x as u32)))
            },
        )
        .optional()?
        .unwrap_or((None, None));
    let lp = match (current.0, last_processed) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let sy = match (current.1, synced) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    tx.execute(
        "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            last_processed_height = excluded.last_processed_height, \
            synced_height = excluded.synced_height",
        params![wallet_id.as_slice(), lp.map(i64::from), sy.map(i64::from),],
    )?;
    Ok(())
}

/// Fetch a single transaction record by txid. Returns `Ok(None)` if
/// absent.
pub fn get_tx_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    let row: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some(payload) => Ok(Some(blob::decode(&payload)?)),
    }
}

/// Row representing one unspent UTXO. Used by tests that probe the
/// `core_utxos` table without going through full `Wallet` reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnspentRow {
    pub outpoint: dashcore::OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub height: Option<u32>,
    pub account_index: u32,
}

/// All UTXOs for a wallet that have not been spent yet, bucketed by
/// account index. Used by `load` and tests.
pub fn list_unspent_utxos(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, Vec<UnspentRow>>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, value, script, height, account_index \
         FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let value: i64 = row.get(1)?;
        let script: Vec<u8> = row.get(2)?;
        let height: Option<i64> = row.get(3)?;
        let account_index: i64 = row.get(4)?;
        Ok((op_bytes, value, script, height, account_index))
    })?;
    let mut by_account: BTreeMap<u32, Vec<UnspentRow>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, value, script_bytes, height, account_index) = r?;
        let outpoint = blob::decode_outpoint(&op_bytes)?;
        let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
        let height = match height {
            None => None,
            Some(h) => Some(
                u32::try_from(h).map_err(|_| WalletStorageError::IntegerOverflow {
                    field: "core_utxos.height",
                    value: h as u64,
                    target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
                })?,
            ),
        };
        let account_index =
            u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
                field: "core_utxos.account_index",
                value: account_index as u64,
                target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
            })?;
        let row = UnspentRow {
            outpoint,
            value,
            script: script_bytes,
            height,
            account_index,
        };
        by_account.entry(account_index).or_default().push(row);
    }
    Ok(by_account)
}
