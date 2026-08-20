//! Writers + readers for the `core_*` tables.

#[cfg(any(test, feature = "__test-helpers"))]
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use dashcore::ephemerealdata::chain_lock::ChainLock;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;
use crate::sqlite::schema::core_pool::{owning_account_for_script, OwningAccount};

// PUBLIC material only: core-chain state reaching `record_blob` /
// `islock_blob` (transaction records + InstantLocks are public chain data).
impl_persistable_blob!(TransactionRecord, dashcore::InstantLock);

/// Encode a `ChainLock` to bytes for storage in `core_sync_state`.
fn encode_chain_lock(cl: &ChainLock) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::encode_to_vec(cl, blob::bounded_config())?)
}

/// Decode a `ChainLock` from `core_sync_state.last_applied_chain_lock`.
///
/// Error mapping mirrors [`blob::decode`]: an over-cap payload is
/// [`WalletStorageError::BlobTooLarge`], trailing bytes after the typed
/// length are a `BlobDecode`, and anything else keeps the upstream bincode
/// error as its source.
///
/// # Errors
///
/// Under [`LoadPolicy::Strict`](crate::LoadPolicy) any of the above aborts
/// the load. Under `Recovery` they are counted and the field is left
/// `None`, which the next ChainLock sync repopulates. `BlobTooLarge` is
/// fatal in both — recovery tolerates inconsistent rows, not oversize
/// allocations.
fn decode_chain_lock(bytes: &[u8], ctx: &LoadCtx) -> Result<Option<ChainLock>, WalletStorageError> {
    let failure = match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        Ok((cl, consumed)) if consumed == bytes.len() => return Ok(Some(cl)),
        Ok(_) => WalletStorageError::blob_decode(
            "unexpected trailing bytes in core_sync_state.last_applied_chain_lock",
        ),
        Err(bincode::error::DecodeError::LimitExceeded) => {
            return Err(WalletStorageError::BlobTooLarge {
                len_bytes: bytes.len(),
                limit_bytes: blob::BLOB_SIZE_LIMIT_BYTES,
            })
        }
        Err(other) => WalletStorageError::from(other),
    };
    ctx.tolerate(LoadSite::ChainLockBlob, failure)?;
    Ok(None)
}

/// Block height of an encoded `last_applied_chain_lock` blob, or `None` if it
/// can't be decoded. Used to monotonic-max-merge the chain lock so an
/// out-of-order lower-height update never regresses the finalized checkpoint.
fn chain_lock_height(bytes: &[u8]) -> Option<u32> {
    match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        // Require full consumption (like `decode_chain_lock`) so a corrupt
        // stored blob can't out-rank a later valid update and stay stuck.
        Ok((cl, consumed)) if consumed == bytes.len() => Some(cl.block_height),
        _ => None,
    }
}

/// Apply a `CoreChangeSet` inside a transaction.
///
/// Recordless UTXOs write monotonic height-only rows; transaction records win.
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
    // `addresses_derived` is intentionally NOT persisted here — the pool
    // snapshot (`account_address_pools`) is the derived-address source used
    // to resolve UTXO ownership during rehydration.
    if !cs.new_utxos.is_empty() {
        // Blob-bearing records always win; height-only writes never overwrite them.
        // Placeholder confirmation is `height IS NOT NULL`; `finalized` stays 0.
        let mut height_only_stmt = tx.prepare_cached(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, ?3, NULL, NULL, 0, NULL) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET height = \
                CASE \
                    WHEN excluded.height IS NOT NULL \
                         AND (core_transactions.height IS NULL \
                              OR excluded.height > core_transactions.height) \
                    THEN excluded.height \
                    ELSE core_transactions.height \
                END \
             WHERE core_transactions.record_blob IS NULL",
        )?;
        let mut utxo_stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.new_utxos {
            let affected = height_only_stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(&utxo.outpoint.txid),
                utxo.is_confirmed.then_some(i64::from(utxo.height)),
            ])?;
            if affected == 0 {
                tracing::debug!(
                    txid = %utxo.outpoint.txid,
                    "existing transaction record blocked a stale height-only write; \
                     refresh the record itself to update its confirmation height"
                );
            }
            execute_upsert_utxo(&mut utxo_stmt, wallet_id, utxo, false)?;
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
        (wallet_id, outpoint, value, script, spent) \
     VALUES (?1, ?2, ?3, ?4, ?5) \
     ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
        value = excluded.value, \
        script = excluded.script, \
        spent = excluded.spent";

/// Upsert one `core_utxos` row; `spent` marks spent-only synthetic rows.
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
        height_column_u32("core_sync_state.last_processed_height", current_raw.0)?,
        height_column_u32("core_sync_state.synced_height", current_raw.1)?,
    );
    let lp = match (current.0, last_processed) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let sy = match (current.1, synced) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    // Chain lock: monotonic-max by height like the sync watermarks above.
    // A new chain lock replaces the stored one only when its height is >=
    // the stored height, so an out-of-order lower-height update can't
    // regress the finalized checkpoint. `None` (no update) keeps existing.
    let cl_final = match (chain_lock_bytes, current_raw.2) {
        (Some(new_bytes), Some(existing_bytes)) => {
            if chain_lock_height(&new_bytes) >= chain_lock_height(&existing_bytes) {
                Some(new_bytes)
            } else {
                Some(existing_bytes)
            }
        }
        (Some(new_bytes), None) => Some(new_bytes),
        (None, existing) => existing,
    };
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
/// from the `core_*` tables, plus the per-outpoint owning-account side channel.
/// PUBLIC material only; mints no `Wallet`. `network` (from `wallets`) turns a
/// persisted `script` back into an `Address`.
///
/// [`CoreChangeSet::new_utxos`] cannot carry each UTXO's owning account (it is a
/// bare `Vec<Utxo>`), so the returned map surfaces, per unspent outpoint, the
/// funds account that owns it — resolved by matching the UTXO's script against
/// `core_address_pool`. [`apply_persisted_core_state`](crate::sqlite::util::apply_persisted_core_state)
/// consumes it to route each UTXO to its true account. An outpoint whose script
/// matches no pool row is absent from the map and falls back to the first funds
/// account (the one-way historical-attribution default; re-warms on next sync).
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Unspent UTXOs** (`new_utxos`): every `spent = 0` row — the balance
///   source (no-silent-zero); confirmation height comes from the matching
///   `core_transactions` row. A missing row or height loads as unconfirmed.
/// - **Transaction records**: height-only rows supply UTXO confirmation
///   metadata but are not emitted as records. Blob-bearing rows are decoded
///   and checked against their typed txid and height columns.
/// - **IS-locks** / **sync watermarks**: decoded bit-exact, fail-hard on a
///   corrupt blob.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **`is_coinbase` / `is_instantlocked` / `is_trusted` / `used` flags**: not
///   carried by `core_utxos`; defaulted and refreshed on the next scan.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
    ctx: &LoadCtx,
) -> Result<
    (
        CoreChangeSet,
        std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    ),
    WalletStorageError,
> {
    let mut cs = CoreChangeSet::default();
    let mut utxo_accounts: HashMap<dashcore::OutPoint, OwningAccount> = HashMap::new();

    let mut transaction_heights: HashMap<dashcore::Txid, Option<u32>> = HashMap::new();
    let mut blob_backed_transaction_heights = HashSet::new();
    {
        use dashcore::hashes::Hash;

        // Pre-read length gates keep fixed-width txids and record blobs from
        // being materialized before their stored sizes are validated.
        let mut stmt = conn.prepare(
            "SELECT length(txid), txid, height, length(record_blob), record_blob \
             FROM core_transactions WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get::<_, i64>(0)?, 32, "core_transactions.txid")?;
            let txid_bytes: Vec<u8> = row.get(1)?;
            let txid = dashcore::Txid::from_slice(&txid_bytes)?;
            let height =
                height_column_u32("core_transactions.height", row.get::<_, Option<i64>>(2)?)?;
            let mut effective_txid = txid;
            let mut effective_height = height;
            if let Some(record_blob_len) = row.get::<_, Option<i64>>(3)? {
                blob::check_size(record_blob_len)?;
                let payload: Vec<u8> = row.get(4)?;
                let record = blob::decode::<TransactionRecord>(&payload)?;
                effective_txid = record.txid;
                effective_height = record.block_info().map(|block_info| block_info.height());
                if let Err(mismatch) =
                    ensure_transaction_record_matches_columns(&txid, height, &record)
                {
                    // The blob is authoritative, so the projection keeps
                    // using it; the typed columns are left exactly as found.
                    ctx.tolerate(LoadSite::CoreTransactionColumnDrift, mismatch)?;
                }
                cs.records.push(record);
                transaction_heights.insert(effective_txid, effective_height);
                blob_backed_transaction_heights.insert(effective_txid);
            } else if !blob_backed_transaction_heights.contains(&effective_txid) {
                transaction_heights
                    .entry(effective_txid)
                    .or_insert(effective_height);
            }
        }
    }

    // Unspent UTXOs → new_utxos (the balance source).
    // Pre-read `length()` gates on `outpoint` and `script` before materializing
    // the Vec so tampered oversize values are caught before heap allocation.
    // Uses `prepare + query + while let` (not `query_map`) so the typed
    // `BlobTooLarge` error can be returned from the loop body directly.
    {
        let mut stmt = conn.prepare(
            "SELECT length(outpoint), outpoint, value, length(script), script \
             FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            // col 0: length(outpoint) — gate before materializing
            blob::check_size(row.get::<_, i64>(0)?)?;
            let op_bytes: Vec<u8> = row.get(1)?;
            let value: i64 = row.get(2)?;
            // col 3: length(script) — gate before materializing
            blob::check_size(row.get::<_, i64>(3)?)?;
            let script_bytes: Vec<u8> = row.get(4)?;
            let outpoint = blob::decode_outpoint(&op_bytes)?;
            let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
            let height = transaction_heights.get(&outpoint.txid).copied().flatten();
            let script = dashcore::ScriptBuf::from_bytes(script_bytes);
            if let Some(owner) = owning_account_for_script(conn, wallet_id, script.as_bytes())? {
                utxo_accounts.insert(outpoint, owner);
            }
            let address = dashcore::Address::from_script(&script, network)?;
            let utxo = Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value,
                    script_pubkey: script,
                },
                address,
                height: height.unwrap_or(0),
                is_coinbase: false,
                is_confirmed: height.is_some(),
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            };
            cs.new_utxos.push(utxo);
        }
    }

    {
        // Same pre-read length gate as `record_blob` above. `txid` is a raw
        // 32-byte hash, so its width is gated fixed before materializing —
        // an oversize column raises `BlobTooLarge` ahead of the `Vec` alloc
        // rather than materializing then failing in `Txid::from_slice`.
        let mut stmt = conn.prepare(
            "SELECT length(txid), txid, length(islock_blob), islock_blob \
             FROM core_instant_locks WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            use dashcore::hashes::Hash;
            blob::check_fixed_width(row.get::<_, i64>(0)?, 32, "core_instant_locks.txid")?;
            let txid_bytes: Vec<u8> = row.get(1)?;
            blob::check_size(row.get::<_, i64>(2)?)?;
            let blob_bytes: Vec<u8> = row.get(3)?;
            let txid = dashcore::Txid::from_slice(&txid_bytes)?;
            let islock: dashcore::ephemerealdata::instant_lock::InstantLock =
                blob::decode(&blob_bytes)?;
            cs.instant_locks_for_non_final_records.insert(txid, islock);
        }
    }

    // Sync watermarks + persisted chain lock. Read `length()` first so an
    // oversize chain-lock blob is rejected before the Vec is allocated.
    {
        let mut stmt = conn.prepare(
            "SELECT last_processed_height, synced_height, \
                    length(last_applied_chain_lock), last_applied_chain_lock \
             FROM core_sync_state WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        if let Some(row) = rows.next()? {
            let lp: Option<i64> = row.get(0)?;
            let sy: Option<i64> = row.get(1)?;
            // Gate before materializing: NULL length means no chain lock.
            if let Some(n) = row.get::<_, Option<i64>>(2)? {
                blob::check_size(n)?;
            }
            let cl_bytes: Option<Vec<u8>> = row.get(3)?;
            // Fail-hard on an out-of-range watermark (corruption never skipped).
            cs.last_processed_height =
                height_column_u32("core_sync_state.last_processed_height", lp)?;
            cs.synced_height = height_column_u32("core_sync_state.synced_height", sy)?;
            // Policy decides: strict aborts on a corrupt chain-lock blob,
            // recovery leaves the field None for the next ChainLock event.
            if let Some(bytes) = cl_bytes {
                cs.last_applied_chain_lock = decode_chain_lock(&bytes, ctx)?;
            }
        }
    }

    Ok((cs, utxo_accounts))
}

/// Every address that has ever held a `core_utxos` row for this wallet —
/// spent **and** unspent — deduplicated, each paired with its resolved
/// owning account. The rehydration address-reuse guard: an address whose
/// UTXO was since spent must still be marked used so it's never handed back
/// out as a fresh receive address.
///
/// `core_utxos` carries no unambiguous account attribution, so ownership is
/// resolved per script via [`owning_account_for_script`]; the result is
/// `None` when the script matches no pool row (the caller then routes to the
/// first funds account). `network` turns each persisted `script` back into an
/// [`Address`](dashcore::Address); a script that isn't a valid address is a
/// hard error (corruption is never silently dropped), matching [`load_state`]'s
/// unspent-UTXO handling.
pub fn load_used_addresses(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<(dashcore::Address, Option<OwningAccount>)>, WalletStorageError> {
    // Gate the largest stored `script` with a cheap aggregate BEFORE the
    // `DISTINCT ... ORDER BY script` read materializes or sorts any blob, so a
    // corrupt/oversize column raises a typed `BlobTooLarge` (the crate's 16 MiB
    // cap) rather than SQLite's own `TooBig` mid-sort, and never OOMs the host.
    // `core_utxos` has no `(wallet_id, script)` index, so the read would sort
    // the blob; the aggregate gate fires first regardless of query plan.
    let max_script_len: Option<i64> = conn.query_row(
        "SELECT MAX(length(script)) FROM core_utxos WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    if let Some(len) = max_script_len {
        blob::check_size(len)?;
    }
    // Materialize the scripts before resolving ownership: `owning_account_for_script`
    // prepares its own statement on `conn`, so the reader statement must be
    // finished first.
    let scripts: Vec<Vec<u8>> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT script FROM core_utxos WHERE wallet_id = ?1 ORDER BY script",
        )?;
        let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let mut out = Vec::with_capacity(scripts.len());
    for raw in scripts {
        let owner = owning_account_for_script(conn, wallet_id, &raw)?;
        let script = dashcore::ScriptBuf::from_bytes(raw);
        let address = dashcore::Address::from_script(&script, network)?;
        out.push((address, owner));
    }
    Ok(out)
}

/// Convert a stored height column to `u32`, erroring on overflow
/// rather than silently truncating a corrupt/out-of-range value.
fn height_column_u32(
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u32>, WalletStorageError> {
    value
        .map(|v| crate::sqlite::util::safe_cast::i64_to_u32(field, v))
        .transpose()
}

/// Fetch a single transaction record by txid.
///
/// Returns `Ok(None)` when the row is absent or carries only confirmation
/// height metadata.
/// A height-only row is not synthesized because UTXO height is not attested
/// block context and must not masquerade as a `BlockInfo`.
pub fn get_tx_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
    ctx: &LoadCtx,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    // Pre-read `length()` gate before materializing, consistent with the
    // bulk load_state path above.
    let mut stmt = conn.prepare_cached(
        "SELECT height, length(record_blob), record_blob FROM core_transactions \
         WHERE wallet_id = ?1 AND txid = ?2",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let height = height_column_u32("core_transactions.height", row.get::<_, Option<i64>>(0)?)?;
    let Some(record_blob_len) = row.get::<_, Option<i64>>(1)? else {
        return Ok(None);
    };
    blob::check_size(record_blob_len)?;
    let payload: Vec<u8> = row.get(2)?;
    let record = blob::decode(&payload)?;
    drop(rows);
    drop(stmt);
    if let Err(mismatch) = ensure_transaction_record_matches_columns(txid, height, &record) {
        ctx.tolerate(LoadSite::CoreTransactionColumnDrift, mismatch)?;
    }
    Ok(Some(record))
}

/// Diagnose typed txid/height columns that disagree with the authoritative blob.
fn ensure_transaction_record_matches_columns(
    typed_txid: &dashcore::Txid,
    typed_height: Option<u32>,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let blob_height = record.block_info().map(|block_info| block_info.height());
    if record.txid != *typed_txid || blob_height != typed_height {
        return Err(WalletStorageError::CoreTransactionEntryMismatch {
            typed_txid: typed_txid.to_string(),
            blob_txid: record.txid.to_string(),
            typed_height,
            blob_height,
        });
    }
    Ok(())
}

/// Row representing one unspent UTXO. Used by tests that probe the
/// `core_utxos` table without going through full `Wallet` reconstruction.
#[cfg(any(test, feature = "__test-helpers"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnspentRow {
    pub outpoint: dashcore::OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub account_index: u32,
}

/// All UTXOs for a wallet that have not been spent yet, bucketed by the
/// account index resolved from `core_address_pool` during the read.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn list_unspent_utxos(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, Vec<UnspentRow>>, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT outpoint, value, script \
         FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let value: i64 = row.get(1)?;
        let script: Vec<u8> = row.get(2)?;
        Ok((op_bytes, value, script))
    })?;
    let mut by_account: BTreeMap<u32, Vec<UnspentRow>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, value, script_bytes) = r?;
        let outpoint = blob::decode_outpoint(&op_bytes)?;
        let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
        let account_index = owning_account_for_script(conn, wallet_id, &script_bytes)?
            .map(|owner| owner.account_index)
            .unwrap_or(0);
        let row = UnspentRow {
            outpoint,
            value,
            script: script_bytes,
            account_index,
        };
        by_account.entry(account_index).or_default().push(row);
    }
    Ok(by_account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, OutPoint, PubkeyHash, Transaction, TxOut, Txid};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

    fn transaction_record(txid: Txid, context: TransactionContext) -> TransactionRecord {
        let mut record = TransactionRecord::new(
            Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            100,
        );
        record.txid = txid;
        record
    }

    fn sample_utxo(txid: Txid, height: u32, is_confirmed: bool) -> Utxo {
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0x23u8; 20])),
        );
        Utxo {
            outpoint: OutPoint { txid, vout: 0 },
            txout: TxOut {
                value: 150_000,
                script_pubkey: address.script_pubkey(),
            },
            address,
            height,
            is_coinbase: false,
            is_confirmed,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        }
    }

    fn sample_chain_lock(height: u32) -> ChainLock {
        ChainLock {
            block_height: height,
            block_hash: BlockHash::from_byte_array([0x11u8; 32]),
            signature: [0x22u8; 96].into(),
        }
    }

    /// A tampered `core_instant_locks.txid` that overflows the blob cap must
    /// raise `BlobTooLarge` from the fixed-width gate BEFORE the oversize `Vec`
    /// is materialized — not `BlobDecode` after `Txid::from_slice` on a
    /// multi-megabyte allocation.
    #[test]
    fn load_state_rejects_oversize_instant_lock_txid() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let w = [0xABu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();

        // Plant a txid one byte past the 16 MiB cap; islock_blob content is
        // irrelevant — the txid gate fires before it is read.
        let oversize_txid = vec![0u8; crate::SIZE_LIMIT_BYTES + 1];
        conn.execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, ?3)",
            params![&w[..], oversize_txid.as_slice(), &[0u8; 4][..]],
        )
        .unwrap();

        let err = load_state(&conn, &w, dashcore::Network::Testnet, &LoadCtx::strict())
            .expect_err("load_state must reject an oversize instant-lock txid");
        assert!(
            matches!(err, WalletStorageError::BlobTooLarge { .. }),
            "expected BlobTooLarge from the pre-materialization gate, got {err:?}"
        );
    }

    #[test]
    fn load_state_reconciles_utxo_height_from_confirmed_transaction_record() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x42u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x7Eu8; 32]);
        let utxo = sample_utxo(txid, 123, true);
        let outpoint = utxo.outpoint;

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    records: vec![transaction_record(txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let confirmed_height = 321;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            confirmed_height,
                            BlockHash::from_byte_array([0x34u8; 32]),
                            1_735_689_600,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state
            .new_utxos
            .iter()
            .find(|candidate| candidate.outpoint == outpoint)
            .expect("matching UTXO must be loaded");
        assert_eq!(loaded.height, confirmed_height);
        assert!(loaded.is_confirmed);
    }

    #[test]
    fn load_state_restores_confirmed_recordless_utxo_height() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x44u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x80u8; 32]);
        let utxo = sample_utxo(txid, 456, true);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state.new_utxos.first().expect("recordless UTXO must load");
        assert_eq!(loaded.height, 456);
        assert!(loaded.is_confirmed);
        assert!(state.records.is_empty());
        assert!(get_tx_record(&conn, &wallet_id, &txid, &LoadCtx::strict())
            .unwrap()
            .is_none());
    }

    #[test]
    fn height_only_placeholder_does_not_regress() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x49u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x86u8; 32]);
        let mut utxo = sample_utxo(txid, 500, true);
        for stale in [
            CoreChangeSet {
                new_utxos: vec![utxo.clone()],
                ..Default::default()
            },
            {
                utxo.height = 400;
                CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                }
            },
            {
                utxo.height = 0;
                utxo.is_confirmed = false;
                CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                }
            },
        ] {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &stale).unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 500);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn load_state_treats_height_zero_as_confirmed() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Au8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x87u8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                new_utxos: vec![sample_utxo(txid, 0, true)],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 0);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn transaction_record_always_overrides_height_only_placeholder() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x45u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x81u8; 32]);
        let mut utxo = sample_utxo(txid, 456, true);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        utxo.height = 789;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 0);
        assert!(!state.new_utxos[0].is_confirmed);
        assert_eq!(state.records.len(), 1);

        let confirmed_height = 900;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            confirmed_height,
                            BlockHash::from_byte_array([0x35u8; 32]),
                            1_735_689_700,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, confirmed_height);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn load_state_defaults_utxo_without_transaction_record_to_unconfirmed() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x43u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x7Fu8; 32]);
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0x24u8; 20])),
        );
        let outpoint = OutPoint { txid, vout: 0 };
        let utxo = Utxo::new(
            outpoint,
            TxOut {
                value: 175_000,
                script_pubkey: address.script_pubkey(),
            },
            address,
            777,
            false,
        );

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state
            .new_utxos
            .iter()
            .find(|candidate| candidate.outpoint == outpoint)
            .expect("matching UTXO must be loaded");
        assert_eq!(loaded.height, 0);
        assert!(!loaded.is_confirmed);
    }

    #[test]
    fn load_state_tolerates_transaction_blob_txid_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x46u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let blob_txid = Txid::from_byte_array([0x82u8; 32]);
        let typed_txid = Txid::from_byte_array([0x83u8; 32]);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(blob_txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }
        conn.execute(
            "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
            params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
        )
        .unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::recovery(),
        )
        .expect("recovery mode must reconstruct from the authoritative blob");
        assert_eq!(state.records[0].txid, blob_txid);
        let on_disk: Vec<u8> = conn
            .query_row(
                "SELECT txid FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            AsRef::<[u8]>::as_ref(&typed_txid),
            "a read must never rewrite the row it read"
        );
    }

    #[test]
    fn load_state_blob_height_wins_over_drifted_typed_column_in_either_scan_order() {
        for (case, typed_byte) in [0x10, 0xF0].into_iter().enumerate() {
            let mut conn = rusqlite::Connection::open_in_memory().unwrap();
            crate::sqlite::migrations::run(&mut conn).unwrap();
            let wallet_id = [0x50 + case as u8; 32];
            conn.execute(
                "INSERT INTO wallets (wallet_id, network, birth_height) \
                 VALUES (?1, 'testnet', 0)",
                params![&wallet_id[..]],
            )
            .unwrap();

            let blob_txid = Txid::from_byte_array([0x80; 32]);
            let typed_txid = Txid::from_byte_array([typed_byte; 32]);
            let confirmed_utxo = sample_utxo(blob_txid, 500, true);
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![confirmed_utxo.clone()],
                    records: vec![transaction_record(
                        blob_txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            500,
                            BlockHash::from_byte_array([0x38; 32]),
                            1_735_690_000,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
            conn.execute(
                "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
                params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
            )
            .unwrap();

            let mut stale_utxo = confirmed_utxo;
            stale_utxo.height = 100;
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![stale_utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();

            let (state, _) = load_state(
                &conn,
                &wallet_id,
                dashcore::Network::Testnet,
                &LoadCtx::recovery(),
            )
            .expect("recovery mode must still reconstruct blob-authoritative state");
            let loaded = state.new_utxos.first().expect("UTXO must load");
            assert_eq!(loaded.height, 500, "failed scan-order case {case}");
            assert!(loaded.is_confirmed);
        }
    }

    #[test]
    fn load_state_tolerates_transaction_blob_height_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x47u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x84u8; 32]);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            500,
                            BlockHash::from_byte_array([0x36u8; 32]),
                            1_735_689_800,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }
        conn.execute(
            "UPDATE core_transactions SET height = 501 WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
        )
        .unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::recovery(),
        )
        .expect("recovery mode must reconstruct from the authoritative blob");
        assert_eq!(state.records[0].height(), Some(500));
        let on_disk: Option<i64> = conn
            .query_row(
                "SELECT height FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            Some(501),
            "a read must never rewrite the row it read"
        );
    }

    #[test]
    fn get_tx_record_tolerates_blob_txid_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Bu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let blob_txid = Txid::from_byte_array([0x88u8; 32]);
        let typed_txid = Txid::from_byte_array([0x89u8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                records: vec![transaction_record(blob_txid, TransactionContext::Mempool)],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        conn.execute(
            "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
            params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
        )
        .unwrap();

        let record = get_tx_record(&conn, &wallet_id, &typed_txid, &LoadCtx::recovery())
            .expect("recovery mode must still serve the point read")
            .expect("blob-bearing row must return its record");
        assert_eq!(record.txid, blob_txid);
        // The row was NOT repaired, so the blob txid still matches no row.
        assert!(
            get_tx_record(&conn, &wallet_id, &blob_txid, &LoadCtx::recovery())
                .unwrap()
                .is_none(),
            "a read must never rewrite the row it read"
        );
    }

    #[test]
    fn get_tx_record_tolerates_blob_height_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Cu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x8Au8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                records: vec![transaction_record(
                    txid,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        600,
                        BlockHash::from_byte_array([0x37u8; 32]),
                        1_735_689_900,
                    )),
                )],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        conn.execute(
            "UPDATE core_transactions SET height = 601 WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
        )
        .unwrap();

        let record = get_tx_record(&conn, &wallet_id, &txid, &LoadCtx::recovery())
            .expect("recovery mode must still serve the point read")
            .expect("blob-bearing row must return its record");
        assert_eq!(record.height(), Some(600));
        let on_disk: Option<i64> = conn
            .query_row(
                "SELECT height FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            Some(601),
            "a read must never rewrite the row it read"
        );
    }

    /// `load_used_addresses` (the address-reuse-guard rehydration path called
    /// from `persister.rs`) must surface `AddressDecode` — carrying the
    /// upstream `dashcore::address::Error` — when a stored `core_utxos.script`
    /// parses as bytes but not as an address, not the context-free `BlobDecode`.
    #[test]
    fn load_used_addresses_wraps_address_error_as_address_decode() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let w = [0x99u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        // A bare OP_RETURN script is well-formed bytes but not any address
        // type, so `Address::from_script` returns `UnrecognizedScript`.
        let bad_script = [0x6au8];
        conn.execute(
            "INSERT INTO core_utxos \
                (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 0, ?3, 0)",
            params![&w[..], &[0u8; 36][..], &bad_script[..]],
        )
        .unwrap();

        let err = load_used_addresses(&conn, &w, dashcore::Network::Testnet)
            .expect_err("an unparseable script must be a hard error");
        assert!(
            matches!(err, WalletStorageError::AddressDecode { .. }),
            "expected AddressDecode carrying the upstream error, got {err:?}"
        );
    }

    #[test]
    fn chain_lock_height_rejects_trailing_bytes() {
        let bytes = encode_chain_lock(&sample_chain_lock(100_000)).expect("encode");
        assert_eq!(chain_lock_height(&bytes), Some(100_000));

        // A corrupt blob (valid prefix + trailing garbage) must not yield a
        // height, else it stays stuck atop later valid lower-height updates.
        let mut corrupt = bytes.clone();
        corrupt.extend_from_slice(&[0xFFu8; 4]);
        assert_eq!(chain_lock_height(&corrupt), None);
    }
}
