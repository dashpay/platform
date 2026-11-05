//! SQL adapter for wallet-owned replay effects.

use crate::sqlite::util::safe_cast::{i64_to_u32, i64_to_u64};
use dashcore::{hashes::Hash, OutPoint, ScriptBuf, TxOut, Txid};
use platform_wallet::manager::history_replay::{
    HistoryReplayEffects, ReplayClaimEffect, StoredReplayClaim,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};
use std::collections::BTreeMap;

use crate::sqlite::schema::{asset_locks, blob, dashpay};
use crate::WalletStorageError;

/// Decode durable claim provenance from the same snapshot as wallet history.
pub(crate) fn load_spent_claims(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<StoredReplayClaim>, WalletStorageError> {
    // SQLite evaluates every result column when the row is stepped, but a
    // CASE lazily: a payload past its gate is never loaded, and the length
    // beside it reports the typed error below.
    let mut stmt = conn.prepare_cached(
        "SELECT length(outpoint), CASE WHEN length(outpoint) <= ?2 THEN outpoint END, \
                length(spent_in_txid), \
                CASE WHEN length(spent_in_txid) = 32 THEN spent_in_txid END, winner_mined_height \
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
        outpoints.push(StoredReplayClaim {
            outpoint: blob::decode_outpoint(&bytes)?,
            claimant,
            mined_height: row
                .get::<_, Option<i64>>(4)?
                .map(|height| i64_to_u32("core_utxos.winner_mined_height", height))
                .transpose()?,
        });
    }
    Ok(outpoints)
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

/// Apply storage-independent effects without selecting claimants in SQL.
pub(crate) fn apply_effects(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    effects: &HistoryReplayEffects,
) -> Result<(), WalletStorageError> {
    for txid in &effects.removed_records {
        tx.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        )?;
    }
    for txid in &effects.removed_evidence {
        let key = params![wallet_id.as_slice(), txid.as_byte_array().as_slice()];
        tx.execute(
            "DELETE FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
            key,
        )?;
        // The encoded outpoint's transaction ID occupies bytes 2..=33.
        tx.execute(
            "DELETE FROM core_utxos WHERE wallet_id = ?1 AND substr(outpoint, 2, 32) = ?2",
            key,
        )?;
    }
    let removed = effects.removed_evidence.iter().copied().collect();
    asset_locks::remove_swept(tx, wallet_id, &removed)?;
    dashpay::settle_swept_payments(tx, wallet_id, &removed)?;
    // Row deletion can clear claimant links; exact owner effects restore them last.
    for (outpoint, effect) in &effects.claims {
        let key = blob::encode_outpoint(outpoint)?;
        match effect {
            ReplayClaimEffect::Remove => {
                tx.execute(
                    "DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                    params![wallet_id.as_slice(), key],
                )?;
            }
            ReplayClaimEffect::Release => {
                tx.execute("DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 1", params![wallet_id.as_slice(), key])?;
                tx.execute("UPDATE core_utxos SET spent = 0, spent_in_txid = NULL, winner_mined_height = NULL WHERE wallet_id = ?1 AND outpoint = ?2", params![wallet_id.as_slice(), key])?;
            }
            ReplayClaimEffect::Hold {
                claimant,
                mined_height,
            } => {
                tx.execute("INSERT INTO core_utxos (wallet_id, outpoint, value, script, is_sweep_placeholder, spent, spent_in_txid, winner_mined_height) VALUES (?1, ?2, 0, X'', 1, 1, ?3, ?4) ON CONFLICT(wallet_id, outpoint) DO UPDATE SET spent = 1, spent_in_txid = excluded.spent_in_txid, winner_mined_height = excluded.winner_mined_height",
                    params![wallet_id.as_slice(), key, claimant.map(|id| id.to_byte_array().to_vec()), mined_height.map(i64::from)])?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::migrations;
    use platform_wallet::manager::history_replay::{HistoryReplayEffects, ReplayClaimEffect};

    fn database() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        migrations::run(&mut connection).unwrap();
        for id in [[1u8; 32], [2u8; 32]] {
            connection.execute("INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)", params![id.as_slice()]).unwrap();
        }
        connection
    }

    fn point(marker: u8) -> OutPoint {
        OutPoint::new(Txid::from_byte_array([marker; 32]), 0)
    }

    fn seed_coin(connection: &Connection, wallet: &[u8; 32], point: OutPoint) {
        connection.execute("INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent, spent_in_txid, winner_mined_height) VALUES (?1, ?2, 100, X'', 1, ?3, 99)",
            params![wallet.as_slice(), blob::encode_outpoint(&point).unwrap(), point.txid.as_byte_array().as_slice()]).unwrap();
    }

    #[test]
    fn should_apply_exact_owner_claims_without_inheriting_a_removed_stamp() {
        let mut connection = database();
        let held = point(3);
        let released = point(4);
        let missing = point(5);
        for id in [[1u8; 32], [2u8; 32]] {
            seed_coin(&connection, &id, held);
            seed_coin(&connection, &id, released);
        }
        let effects = HistoryReplayEffects {
            claims: [
                (
                    held,
                    ReplayClaimEffect::Hold {
                        claimant: None,
                        mined_height: None,
                    },
                ),
                (released, ReplayClaimEffect::Release),
                (
                    missing,
                    ReplayClaimEffect::Hold {
                        claimant: Some(point(6).txid),
                        mined_height: Some(101),
                    },
                ),
            ]
            .into(),
            ..Default::default()
        };
        let tx = connection.transaction().unwrap();
        apply_effects(&tx, &[1; 32], &effects).unwrap();
        tx.commit().unwrap();
        let row = |wallet: [u8; 32],
                   point: OutPoint|
         -> (bool, Option<Vec<u8>>, Option<u32>, bool) {
            connection.query_row("SELECT spent, spent_in_txid, winner_mined_height, is_sweep_placeholder FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2", params![wallet.as_slice(), blob::encode_outpoint(&point).unwrap()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap()
        };
        assert_eq!(row([1; 32], held), (true, None, None, false));
        assert_eq!(row([1; 32], released), (false, None, None, false));
        assert_eq!(
            row([1; 32], missing),
            (
                true,
                Some(point(6).txid.to_byte_array().to_vec()),
                Some(101),
                true
            )
        );
        assert_eq!(
            row([2; 32], held),
            (
                true,
                Some(held.txid.to_byte_array().to_vec()),
                Some(99),
                false
            )
        );
    }

    #[test]
    fn should_roll_back_removals_when_a_later_claim_write_fails() {
        let mut connection = database();
        let record = point(7).txid;
        let held = point(8);
        seed_coin(&connection, &[1; 32], held);
        connection.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized) VALUES (?1, ?2, 1, 0)", params![[1u8; 32].as_slice(), record.as_byte_array().as_slice()]).unwrap();
        connection.execute_batch("CREATE TRIGGER fail_replay_claim BEFORE UPDATE ON core_utxos WHEN NOT EXISTS (SELECT 1 FROM core_transactions) BEGIN SELECT RAISE(ABORT, 'replay claim fault'); END;").unwrap();
        {
            let tx = connection.transaction().unwrap();
            let effects = HistoryReplayEffects {
                removed_records: [record].into(),
                claims: [(
                    held,
                    ReplayClaimEffect::Hold {
                        claimant: None,
                        mined_height: None,
                    },
                )]
                .into(),
                ..Default::default()
            };
            let error = apply_effects(&tx, &[1; 32], &effects).unwrap_err();
            assert!(
                matches!(&error, WalletStorageError::Sqlite(cause) if cause.to_string().contains("replay claim fault")),
                "{error:?}"
            );
        }
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                    params![[1u8; 32].as_slice(), record.as_byte_array().as_slice()],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
}
