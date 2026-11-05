//! SQL adapter for wallet-owned replay effects.

use crate::sqlite::util::safe_cast::i64_to_u64;
use dashcore::{hashes::Hash, OutPoint, ScriptBuf, TxOut, Txid};
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::manager::history_replay::HistoryReplay;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::slice;

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
) -> Result<(), WalletStorageError> {
    apply_authorization(tx, wallet_id, outcome)?;
    let claims = load_spent_claims(tx, wallet_id)?;
    let held = outcome.protect_durable_claims(&claims);
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

/// Stored coin material is validated against wallet address ownership by the owner.
pub(crate) fn load_prevouts(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<OutPoint, TxOut>, WalletStorageError> {
    let mut coins = BTreeMap::new();
    let limit = i64::try_from(blob::BLOB_SIZE_LIMIT_BYTES).unwrap_or(i64::MAX);
    let mut stmt = conn.prepare_cached("SELECT length(outpoint), CASE WHEN length(outpoint) <= ?2 THEN outpoint END, value, length(script), CASE WHEN length(script) <= ?2 THEN script END FROM core_utxos WHERE wallet_id = ?1 AND is_sweep_placeholder = 0")?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), limit])?;
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        blob::check_size(row.get(3)?)?;
        let outpoint: Vec<u8> = row.get(1)?;
        let script: Vec<u8> = row.get(4)?;
        coins.insert(
            blob::decode_outpoint(&outpoint)?,
            TxOut {
                value: i64_to_u64("core_utxos.value", row.get(2)?)?,
                script_pubkey: ScriptBuf::from_bytes(script),
            },
        );
    }
    Ok(coins)
}

/// Map canonical owner verdicts to physical rows without inferring authorization in SQL.
fn apply_authorization(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    outcome: &HistoryReplay,
) -> Result<(), WalletStorageError> {
    let rejected: HashSet<_> = outcome.authorization.rejected.iter().copied().collect();
    if rejected.is_empty() {
        return Ok(());
    }
    let mut record_keys: HashMap<Txid, Vec<Txid>> = HashMap::new();
    let limit = i64::try_from(blob::BLOB_SIZE_LIMIT_BYTES).unwrap_or(i64::MAX);
    let mut stmt = tx.prepare_cached("SELECT length(txid), CASE WHEN length(txid) = 32 THEN txid END, length(record_blob), CASE WHEN length(record_blob) <= ?2 THEN record_blob END FROM core_transactions WHERE wallet_id = ?1 AND record_blob IS NOT NULL")?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), limit])?;
    while let Some(row) = rows.next()? {
        blob::check_fixed_width(row.get(0)?, 32, "core_transactions.txid")?;
        blob::check_size(row.get(2)?)?;
        let key: Vec<u8> = row.get(1)?;
        let raw: Vec<u8> = row.get(3)?;
        let record = core_state::decode_transaction_record(&raw)?;
        record_keys
            .entry(record.transaction.txid())
            .or_default()
            .push(Txid::from_slice(&key)?);
    }
    let survivors: HashMap<_, _> = outcome
        .authorization
        .surviving_claimants
        .iter()
        .filter_map(|(outpoint, claims)| claims.first().map(|claim| (*outpoint, *claim)))
        .collect();
    let physical_keys: HashSet<Txid> = rejected
        .iter()
        .flat_map(|txid| {
            record_keys
                .get(txid)
                .map(Vec::as_slice)
                .unwrap_or_else(|| slice::from_ref(txid))
        })
        .copied()
        .collect();
    let surviving_txids: HashSet<Txid> = record_keys
        .keys()
        .filter(|txid| !rejected.contains(*txid))
        .copied()
        .collect();
    let protected_keys: HashSet<Txid> = physical_keys
        .intersection(&surviving_txids)
        .copied()
        .collect();
    // A corrupt physical key can also name a surviving body: its evidence belongs to that survivor.
    let rejected_evidence: HashSet<Txid> = rejected
        .union(&physical_keys)
        .filter(|txid| !surviving_txids.contains(*txid))
        .copied()
        .collect();
    let mut protected_claims = Vec::new();
    // Release before deleting bodies: their cleanup trigger clears claimant IDs.
    // Never reinterpret an anonymous or recordless final claim as a rejected peer claim.
    let mut claims = Vec::new();
    let mut stmt = tx.prepare_cached("SELECT length(outpoint), CASE WHEN length(outpoint) <= ?2 THEN outpoint END, length(spent_in_txid), CASE WHEN length(spent_in_txid) = 32 THEN spent_in_txid END, is_sweep_placeholder FROM core_utxos WHERE wallet_id = ?1 AND spent = 1 AND spent_in_txid IS NOT NULL")?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), limit])?;
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        let raw: Vec<u8> = row.get(1)?;
        blob::check_fixed_width(row.get(2)?, 32, "core_utxos.spent_in_txid")?;
        let claimant: Vec<u8> = row.get(3)?;
        let claimant = Txid::from_slice(&claimant)?;
        if protected_keys.contains(&claimant) {
            protected_claims.push((raw, claimant));
        } else if rejected_evidence.contains(&claimant) {
            claims.push((raw, row.get::<_, bool>(4)?));
        }
    }
    for (raw, placeholder) in claims {
        let op = blob::decode_outpoint(&raw)?;
        if let Some(survivor) = survivors.get(&op) {
            if protected_keys.contains(survivor) {
                protected_claims.push((raw.clone(), *survivor));
            }
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
    for txid in &physical_keys {
        tx.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        )?;
    }
    // The row-delete trigger clears claims by physical key, even when that key
    // names a surviving blob. Restore only the claims we just protected.
    for (raw, claimant) in protected_claims {
        tx.execute("UPDATE core_utxos SET spent_in_txid = ?3 WHERE wallet_id = ?1 AND outpoint = ?2 AND spent = 1 AND spent_in_txid IS NULL",
            params![wallet_id.as_slice(), raw, claimant.as_byte_array().as_slice()])?;
    }
    for txid in &rejected_evidence {
        let key = params![wallet_id.as_slice(), txid.as_byte_array().as_slice()];
        tx.execute(
            "DELETE FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
            key,
        )?;
        // `encode_outpoint_txid_occupies_bytes_two_to_thirty_three` pins this prefix.
        tx.execute(
            "DELETE FROM core_utxos \
             WHERE wallet_id = ?1 AND substr(outpoint, 2, 32) = ?2",
            key,
        )?;
    }
    asset_locks::remove_swept(tx, wallet_id, &rejected_evidence)?;
    dashpay::settle_swept_payments(tx, wallet_id, &rejected_evidence)?;
    Ok(())
}
