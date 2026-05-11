//! Writers + readers for the `core_*` tables.

use std::collections::BTreeMap;

use dashcore::hashes::Hash;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::error::SqlitePersisterError;
use crate::schema::blob::{decode_outpoint, encode_outpoint, BlobReader, BlobWriter};

/// Apply a `CoreChangeSet` inside a transaction.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), SqlitePersisterError> {
    for record in &cs.records {
        upsert_tx_record(tx, wallet_id, record)?;
    }
    for utxo in &cs.new_utxos {
        upsert_utxo(tx, wallet_id, utxo, false)?;
    }
    for utxo in &cs.spent_utxos {
        // Mark existing as spent OR insert as already-spent if unknown.
        let op = encode_outpoint(&utxo.outpoint);
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
        let blob = encode_islock(islock);
        tx.execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET islock_blob = excluded.islock_blob",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid), blob],
        )?;
    }
    if cs.last_processed_height.is_some() || cs.synced_height.is_some() {
        upsert_sync_state(tx, wallet_id, cs.last_processed_height, cs.synced_height)?;
    }
    for da in &cs.addresses_derived {
        // We persist the rendered base58 address as the natural key.
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
) -> Result<(), SqlitePersisterError> {
    let block_info = record.block_info();
    let height = block_info.map(|b| b.height() as i64);
    let block_hash = block_info.map(|b| AsRef::<[u8]>::as_ref(&b.block_hash()).to_vec());
    let block_time = block_info.map(|b| b.timestamp() as i64);
    let finalized = block_info.is_some();
    let blob = encode_record(record);
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
            blob,
        ],
    )?;
    Ok(())
}

fn upsert_utxo(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    spent: bool,
) -> Result<(), SqlitePersisterError> {
    let op = encode_outpoint(&utxo.outpoint);
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
            utxo.value() as i64,
            utxo.txout.script_pubkey.as_bytes(),
            utxo.height as i64,
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
) -> Result<(), SqlitePersisterError> {
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
        params![
            wallet_id.as_slice(),
            lp.map(|x| x as i64),
            sy.map(|x| x as i64),
        ],
    )?;
    Ok(())
}

/// Fetch a single transaction record by txid.
///
/// Returns `Ok(None)` if absent. Per the trait's field contract we only
/// need `txid` + `context` populated; we synthesise a minimal record
/// from the typed columns + the stored blob's height/block-hash data.
pub fn get_tx_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
) -> Result<Option<TransactionRecord>, SqlitePersisterError> {
    type RecordRow = (Option<i64>, Option<Vec<u8>>, Option<i64>, Vec<u8>);
    let row: Option<RecordRow> = conn
        .query_row(
            "SELECT height, block_hash, block_time, record_blob \
             FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let Some((height, block_hash, block_time, blob)) = row else {
        return Ok(None);
    };
    let record = decode_record(&blob, *txid, height, block_hash.as_deref(), block_time)?;
    Ok(Some(record))
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
) -> Result<BTreeMap<u32, Vec<UnspentRow>>, SqlitePersisterError> {
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
        let outpoint = decode_outpoint(&op_bytes).map_err(SqlitePersisterError::serialization)?;
        let row = UnspentRow {
            outpoint,
            value: value as u64,
            script: script_bytes,
            height: height.map(|h| h as u32),
            account_index: account_index as u32,
        };
        by_account
            .entry(account_index as u32)
            .or_default()
            .push(row);
    }
    Ok(by_account)
}

// ----- Blob codecs -----

fn encode_record(record: &TransactionRecord) -> Vec<u8> {
    let mut w = BlobWriter::new();
    // Fields persisted: txid (already a PK column, but redundancy
    // keeps the blob self-describing), label, fee, net_amount.
    w.bytes(AsRef::<[u8]>::as_ref(&record.txid));
    w.str(&record.label);
    w.opt_u64(record.fee);
    w.u64(record.net_amount as u64);
    w.finish()
}

fn decode_record(
    blob: &[u8],
    txid: dashcore::Txid,
    height: Option<i64>,
    block_hash: Option<&[u8]>,
    block_time: Option<i64>,
) -> Result<TransactionRecord, SqlitePersisterError> {
    let mut r = BlobReader::new(blob).map_err(SqlitePersisterError::serialization)?;
    let _persisted_txid = r.bytes().map_err(SqlitePersisterError::serialization)?;
    let label = r.str().map_err(SqlitePersisterError::serialization)?;
    let fee = r.opt_u64().map_err(SqlitePersisterError::serialization)?;
    let net_amount = r.u64().map_err(SqlitePersisterError::serialization)? as i64;

    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

    let context = match (height, block_hash, block_time) {
        (Some(h), Some(hash_bytes), Some(t)) if hash_bytes.len() == 32 => {
            let hash = dashcore::BlockHash::from_slice(hash_bytes)
                .map_err(SqlitePersisterError::serialization)?;
            TransactionContext::InChainLockedBlock(BlockInfo::new(h as u32, hash, t as u32))
        }
        _ => TransactionContext::Mempool,
    };

    // Per the trait's `get_core_tx_record` contract: only `txid` and
    // `context` are required. Everything else MAY be a placeholder.
    let placeholder_tx = dashcore::blockdata::transaction::Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    };
    let mut record = TransactionRecord::new(
        placeholder_tx,
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        context,
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        Vec::new(),
        net_amount,
    );
    record.txid = txid;
    if let Some(f) = fee {
        record.set_fee(f);
    }
    let _ = record.set_label(label);
    Ok(record)
}

fn encode_islock(islock: &dashcore::ephemerealdata::instant_lock::InstantLock) -> Vec<u8> {
    use dashcore::consensus::Encodable;
    let mut buf = Vec::new();
    let _ = islock.consensus_encode(&mut buf);
    buf
}
