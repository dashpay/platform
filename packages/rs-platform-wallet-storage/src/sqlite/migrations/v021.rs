//! Rewind coverage without discarding any wallet's unresolved InstantSend evidence.

use dashcore::{hashes::Hash, Txid};
use key_wallet::transaction_checking::TransactionContext;
use rusqlite::{params, Row, Transaction};

use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{asset_locks, blob, core_state, wallets};
use crate::sqlite::util::safe_cast::{i64_to_u32, u64_to_i64};

pub(super) fn rescan(tx: &Transaction<'_>, policy: LoadPolicy) -> Result<(), WalletStorageError> {
    rewind_sync_heights(tx, policy)?;
    tx.execute_batch(
        "CREATE TEMP TABLE core_rescan_preserved_wallets (wallet_id BLOB PRIMARY KEY)",
    )?;
    for wallet_id in wallets::list_ids(tx)? {
        let has_raw: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM core_instant_locks WHERE wallet_id=?1)
                 OR EXISTS(SELECT 1 FROM asset_locks WHERE wallet_id=?1 AND status='is_locked')",
            [wallet_id.as_slice()],
            |row| row.get(0),
        )?;
        // These bytes preserve the entire conflict graph; they do not grant finality.
        if has_raw
            || has_record_observation(tx, &wallet_id, policy)?
            || has_asset_observation(tx, &wallet_id, policy)?
        {
            tx.execute(
                "INSERT INTO core_rescan_preserved_wallets(wallet_id) VALUES (?1)",
                [wallet_id.as_slice()],
            )?;
        }
    }
    tx.execute_batch(
        "DELETE FROM core_utxos WHERE wallet_id NOT IN (SELECT wallet_id FROM core_rescan_preserved_wallets) AND NOT (
            EXISTS (SELECT 1 FROM core_transactions t WHERE t.wallet_id = core_utxos.wallet_id
                AND t.txid = substr(core_utxos.outpoint, 2, 32) AND t.height IS NULL)
            OR (spent = 1 AND winner_mined_height IS NULL AND NOT EXISTS (
                SELECT 1 FROM core_transactions t WHERE t.wallet_id = core_utxos.wallet_id
                    AND t.txid = core_utxos.spent_in_txid AND t.height IS NOT NULL))
        );
        DELETE FROM core_instant_locks WHERE wallet_id NOT IN (SELECT wallet_id FROM core_rescan_preserved_wallets)
            AND NOT EXISTS (SELECT 1 FROM core_transactions t WHERE t.wallet_id = core_instant_locks.wallet_id
                AND t.txid = core_instant_locks.txid AND t.height IS NULL);
        DELETE FROM core_transaction_inputs WHERE wallet_id NOT IN (SELECT wallet_id FROM core_rescan_preserved_wallets)
            AND EXISTS (SELECT 1 FROM core_transactions t WHERE t.wallet_id = core_transaction_inputs.wallet_id
                AND t.txid = core_transaction_inputs.txid AND t.height IS NOT NULL);
        DELETE FROM core_transactions WHERE wallet_id NOT IN (SELECT wallet_id FROM core_rescan_preserved_wallets)
            AND height IS NOT NULL AND NOT EXISTS (SELECT 1 FROM core_utxos u
                WHERE u.wallet_id = core_transactions.wallet_id AND substr(u.outpoint, 2, 32) = core_transactions.txid);
        UPDATE core_transactions SET block_hash = NULL, block_time = NULL, finalized = 0, record_blob = NULL
            WHERE wallet_id NOT IN (SELECT wallet_id FROM core_rescan_preserved_wallets) AND height IS NOT NULL;
        DROP TABLE core_rescan_preserved_wallets;",
    )?;
    Ok(())
}

fn has_record_observation(
    tx: &Transaction<'_>,
    wallet_id: &[u8; 32],
    policy: LoadPolicy,
) -> Result<bool, WalletStorageError> {
    scan_observations(tx, wallet_id, policy,
        "SELECT CASE WHEN typeof(record_blob)='blob' THEN length(record_blob) END,
                CASE WHEN typeof(record_blob)='blob' AND length(record_blob)<=?2 THEN record_blob END,
                length(txid), CASE WHEN length(txid)=32 THEN txid END, height
         FROM core_transactions WHERE wallet_id=?1 AND record_blob IS NOT NULL",
        |bytes, row| {
            let record = core_state::decode_transaction_record(bytes)?;
            blob::check_fixed_width(row.get(2)?, 32, "core_transactions.txid")?;
            let txid = Txid::from_slice(&row.get::<_, Vec<u8>>(3)?)?;
            let height = row.get::<_, Option<i64>>(4)?
                .map(|height| i64_to_u32("core_transactions.height", height)).transpose()?;
            core_state::ensure_transaction_record_matches_columns(&txid, height, &record)?;
            Ok(matches!(record.context, TransactionContext::InstantSend(_)))
        })
}

fn has_asset_observation(
    tx: &Transaction<'_>,
    wallet_id: &[u8; 32],
    policy: LoadPolicy,
) -> Result<bool, WalletStorageError> {
    scan_observations(tx, wallet_id, policy,
        "SELECT CASE WHEN typeof(lifecycle_blob)='blob' THEN length(lifecycle_blob) END,
                CASE WHEN typeof(lifecycle_blob)='blob' AND length(lifecycle_blob)<=?2 THEN lifecycle_blob END
         FROM asset_locks WHERE wallet_id=?1",
        |bytes, _| asset_locks::has_instant_observation(bytes))
}

fn scan_observations(
    tx: &Transaction<'_>,
    wallet_id: &[u8; 32],
    policy: LoadPolicy,
    query: &str,
    decode: impl Fn(&[u8], &Row<'_>) -> Result<bool, WalletStorageError>,
) -> Result<bool, WalletStorageError> {
    let mut stmt = tx.prepare(query)?;
    let mut rows = stmt.query(params![
        wallet_id.as_slice(),
        u64_to_i64("record_blob.limit", blob::BLOB_SIZE_LIMIT_BYTES as u64)?
    ])?;
    while let Some(row) = rows.next()? {
        let length = row.get::<_, Option<i64>>(0)?;
        let decoded = match length {
            Some(length) => blob::check_size(length).and_then(|()| {
                let bytes: Vec<u8> = row.get(1)?;
                decode(&bytes, row)
            }),
            None => Err(WalletStorageError::blob_decode(
                "rescan evidence must be a BLOB",
            )),
        };
        match decoded {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(error) if policy == LoadPolicy::Recovery && !error.is_transient() => {
                tracing::warn!(wallet_id = %hex::encode(wallet_id), %error,
                    "V021 preserved wallet history because finality evidence is unreadable");
                return Ok(true);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}

/// Validate every birth height before V021 can discard mined history.
fn rewind_sync_heights(tx: &Transaction<'_>, policy: LoadPolicy) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare("SELECT rowid, birth_height FROM wallets")?;
    let mut rows = stmt.query([])?;
    let mut rewinds = Vec::new();
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let stored_birth = row.get::<_, i64>(1).map_err(WalletStorageError::from);
        let (rewind, full_rescan) = match stored_birth
            .and_then(|birth| i64_to_u32("wallets.birth_height", birth))
        {
            Ok(birth) => (birth.saturating_sub(1), false),
            Err(error) => match policy {
                LoadPolicy::Strict => return Err(error),
                LoadPolicy::Recovery => {
                    tracing::warn!(rowid, error_kind = error.error_kind_str(), %error,
                        "V021 rewound the sync watermark to zero for an invalid birth height; wallet metadata is unchanged");
                    (0, true)
                }
            },
        };
        rewinds.push((rowid, rewind, full_rescan));
    }
    for (rowid, rewind, full_rescan) in rewinds {
        tx.execute(
            "UPDATE core_sync_state SET synced_height = CASE WHEN ?3 THEN 0
                ELSE MIN(COALESCE(synced_height, ?2), ?2) END
             WHERE wallet_id = (SELECT wallet_id FROM wallets WHERE rowid = ?1)",
            params![rowid, rewind, full_rescan],
        )?;
    }
    Ok(())
}
