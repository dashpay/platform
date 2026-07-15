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
use crate::sqlite::schema::core_pool::{owning_account_for_script, OwningAccount};

// PUBLIC material only: core-chain state reaching `record_blob` /
// `islock_blob` (transaction records + InstantLocks are public chain data).
impl_persistable_blob!(TransactionRecord, dashcore::InstantLock);

/// Encode a `ChainLock` to bytes for storage in `core_sync_state`.
fn encode_chain_lock(cl: &ChainLock) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::encode_to_vec(cl, blob::bounded_config())?)
}

/// Decode a `ChainLock` from `core_sync_state.last_applied_chain_lock`.
/// Returns `None` + emits a `tracing::warn` on any decode failure so a
/// single corrupt byte cannot prevent the wallet from loading (the next
/// ChainLock event will repopulate the column).
fn decode_chain_lock_soft(bytes: &[u8]) -> Option<ChainLock> {
    match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        // Reject a valid-prefix + trailing-garbage payload (bincode stops
        // after the typed length) the same way the BLOB decoders do.
        Ok((cl, consumed)) if consumed == bytes.len() => Some(cl),
        Ok(_) => {
            tracing::warn!(
                "core_sync_state.last_applied_chain_lock: trailing bytes after \
                 ChainLock; field left None — the next ChainLock sync will repopulate"
            );
            None
        }
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

/// Block height of an encoded `last_applied_chain_lock` blob, or `None` if it
/// can't be decoded. Used to monotonic-max-merge the chain lock so an
/// out-of-order lower-height update never regresses the finalized checkpoint.
fn chain_lock_height(bytes: &[u8]) -> Option<u32> {
    match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        // Require full consumption (like `decode_chain_lock_soft`) so a corrupt
        // stored blob can't out-rank a later valid update and stay stuck.
        Ok((cl, consumed)) if consumed == bytes.len() => Some(cl.block_height),
        _ => None,
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
    // `addresses_derived` is intentionally NOT persisted here — the pool
    // snapshot (`account_address_pools`) is the derived-address source, and
    // it is applied to `core_address_pool` before this in the same flush tx,
    // so a UTXO's owning account resolves by matching its script against a
    // pool row (falling back to account 0 when no pool row covers it).
    if !cs.new_utxos.is_empty() {
        let mut stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.new_utxos {
            let account_index = resolve_account_index(tx, wallet_id, utxo)?;
            execute_upsert_utxo(&mut stmt, wallet_id, utxo, account_index, false)?;
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
                // Spent-only synthetic row for a UTXO we never saw unspent;
                // attribute like any other row (inert — spent rows are
                // excluded from `list_unspent_utxos`).
                let account_index = resolve_account_index(tx, wallet_id, utxo)?;
                execute_upsert_utxo(&mut upsert_stmt, wallet_id, utxo, account_index, true)?;
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

/// Owning account for a UTXO, resolved by matching its `script_pubkey`
/// against a `core_address_pool` row. Falls back to account 0 when no pool
/// row covers the script — the one-way historical-attribution default (R7):
/// funds are never dropped, only conservatively bucketed.
fn resolve_account_index(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
) -> Result<i64, WalletStorageError> {
    let script = utxo.txout.script_pubkey.as_bytes();
    let account =
        crate::sqlite::schema::core_pool::account_index_for_script(tx, wallet_id, script)?
            .unwrap_or(0);
    Ok(i64::from(account))
}

/// Upsert one `core_utxos` row with its resolved `account_index`; `spent`
/// marks spent-only synthetic rows.
fn execute_upsert_utxo(
    stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    account_index: i64,
    spent: bool,
) -> Result<(), WalletStorageError> {
    let op = blob::encode_outpoint(&utxo.outpoint)?;
    stmt.execute(params![
        wallet_id.as_slice(),
        &op[..],
        crate::sqlite::util::safe_cast::u64_to_i64("core_utxos.value", utxo.value())?,
        utxo.txout.script_pubkey.as_bytes(),
        i64::from(utxo.height),
        account_index,
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
///   source (no-silent-zero); a row with a block `height` is confirmed.
/// - **Transaction records** / **IS-locks** / **sync watermarks**: decoded
///   bit-exact, fail-hard on a corrupt blob.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **`is_coinbase` / `is_instantlocked` / `is_trusted` / `used` flags**: not
///   carried by `core_utxos`; defaulted and refreshed on the next scan.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<
    (
        CoreChangeSet,
        std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    ),
    WalletStorageError,
> {
    let mut cs = CoreChangeSet::default();
    let mut utxo_accounts: std::collections::HashMap<dashcore::OutPoint, OwningAccount> =
        std::collections::HashMap::new();

    // Unspent UTXOs → new_utxos (the balance source).
    // Pre-read `length()` gates on `outpoint` and `script` before materializing
    // the Vec so tampered oversize values are caught before heap allocation.
    // Uses `prepare + query + while let` (not `query_map`) so the typed
    // `BlobTooLarge` error can be returned from the loop body directly.
    {
        let mut stmt = conn.prepare(
            "SELECT length(outpoint), outpoint, value, length(script), script, height \
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
            let height: Option<i64> = row.get(5)?;
            let outpoint = blob::decode_outpoint(&op_bytes)?;
            let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
            let height_u32 = match height {
                None => 0u32,
                Some(h) => crate::sqlite::util::safe_cast::i64_to_u32("core_utxos.height", h)?,
            };
            let script = dashcore::ScriptBuf::from_bytes(script_bytes);
            if let Some(owner) = owning_account_for_script(conn, wallet_id, script.as_bytes())? {
                utxo_accounts.insert(outpoint, owner);
            }
            let address = dashcore::Address::from_script(&script, network)?;
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
        // Pre-read `length()` gate (O(1) from the row header) before
        // materializing the blob so a tampered oversize `record_blob` can't
        // force a multi-gigabyte allocation.
        let mut stmt = conn.prepare(
            "SELECT length(record_blob), record_blob FROM core_transactions WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_size(row.get::<_, i64>(0)?)?;
            let payload: Vec<u8> = row.get(1)?;
            cs.records
                .push(blob::decode::<TransactionRecord>(&payload)?);
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
                sync_height_u32("core_sync_state.last_processed_height", lp)?;
            cs.synced_height = sync_height_u32("core_sync_state.synced_height", sy)?;
            // Soft-fail on a corrupt chain-lock blob — a single bad byte must
            // not prevent loading; the next ChainLock event repopulates.
            if let Some(bytes) = cl_bytes {
                cs.last_applied_chain_lock = decode_chain_lock_soft(&bytes);
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
        let address = dashcore::Address::from_script(&script, network)
            .map_err(|_| WalletStorageError::blob_decode("core_utxos.script not an address"))?;
        out.push((address, owner));
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
    // Pre-read `length()` gate before materializing, consistent with the
    // bulk load_state path above.
    let mut stmt = conn.prepare(
        "SELECT length(record_blob), record_blob FROM core_transactions \
         WHERE wallet_id = ?1 AND txid = ?2",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    blob::check_size(row.get::<_, i64>(0)?)?;
    let payload: Vec<u8> = row.get(1)?;
    Ok(Some(blob::decode(&payload)?))
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

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::BlockHash;

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

        let err = load_state(&conn, &w, dashcore::Network::Testnet)
            .expect_err("load_state must reject an oversize instant-lock txid");
        assert!(
            matches!(err, WalletStorageError::BlobTooLarge { .. }),
            "expected BlobTooLarge from the pre-materialization gate, got {err:?}"
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
