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

use rusqlite::{params, Transaction};

use super::record_v019;
use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, id32};
use crate::sqlite::util::safe_cast::{i64_to_u32, u64_to_i64};

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
        "SELECT rowid,
                CASE WHEN typeof(wallet_id) = 'blob' THEN length(wallet_id) END,
                CASE WHEN typeof(wallet_id) = 'blob' AND length(wallet_id) = 32 THEN wallet_id END,
                CASE WHEN typeof(txid) = 'blob' THEN length(txid) END,
                CASE WHEN typeof(txid) = 'blob' AND length(txid) = 32 THEN txid END,
                CASE WHEN typeof(record_blob) = 'blob' THEN length(record_blob) END,
                CASE WHEN typeof(record_blob) = 'blob' AND length(record_blob) <= ?1 THEN record_blob END,
                typeof(wallet_id) = 'blob' AND typeof(txid) = 'blob' AND typeof(record_blob) = 'blob'
         FROM core_transactions WHERE height IS NULL AND record_blob IS NOT NULL",
    )?;
    let mut rows = stmt.query([u64_to_i64(
        "record_blob.limit",
        blob::BLOB_SIZE_LIMIT_BYTES as u64,
    )?])?;
    while let Some(row) = rows.next()? {
        let record = (|| {
            if !row.get::<_, bool>(7)? {
                return Err(WalletStorageError::blob_decode(
                    "kept transaction columns must be BLOBs",
                ));
            }
            blob::check_fixed_width(row.get(1)?, 32, "core_transactions.wallet_id")?;
            let wallet_id = id32("core_transactions.wallet_id", &row.get::<_, Vec<u8>>(2)?)?;
            blob::check_fixed_width(row.get(3)?, 32, "core_transactions.txid")?;
            let txid: Vec<u8> = row.get(4)?;
            blob::check_size(row.get(5)?)?;
            let record = record_v019::decode_transaction_record(&row.get::<_, Vec<u8>>(6)?)?;
            if record.txid.as_slice() != txid.as_slice() {
                return Err(WalletStorageError::blob_decode(
                    "kept transaction record is stored under another txid",
                ));
            }
            Ok((wallet_id, record))
        })();
        match (record, policy) {
            (Ok((wallet_id, record)), _) => {
                for outpoint in record.inputs {
                    tx.execute("INSERT OR IGNORE INTO core_transaction_inputs (wallet_id, txid, outpoint) VALUES (?1, ?2, ?3)",
                        params![wallet_id.as_slice(), record.txid.as_slice(), outpoint])?;
                }
            }
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
pub(super) fn rewind_sync_heights(
    tx: &Transaction<'_>,
    policy: LoadPolicy,
) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare("SELECT rowid, birth_height FROM wallets")?;
    let mut rows = stmt.query([])?;
    let mut rewinds = Vec::new();
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let stored_birth = row.get::<_, i64>(1).map_err(WalletStorageError::from);
        let (rewind, repaired) =
            match stored_birth.and_then(|birth| i64_to_u32("wallets.birth_height", birth)) {
                Ok(birth) => (birth.saturating_sub(1), false),
                Err(error) => match policy {
                    LoadPolicy::Strict => return Err(error),
                    LoadPolicy::Recovery => {
                        tracing::warn!(rowid, error_kind = error.error_kind_str(), %error,
                        "V020 recovered an invalid birth height with a full rescan");
                        (0, true)
                    }
                },
            };
        rewinds.push((rowid, rewind, repaired));
    }
    for (rowid, rewind, repaired) in rewinds {
        if repaired {
            tx.execute(
                "UPDATE wallets SET birth_height = 0 WHERE rowid = ?1",
                [rowid],
            )?;
        }
        tx.execute(
            "UPDATE core_sync_state SET synced_height = CASE WHEN ?3 THEN 0
                ELSE MIN(COALESCE(synced_height, ?2), ?2) END
             WHERE wallet_id = (SELECT wallet_id FROM wallets WHERE rowid = ?1)",
            params![rowid, rewind, repaired],
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

    /// Both contexts retained by V020, with literal txids and non-null input keys.
    #[test]
    fn should_index_immutable_v019_record_bytes() {
        for line in include_str!("../../../tests/fixtures/v019_kept_records.txt").lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            let (name, txid, outpoint, bytes) = (
                fields[0],
                hex::decode(fields[1]).unwrap(),
                hex::decode(fields[2]).unwrap(),
                hex::decode(fields[3]).unwrap(),
            );
            let wallet = [0xB7; 32];
            let mut conn = wallet_db(&wallet);
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) VALUES (?1, ?2, NULL, 0, ?3)", params![wallet.as_slice(), txid, bytes]).unwrap();
            rewind_v020(&conn);
            migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
            let indexed: (Vec<u8>, Vec<u8>) = conn
                .query_row(
                    "SELECT txid, outpoint FROM core_transaction_inputs",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(indexed, (txid, outpoint), "{name}");
        }
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
    fn should_skip_wrong_width_v019_identifiers_only_under_recovery() {
        for column in ["wallet_id", "txid"] {
            for width in [0, 31, 33] {
                let mut conn = wallet_db(&[0xBF; 32]);
                conn.execute("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, X'00')", params![[0xBFu8; 32].as_slice(), [0xC0u8; 32].as_slice()]).unwrap();
                conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
                conn.execute(
                    &format!("UPDATE core_transactions SET {column} = zeroblob(?1)"),
                    [width],
                )
                .unwrap();
                let tx = conn.transaction().unwrap();
                assert!(matches!(
                    index_kept_inputs(&tx, LoadPolicy::Strict),
                    Err(WalletStorageError::BlobDecode { .. })
                ));
                index_kept_inputs(&tx, LoadPolicy::Recovery).unwrap();
            }
        }
    }

    #[test]
    fn should_rewind_v020_at_birth_and_sync_boundaries() {
        let cases = [
            (0, None, 0),
            (0, Some(500), 0),
            (1, None, 0),
            (100, None, 99),
            (100, Some(500), 99),
            (100, Some(50), 50),
            (100, Some(99), 99),
            (100, Some(100), 99),
            (u32::MAX, None, u32::MAX - 1),
            (u32::MAX, Some(u32::MAX), u32::MAX - 1),
        ];
        for (birth, synced, expected) in cases {
            let wallet = [0xBA; 32];
            let mut conn = wallet_db(&wallet);
            conn.execute("UPDATE wallets SET birth_height = ?1", [birth])
                .unwrap();
            conn.execute(
                "INSERT INTO core_sync_state (wallet_id, synced_height) VALUES (?1, ?2)",
                params![wallet.as_slice(), synced],
            )
            .unwrap();
            rewind_v020(&conn);
            migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
            let actual: u32 = conn
                .query_row("SELECT synced_height FROM core_sync_state", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(actual, expected, "birth={birth}, synced={synced:?}");
        }
    }

    #[tracing_test::traced_test]
    #[test]
    fn should_recover_invalid_v019_birth_heights_with_a_logged_full_rescan() {
        for birth in [-1, i64::MIN, i64::from(u32::MAX) + 1, i64::MAX] {
            let wallet = [0xBB; 32];
            let mut conn = wallet_db(&wallet);
            conn.execute("UPDATE wallets SET birth_height = ?1", [birth])
                .unwrap();
            conn.execute(
                "INSERT INTO core_sync_state (wallet_id, synced_height) VALUES (?1, 500)",
                [wallet.as_slice()],
            )
            .unwrap();
            migrations::insert_test_wallet(&conn, &[0xBC; 32], 100);
            conn.execute(
                "INSERT INTO core_sync_state (wallet_id, synced_height) VALUES (?1, 500)",
                [[0xBCu8; 32].as_slice()],
            )
            .unwrap();
            rewind_v020(&conn);
            migrations::run_for_open(&mut conn, LoadPolicy::Recovery)
                .expect("Recovery must open the store");
            let repaired: (i64, i64) = conn.query_row("SELECT birth_height, synced_height FROM wallets JOIN core_sync_state USING(wallet_id) WHERE wallet_id = ?1", [wallet.as_slice()], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!(repaired, (0, 0));
            let healthy: i64 = conn
                .query_row(
                    "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
                    [[0xBCu8; 32].as_slice()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(healthy, 99);
            assert!(logs_contain(
                "V020 recovered an invalid birth height with a full rescan"
            ));
        }
    }

    #[test]
    fn should_reject_text_v019_values_before_materializing_them() {
        for column in ["wallet_id", "txid", "record_blob"] {
            let mut conn = wallet_db(&[0xBD; 32]);
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, X'00')", params![[0xBDu8; 32].as_slice(), [0xBEu8; 32].as_slice()]).unwrap();
            conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
            // Under the character cap, over SQLite's byte cap.
            let text = "🦀".repeat(SQLITE_MAX_BLOB_BYTES as usize / 4 + 1);
            conn.execute(
                &format!("UPDATE core_transactions SET {column} = ?1"),
                [text],
            )
            .unwrap();
            conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, SQLITE_MAX_BLOB_BYTES)
                .unwrap();
            let tx = conn.transaction().unwrap();
            let error = index_kept_inputs(&tx, LoadPolicy::Strict).unwrap_err();
            assert!(
                matches!(error, WalletStorageError::BlobDecode { .. }),
                "{column}: {error:?}"
            );
            index_kept_inputs(&tx, LoadPolicy::Recovery).unwrap();
        }
    }

    #[test]
    fn should_reject_invalid_birth_heights_before_v020_cleanup() {
        for birth in [-1, i64::MIN, i64::from(u32::MAX) + 1, i64::MAX] {
            let policy = LoadPolicy::Strict;
            let wallet_id = [0xB5; 32];
            let mut conn = wallet_db(&wallet_id);
            conn.execute("UPDATE wallets SET birth_height = ?1", [birth])
                .unwrap();
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized) VALUES (?1, ?2, 100, 0)", params![wallet_id.as_slice(), [0xB6u8; 32].as_slice()]).unwrap();
            rewind_v020(&conn);
            conn.execute_batch("CREATE TRIGGER forbid_cleanup BEFORE DELETE ON core_transactions BEGIN SELECT RAISE(ABORT, 'cleanup reached'); END;").unwrap();
            let error = migrations::run_for_open(&mut conn, policy).unwrap_err();
            let WalletStorageError::Migration(error) = error else {
                panic!("expected migration error")
            };
            let refinery_core::error::Kind::Connection(_, source) = error.kind() else {
                panic!("expected typed migration failure")
            };
            assert!(matches!(
                source.downcast_ref::<WalletStorageError>(),
                Some(WalletStorageError::IntegerOverflow {
                    field: "wallets.birth_height",
                    ..
                })
            ));

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
