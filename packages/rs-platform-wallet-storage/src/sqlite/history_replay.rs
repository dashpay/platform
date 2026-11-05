//! SQL adapter for wallet-owned replay effects.

use dashcore::{hashes::Hash, OutPoint, Txid};
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::manager::history_replay::HistoryReplay;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};

use crate::sqlite::schema::{asset_locks, blob, core_state, dashpay};
use crate::WalletStorageError;

/// Decode durable claim provenance from the same snapshot as wallet history.
pub(crate) fn load_spent_claims(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<(OutPoint, Option<Txid>)>, WalletStorageError> {
    // SQLite evaluates every result column when the row is stepped, but a
    // CASE lazily: a payload past its gate is never loaded, and the length
    // beside it reports the typed error below.
    let mut stmt = conn.prepare_cached(
        "SELECT length(outpoint), CASE WHEN length(outpoint) <= ?2 THEN outpoint END, \
                length(spent_in_txid), \
                CASE WHEN length(spent_in_txid) = 32 THEN spent_in_txid END \
         FROM core_utxos WHERE wallet_id = ?1 AND spent = 1",
    )?;
    let limit = i64::try_from(blob::BLOB_SIZE_LIMIT_BYTES).unwrap_or(i64::MAX);
    let mut rows = stmt.query(params![wallet_id.as_slice(), limit])?;
    let mut outpoints = Vec::new();
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        let bytes: Vec<u8> = row.get(1)?;
        let claimant = if let Some(length) = row.get::<_, Option<i64>>(2)? {
            blob::check_fixed_width(length, 32, "core_utxos.spent_in_txid")?;
            let raw: Vec<u8> = row.get(3)?;
            Some(Txid::from_slice(&raw)?)
        } else {
            None
        };
        outpoints.push((blob::decode_outpoint(&bytes)?, claimant));
    }
    Ok(outpoints)
}

/// Apply an owner's effects atomically, preserving claim provenance it protects.
pub(crate) fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    outcome: &mut HistoryReplay,
    claims: &[(OutPoint, Option<Txid>)],
) -> Result<(), WalletStorageError> {
    let held = outcome.protect_durable_claims(claims);
    let mut preserved = Vec::new();
    for outpoint in held {
        let key = blob::encode_outpoint(&outpoint)?;
        let (claimant, height): (Option<Vec<u8>>, Option<i64>) = tx.query_row(
            "SELECT spent_in_txid, winner_mined_height FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            params![wallet_id.as_slice(), &key[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        preserved.push((key, claimant, height));
    }
    asset_locks::remove_swept(tx, wallet_id, &outcome.defeated)?;
    dashpay::settle_swept_payments(tx, wallet_id, &outcome.defeated)?;
    core_state::apply(
        tx,
        wallet_id,
        &CoreChangeSet {
            sweeps: outcome.sweeps.clone(),
            ..Default::default()
        },
    )?;
    for (outpoint, claimant, height) in preserved {
        tx.execute(
            "UPDATE core_utxos SET spent = 1, spent_in_txid = ?3, winner_mined_height = ?4 WHERE wallet_id = ?1 AND outpoint = ?2",
            params![wallet_id.as_slice(), &outpoint[..], claimant, height],
        )?;
    }
    Ok(())
}
