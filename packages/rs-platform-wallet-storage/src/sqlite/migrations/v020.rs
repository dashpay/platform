//! Typed half of V020: index every input of each kept unmined record.
//!
//! SQL cannot decode a record body, so the SQL half can only name the coins a
//! kept record already holds the claim on. A sweep that withholds an input
//! hands it to a surviving spender found through this index; a survivor
//! missing from it loses the claim to the winner, and the coin stays held
//! after the survivor is defeated. Every input is therefore indexed here.
//!
//! V019 records use the historical adapter in `record_v019`; runtime codec
//! changes must not change the input set this one-shot migration indexes.

use dashcore::hashes::Hash;
use rusqlite::{params, Transaction};

use super::record_v019;
use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, core_history, id32};
use crate::sqlite::util::safe_cast::i64_to_u32;

/// Index the complete input set of every kept record body.
///
/// A body this cannot read (oversize, undecodable, or stored under another
/// txid) follows `policy`: [`LoadPolicy::Strict`] fails the upgrade, which
/// rolls back to V019, so a codec regression surfaces at once instead of as
/// coins stuck later. [`LoadPolicy::Recovery`] logs and skips it; its wallet
/// then degrades at load, where the same body fails to decode again.
pub(super) fn index_kept_inputs(
    tx: &Transaction<'_>,
    policy: LoadPolicy,
) -> Result<(), WalletStorageError> {
    // CASE is lazy in SQLite: rejected columns never enter the result row.
    let mut stmt = tx.prepare(
        "SELECT rowid, length(wallet_id), CASE WHEN length(wallet_id) = 32 THEN wallet_id END,
                length(txid), CASE WHEN length(txid) = 32 THEN txid END,
                length(record_blob), CASE WHEN length(record_blob) <= ?1 THEN record_blob END
         FROM core_transactions WHERE height IS NULL AND record_blob IS NOT NULL",
    )?;
    let mut rows = stmt.query([blob::BLOB_SIZE_LIMIT_BYTES as i64])?;
    while let Some(row) = rows.next()? {
        let record = (|| {
            blob::check_fixed_width(row.get(1)?, 32, "core_transactions.wallet_id")?;
            let wallet_id = id32("core_transactions.wallet_id", &row.get::<_, Vec<u8>>(2)?)?;
            blob::check_fixed_width(row.get(3)?, 32, "core_transactions.txid")?;
            let txid: Vec<u8> = row.get(4)?;
            blob::check_size(row.get(5)?)?;
            let record = record_v019::decode_transaction_record(&row.get::<_, Vec<u8>>(6)?)?;
            if record.txid.as_byte_array().as_slice() != txid.as_slice() {
                return Err(WalletStorageError::blob_decode(
                    "kept transaction record is stored under another txid",
                ));
            }
            Ok((wallet_id, record))
        })();
        match (record, policy) {
            (Ok((wallet_id, record)), _) => core_history::index_record(tx, &wallet_id, &record)?,
            (Err(error), LoadPolicy::Strict) => return Err(error),
            (Err(error), LoadPolicy::Recovery) => tracing::warn!(
                rowid = row.get::<_, i64>(0)?,
                %error,
                "V020 skipped a kept transaction record it cannot read; its inputs stay unindexed"
            ),
        }
    }
    Ok(())
}

/// Validate every birth height before V020 can discard mined history.
pub(super) fn rewind_sync_heights(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare("SELECT rowid, birth_height FROM wallets")?;
    let mut rows = stmt.query([])?;
    let mut rewinds = Vec::new();
    while let Some(row) = rows.next()? {
        let birth = i64_to_u32("wallets.birth_height", row.get(1)?)?;
        rewinds.push((row.get::<_, i64>(0)?, birth.saturating_sub(1)));
    }
    for (rowid, rewind) in rewinds {
        tx.execute(
            "UPDATE core_sync_state SET synced_height = MIN(COALESCE(synced_height, ?2), ?2)
             WHERE wallet_id = (SELECT wallet_id FROM wallets WHERE rowid = ?1)",
            params![rowid, rewind],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::{limits::Limit, params};

    use super::*;
    use crate::sqlite::conn::SQLITE_MAX_BLOB_BYTES;
    use crate::sqlite::migrations::{self, rewind_v020, wallet_db};

    /// Bytes written before V020, with fixed expected txid and input independent
    /// of the current TransactionRecord serializer.
    #[test]
    fn should_index_immutable_v019_record_bytes() {
        let wallet_id = [0xB7; 32];
        let mut conn = wallet_db(&wallet_id);
        let fixture = include_str!("../../../tests/fixtures/core_transaction_rows.txt")
            .lines()
            .find(|line| line.starts_with("classical_type_bytes_42 "))
            .unwrap();
        let bytes = hex::decode(fixture.split_once(' ').unwrap().1).unwrap();
        let txid = hex::decode("592c22450e6aae21e658bd31b4f68fa942eb9336ef617fe24b56fe391c9a0b66")
            .unwrap();
        conn.execute(
            "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)",
            params![wallet_id.as_slice(), txid, bytes],
        ).unwrap();
        rewind_v020(&conn);
        migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
        let indexed: (Vec<u8>, Vec<u8>) = conn
            .query_row(
                "SELECT txid, outpoint FROM core_transaction_inputs",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(indexed.0, txid);
        assert_eq!(
            hex::encode(indexed.1),
            "20000000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn should_skip_oversized_v019_bodies_before_sqlite_materializes_them() {
        for size in [
            blob::BLOB_SIZE_LIMIT_BYTES + 1,
            SQLITE_MAX_BLOB_BYTES as usize + 1,
        ] {
            let wallet_id = [0xB1; 32];
            let mut conn = wallet_db(&wallet_id);
            conn.execute(
                "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, zeroblob(?3))",
                params![wallet_id.as_slice(), [0xB2u8; 32].as_slice(), size as i64],
            ).unwrap();
            conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, SQLITE_MAX_BLOB_BYTES)
                .unwrap();
            {
                let tx = conn.transaction().unwrap();
                let error = index_kept_inputs(&tx, LoadPolicy::Strict).unwrap_err();
                assert!(
                    matches!(error, WalletStorageError::BlobTooLarge { .. }),
                    "{error:?}"
                );
                index_kept_inputs(&tx, LoadPolicy::Recovery)
                    .expect("Recovery skips oversized bodies");
            }
            rewind_v020(&conn);
            migrations::run_for_open(&mut conn, LoadPolicy::Recovery)
                .expect("Recovery completes V020");
            let count: i64 = conn
                .query_row("SELECT count(*) FROM core_transaction_inputs", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn should_classify_oversized_v019_identifiers_before_materializing_them() {
        for column in ["wallet_id", "txid"] {
            let wallet_id = [0xB3; 32];
            let mut conn = wallet_db(&wallet_id);
            conn.execute(
                "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, X'00')",
                params![wallet_id.as_slice(), [0xB4u8; 32].as_slice()],
            ).unwrap();
            conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
            conn.execute(
                &format!("UPDATE core_transactions SET {column} = zeroblob(?1)"),
                [SQLITE_MAX_BLOB_BYTES + 1],
            )
            .unwrap();
            conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, SQLITE_MAX_BLOB_BYTES)
                .unwrap();
            let tx = conn.transaction().unwrap();
            let error = index_kept_inputs(&tx, LoadPolicy::Strict).unwrap_err();
            assert!(
                matches!(error, WalletStorageError::BlobTooLarge { .. }),
                "{column}: {error:?}"
            );
            index_kept_inputs(&tx, LoadPolicy::Recovery)
                .expect("Recovery skips invalid identifier rows");
        }
    }

    #[test]
    fn should_reject_invalid_birth_heights_before_v020_cleanup() {
        for birth in [-1, i64::MIN, i64::from(u32::MAX) + 1, i64::MAX] {
            for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
                let wallet_id = [0xB5; 32];
                let mut conn = wallet_db(&wallet_id);
                conn.execute("UPDATE wallets SET birth_height = ?1", [birth])
                    .unwrap();
                conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized) VALUES (?1, ?2, 100, 0)", params![wallet_id.as_slice(), [0xB6u8; 32].as_slice()]).unwrap();
                rewind_v020(&conn);
                conn.execute_batch("CREATE TRIGGER forbid_cleanup BEFORE DELETE ON core_transactions BEGIN SELECT RAISE(ABORT, 'cleanup reached'); END;").unwrap();
                let error = migrations::run_for_open(&mut conn, policy).unwrap_err();
                assert!(
                    format!("{error:?}").contains("wallets.birth_height"),
                    "birth height must fail before cleanup: {error:?}"
                );
                let version: i64 = conn
                    .query_row(
                        "SELECT max(version) FROM refinery_schema_history",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(version, 19);
                let count: i64 = conn
                    .query_row("SELECT count(*) FROM core_transactions", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(count, 1);
            }
        }
    }
}
