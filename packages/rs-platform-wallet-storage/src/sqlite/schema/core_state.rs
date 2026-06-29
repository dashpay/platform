//! Writers + readers for the `core_*` tables.

#[cfg(any(test, feature = "__test-helpers"))]
use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use dashcore::ephemerealdata::chain_lock::ChainLock;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: core-chain state reaching `record_blob` /
// `islock_blob` (transaction records + InstantLocks are public chain data).
impl_persistable_blob!(TransactionRecord, dashcore::InstantLock);

/// Bounded bincode config for `ChainLock` BLOB columns — native bincode
/// (not the serde bridge) because `ChainLock` enables the `bincode` feature
/// but not `serde`. The size limit caps allocations symmetrically with other
/// BLOB columns (`blob::BLOB_SIZE_LIMIT_BYTES`).
fn chain_lock_config() -> impl bincode::config::Config {
    bincode::config::standard().with_limit::<{ blob::BLOB_SIZE_LIMIT_BYTES }>()
}

/// Encode a `ChainLock` to bytes for storage in `core_sync_state`.
fn encode_chain_lock(cl: &ChainLock) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::encode_to_vec(cl, chain_lock_config())?)
}

/// Decode a `ChainLock` from `core_sync_state.last_applied_chain_lock`.
/// Returns `None` + emits a `tracing::warn` on any decode failure so a
/// single corrupt byte cannot prevent the wallet from loading (the next
/// ChainLock event will repopulate the column).
fn decode_chain_lock_soft(bytes: &[u8]) -> Option<ChainLock> {
    match bincode::decode_from_slice::<ChainLock, _>(bytes, chain_lock_config()) {
        Ok((cl, _)) => Some(cl),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "core_sync_state.last_applied_chain_lock: decode failed; \
                 field left None — the next ChainLock sync will repopulate"
            );
            None
        }
    }
}

/// Apply a `CoreChangeSet` inside a transaction.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.records.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET \
                height = excluded.height, \
                block_hash = excluded.block_hash, \
                block_time = excluded.block_time, \
                finalized = excluded.finalized, \
                record_blob = excluded.record_blob",
        )?;
        for record in &cs.records {
            let block_info = record.block_info();
            let height = block_info.map(|b| i64::from(b.height()));
            let block_hash = block_info.map(|b| AsRef::<[u8]>::as_ref(&b.block_hash()).to_vec());
            let block_time = block_info.map(|b| i64::from(b.timestamp()));
            let finalized = block_info.is_some();
            let payload = blob::encode(record)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(&record.txid),
                height,
                block_hash,
                block_time,
                finalized,
                payload,
            ])?;
        }
    }
    // `addresses_derived` is intentionally NOT persisted here. The iOS
    // address registry is fed by the FFI `addresses_derived` callback (fired
    // before the UTXO changeset in the same round), and UTXO attribution is
    // hardcoded to the default account (index 0); the storage layer keeps no
    // derived-address lookup table.
    if !cs.new_utxos.is_empty() {
        let mut stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.new_utxos {
            execute_upsert_utxo(&mut stmt, wallet_id, utxo, false)?;
        }
    }
    if !cs.spent_utxos.is_empty() {
        let mut exists_stmt =
            tx.prepare_cached("SELECT 1 FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2")?;
        let mut mark_spent_stmt = tx.prepare_cached(
            "UPDATE core_utxos SET spent = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
        )?;
        let mut upsert_stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.spent_utxos {
            let op = blob::encode_outpoint(&utxo.outpoint)?;
            let exists: bool = exists_stmt
                .query_row(params![wallet_id.as_slice(), &op[..]], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if exists {
                mark_spent_stmt.execute(params![wallet_id.as_slice(), &op[..]])?;
            } else {
                // Spent-only synthetic row for a UTXO we never saw unspent.
                // account_index is the hardcoded default like every row, and
                // inert anyway since spent rows are excluded from
                // `list_unspent_utxos`.
                execute_upsert_utxo(&mut upsert_stmt, wallet_id, utxo, true)?;
            }
        }
    }
    if !cs.instant_locks_for_non_final_records.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET islock_blob = excluded.islock_blob",
        )?;
        for (txid, islock) in &cs.instant_locks_for_non_final_records {
            let payload = blob::encode(islock)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(txid),
                payload
            ])?;
        }
    }
    if cs.last_processed_height.is_some()
        || cs.synced_height.is_some()
        || cs.last_applied_chain_lock.is_some()
    {
        let cl_bytes = cs
            .last_applied_chain_lock
            .as_ref()
            .map(encode_chain_lock)
            .transpose()?;
        upsert_sync_state(
            tx,
            wallet_id,
            cs.last_processed_height,
            cs.synced_height,
            cl_bytes,
        )?;
    }
    Ok(())
}

const UPSERT_UTXO_SQL: &str = "INSERT INTO core_utxos \
        (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL) \
     ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
        value = excluded.value, \
        script = excluded.script, \
        height = excluded.height, \
        account_index = excluded.account_index, \
        spent = excluded.spent";

/// Account index written for every `core_utxos` row. The product uses only
/// the default account (index 0); a non-default funds account causes
/// `core_bridge::warn_if_non_default_account` to emit a `warn!` log but
/// the record is still persisted under index 0 (dropping it would
/// undercount the balance and lose funds). The one reader
/// (`list_unspent_utxos` per-account grouping) groups everything under 0.
const CORE_UTXO_ACCOUNT_INDEX: i64 = 0;

/// Upsert one `core_utxos` row. `account_index` is the hardcoded default
/// ([`CORE_UTXO_ACCOUNT_INDEX`]); `spent` marks spent-only synthetic rows.
fn execute_upsert_utxo(
    stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    spent: bool,
) -> Result<(), WalletStorageError> {
    let op = blob::encode_outpoint(&utxo.outpoint)?;
    stmt.execute(params![
        wallet_id.as_slice(),
        &op[..],
        crate::sqlite::util::safe_cast::u64_to_i64("core_utxos.value", utxo.value())?,
        utxo.txout.script_pubkey.as_bytes(),
        i64::from(utxo.height),
        CORE_UTXO_ACCOUNT_INDEX,
        spent,
    ])?;
    Ok(())
}

fn upsert_sync_state(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    last_processed: Option<u32>,
    synced: Option<u32>,
    chain_lock_bytes: Option<Vec<u8>>,
) -> Result<(), WalletStorageError> {
    // Read current row for monotonic-max height merge + to carry forward any
    // existing chain lock when the changeset doesn't include a new one.
    let current_raw: (Option<i64>, Option<i64>, Option<Vec<u8>>) = tx
        .query_row(
            "SELECT last_processed_height, synced_height, last_applied_chain_lock \
             FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .unwrap_or((None, None, None));
    // Monotonic-max semantics for sync watermarks.
    let current = (
        sync_height_u32("core_sync_state.last_processed_height", current_raw.0)?,
        sync_height_u32("core_sync_state.synced_height", current_raw.1)?,
    );
    let lp = match (current.0, last_processed) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let sy = match (current.1, synced) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    // Chain lock: take the new bytes when provided; keep the existing bytes
    // when the changeset has no chain lock update (None = "no change").
    let cl_final = chain_lock_bytes.or(current_raw.2);
    tx.execute(
        "INSERT INTO core_sync_state \
            (wallet_id, last_processed_height, synced_height, last_applied_chain_lock) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            last_processed_height = excluded.last_processed_height, \
            synced_height = excluded.synced_height, \
            last_applied_chain_lock = excluded.last_applied_chain_lock",
        params![
            wallet_id.as_slice(),
            lp.map(i64::from),
            sy.map(i64::from),
            cl_final
        ],
    )?;
    Ok(())
}

/// Bulk-reconstruct the keyless [`CoreChangeSet`] projection for one wallet
/// from the `core_*` tables. PUBLIC material only; mints no `Wallet`. `network`
/// (from `wallets`) turns a persisted `script` back into an `Address`.
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Unspent UTXOs** (`new_utxos`): every `spent = 0` row — the balance
///   source (no-silent-zero); a row with a block `height` is confirmed.
/// - **Transaction records** / **IS-locks** / **sync watermarks**: decoded
///   bit-exact, fail-hard on a corrupt blob.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **Per-account UTXO attribution / `is_coinbase` / `is_instantlocked` /
///   `is_trusted` / `used` flags**: not carried by `core_utxos`; defaulted and
///   refreshed on the next scan. The wallet *total* balance is unaffected.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<CoreChangeSet, WalletStorageError> {
    let mut cs = CoreChangeSet::default();

    // Unspent UTXOs → new_utxos (the balance source).
    {
        let mut stmt = conn.prepare(
            "SELECT outpoint, value, script, height FROM core_utxos \
             WHERE wallet_id = ?1 AND spent = 0",
        )?;
        let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
            let op: Vec<u8> = row.get(0)?;
            let value: i64 = row.get(1)?;
            let script: Vec<u8> = row.get(2)?;
            let height: Option<i64> = row.get(3)?;
            Ok((op, value, script, height))
        })?;
        for r in rows {
            let (op_bytes, value, script_bytes, height) = r?;
            let outpoint = blob::decode_outpoint(&op_bytes)?;
            let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
            let height_u32 = match height {
                None => 0u32,
                Some(h) => crate::sqlite::util::safe_cast::i64_to_u32("core_utxos.height", h)?,
            };
            let script = dashcore::ScriptBuf::from_bytes(script_bytes);
            let address = dashcore::Address::from_script(&script, network)
                .map_err(|_| WalletStorageError::blob_decode("core_utxos.script not an address"))?;
            let confirmed = height.map(|h| h > 0).unwrap_or(false);
            let utxo = Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value,
                    script_pubkey: script,
                },
                address,
                height: height_u32,
                is_coinbase: false,
                is_confirmed: confirmed,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            };
            cs.new_utxos.push(utxo);
        }
    }

    {
        let mut stmt =
            conn.prepare("SELECT record_blob FROM core_transactions WHERE wallet_id = ?1")?;
        let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })?;
        for r in rows {
            let payload = r?;
            cs.records
                .push(blob::decode::<TransactionRecord>(&payload)?);
        }
    }

    {
        let mut stmt =
            conn.prepare("SELECT txid, islock_blob FROM core_instant_locks WHERE wallet_id = ?1")?;
        let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
            let txid: Vec<u8> = row.get(0)?;
            let blob_bytes: Vec<u8> = row.get(1)?;
            Ok((txid, blob_bytes))
        })?;
        for r in rows {
            use dashcore::hashes::Hash;
            let (txid_bytes, blob_bytes) = r?;
            let txid = dashcore::Txid::from_slice(&txid_bytes)
                .map_err(|_| WalletStorageError::blob_decode("core_instant_locks.txid"))?;
            let islock: dashcore::ephemerealdata::instant_lock::InstantLock =
                blob::decode(&blob_bytes)?;
            cs.instant_locks_for_non_final_records.insert(txid, islock);
        }
    }

    // Sync watermarks + persisted chain lock.
    if let Some((lp, sy, cl_bytes)) = conn
        .query_row(
            "SELECT last_processed_height, synced_height, last_applied_chain_lock \
             FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| {
                let lp: Option<i64> = row.get(0)?;
                let sy: Option<i64> = row.get(1)?;
                let cl: Option<Vec<u8>> = row.get(2)?;
                Ok((lp, sy, cl))
            },
        )
        .optional()?
    {
        // Fail-hard on an out-of-range watermark (corruption is never skipped).
        cs.last_processed_height = sync_height_u32("core_sync_state.last_processed_height", lp)?;
        cs.synced_height = sync_height_u32("core_sync_state.synced_height", sy)?;
        // Soft-fail on a corrupt chain lock blob — a single bad byte must not
        // prevent the wallet from loading; the next ChainLock event repopulates.
        if let Some(bytes) = cl_bytes {
            cs.last_applied_chain_lock = decode_chain_lock_soft(&bytes);
        }
    }

    Ok(cs)
}

/// Every address that has ever held a `core_utxos` row for this wallet —
/// spent **and** unspent — deduplicated. The rehydration address-reuse
/// guard: an address whose UTXO was since spent must still be marked used
/// so it's never handed back out as a fresh receive address. `network`
/// turns each persisted `script` back into an [`Address`](dashcore::Address);
/// a script that isn't a valid address is a hard error (corruption is never
/// silently dropped), matching [`load_state`]'s unspent-UTXO handling.
pub fn load_used_addresses(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<dashcore::Address>, WalletStorageError> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT script FROM core_utxos WHERE wallet_id = ?1 ORDER BY script")?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        row.get::<_, Vec<u8>>(0)
    })?;
    let mut out = Vec::new();
    for r in rows {
        let script = dashcore::ScriptBuf::from_bytes(r?);
        let address = dashcore::Address::from_script(&script, network)
            .map_err(|_| WalletStorageError::blob_decode("core_utxos.script not an address"))?;
        out.push(address);
    }
    Ok(out)
}

/// Convert a stored sync-height column to `u32`, erroring on overflow
/// rather than silently truncating a corrupt/out-of-range value.
fn sync_height_u32(
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u32>, WalletStorageError> {
    value
        .map(|v| crate::sqlite::util::safe_cast::i64_to_u32(field, v))
        .transpose()
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
#[cfg(any(test, feature = "__test-helpers"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnspentRow {
    pub outpoint: dashcore::OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub height: Option<u32>,
    pub account_index: u32,
}

/// All UTXOs for a wallet that have not been spent yet, bucketed by
/// account index. Retained for this crate's integration tests.
#[cfg(any(test, feature = "__test-helpers"))]
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
            Some(h) => Some(crate::sqlite::util::safe_cast::i64_to_u32(
                "core_utxos.height",
                h,
            )?),
        };
        let account_index =
            crate::sqlite::util::safe_cast::i64_to_u32("core_utxos.account_index", account_index)?;
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
