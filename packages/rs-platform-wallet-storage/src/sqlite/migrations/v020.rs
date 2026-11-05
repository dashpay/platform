//! Backfill all historical input indexes and repair accounting without changing spend state.

use dashcore::hashes::Hash;
use rusqlite::Transaction;

use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, core_history, core_state, id32};
use crate::sqlite::util::safe_cast::u64_to_i64;
use platform_wallet::changeset::CoreChangeSet;

/// Index and repair every decodable historical record in the migration transaction.
pub(super) fn backfill_accounting(
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
         FROM core_transactions WHERE rowid >= ?2 AND record_blob IS NOT NULL ORDER BY rowid LIMIT 1",
    )?;
    let limit = u64_to_i64("record_blob.limit", blob::BLOB_SIZE_LIMIT_BYTES as u64)?;
    let mut next_rowid = i64::MIN;
    loop {
        // Close each cursor before repairing its row; no active SELECT observes writes.
        let (rowid, record) = {
            let mut rows = stmt.query((limit, next_rowid))?;
            let Some(row) = rows.next()? else { break };
            let rowid: i64 = row.get(0)?;
            let record = (|| {
                if !row.get::<_, bool>(7)? {
                    return Err(WalletStorageError::blob_decode(
                        "historical transaction columns must be BLOBs",
                    ));
                }
                blob::check_fixed_width(row.get(1)?, 32, "core_transactions.wallet_id")?;
                let wallet_id = id32("core_transactions.wallet_id", &row.get::<_, Vec<u8>>(2)?)?;
                blob::check_fixed_width(row.get(3)?, 32, "core_transactions.txid")?;
                let txid: Vec<u8> = row.get(4)?;
                blob::check_size(row.get(5)?)?;
                let record = core_state::decode_transaction_record(&row.get::<_, Vec<u8>>(6)?)?;
                if record.txid.as_byte_array().as_slice() != txid.as_slice() {
                    return Err(WalletStorageError::blob_decode(
                        "historical transaction record is stored under another txid",
                    ));
                }
                Ok((wallet_id, record))
            })();
            (rowid, record)
        };
        tx.execute_batch("SAVEPOINT accounting_record")?;
        let repaired = record.and_then(|(wallet_id, record)| {
            core_history::index_record(tx, &wallet_id, &record)?;
            core_history::apply(
                tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![record],
                    ..Default::default()
                },
            )
        });
        match (repaired, policy) {
            (Ok(()), _) => tx.execute_batch("RELEASE accounting_record")?,
            (Err(error), LoadPolicy::Strict) => return Err(error),
            (Err(error), LoadPolicy::Recovery) => {
                tx.execute_batch("ROLLBACK TO accounting_record; RELEASE accounting_record")?;
                tracing::warn!(rowid, %error,
                    "V020 skipped an unreadable historical transaction; its stored bytes are unchanged");
            }
        }
        let Some(next) = rowid.checked_add(1) else {
            break;
        };
        next_rowid = next;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{OutputRole, TransactionDirection};
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};
    use rusqlite::{limits::Limit, params};

    use super::*;
    use crate::sqlite::conn::SQLITE_MAX_BLOB_BYTES;
    use crate::sqlite::migrations::{self, rewind_v020, wallet_db};

    #[test]
    fn should_decode_all_immutable_v019_transaction_fixtures_with_live_codec() {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/core_transaction_rows.txt").lines() {
            let (name, bytes) = line.split_once(' ').unwrap();
            if name == "asset_lock_lifecycle" {
                continue;
            }
            let bytes = hex::decode(bytes).unwrap();
            let record = core_state::decode_transaction_record(&bytes)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(record.txid, record.transaction.txid(), "{name}");
            assert_eq!(blob::encode(&record).unwrap(), bytes, "{name}");
            assert!(
                matches!(record.context, TransactionContext::InChainLockedBlock(_)),
                "{name}"
            );
            assert_eq!(record.block_info().unwrap().height(), 200, "{name}");
            let wallet = [0xC1; 32];
            let mut conn = wallet_db(&wallet);
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) VALUES (?1, ?2, 200, 1, ?3)", params![wallet.as_slice(), record.txid.as_byte_array().as_slice(), bytes]).unwrap();
            rewind_v020(&conn);
            migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
            let remaining: (i64, i64) = conn.query_row("SELECT (SELECT count(*) FROM core_transactions), (SELECT count(*) FROM core_transaction_inputs)", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
            assert_eq!(remaining.0, 1, "{name}: mined history must remain");
            assert_eq!(remaining.1, record.transaction.input.len() as i64);
            count += 1;
        }
        assert_eq!(count, 18);
    }

    #[test]
    fn should_decode_immutable_unmined_contexts_and_leaf_values_with_live_codec() {
        for line in include_str!("../../../tests/fixtures/v019_kept_records.txt").lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            let bytes = hex::decode(fields[3]).unwrap();
            let record = core_state::decode_transaction_record(&bytes).unwrap();
            assert!(
                record.block_info().is_none(),
                "height NULL must match the context"
            );
            match (&record.context, fields[0]) {
                (TransactionContext::Mempool, "mempool") => {}
                (TransactionContext::InstantSend(lock), "instant_send") => {
                    assert_eq!(lock.txid, record.txid);
                    assert_eq!(
                        lock.inputs,
                        vec![record.transaction.input[0].previous_output]
                    );
                }
                _ => panic!("wrong fixture context"),
            }
            assert_eq!(record.transaction.input.len(), 1);
            assert_eq!(
                record.transaction.input[0]
                    .previous_output
                    .txid
                    .to_byte_array(),
                [0x42; 32]
            );
            assert_eq!(record.transaction.input[0].previous_output.vout, 7);
            assert_eq!(
                record.input_details[0].address.to_string(),
                "yMsgnH1xKGa85n4bq2imrZbG2KgrmGttAV"
            );
            assert_eq!(record.input_details[0].value, 1000);
            assert_eq!(record.transaction.input[0].script_sig.as_bytes(), &[0x51]);
            assert_eq!(
                record.transaction.input[0].witness.to_vec(),
                vec![vec![1, 2, 3]]
            );
            assert_eq!(record.transaction.output[0].value, 999);
            assert_eq!(record.output_details[0].role, OutputRole::Change);
            assert_eq!(
                record.output_details[0].address.as_ref(),
                Some(&record.input_details[0].address)
            );
            assert_eq!(
                record.account_type,
                AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account
                }
            );
            assert_eq!(record.transaction_type, TransactionType::Standard);
            assert_eq!(record.direction, TransactionDirection::Internal);
            assert_eq!(record.net_amount, -1);
            assert_eq!(record.fee, Some(1));
            assert_eq!(record.label, format!("V019 {} fixture", fields[0]));
            assert_eq!(blob::encode(&record).unwrap(), bytes);
            assert_eq!(hex::encode(record.txid.to_byte_array()), fields[1]);
            assert_eq!(
                hex::encode(
                    blob::encode_outpoint(&record.transaction.input[0].previous_output).unwrap()
                ),
                fields[2]
            );
        }
    }

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
                let error = backfill_accounting(&tx, LoadPolicy::Strict).unwrap_err();
                assert!(
                    matches!(error, WalletStorageError::BlobTooLarge { .. }),
                    "{error:?}"
                );
                backfill_accounting(&tx, LoadPolicy::Recovery)
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
            let error = backfill_accounting(&tx, LoadPolicy::Strict).unwrap_err();
            assert!(
                matches!(error, WalletStorageError::BlobTooLarge { .. }),
                "{column}: {error:?}"
            );
            backfill_accounting(&tx, LoadPolicy::Recovery)
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
                    backfill_accounting(&tx, LoadPolicy::Strict),
                    Err(WalletStorageError::BlobDecode { .. })
                ));
                backfill_accounting(&tx, LoadPolicy::Recovery).unwrap();
            }
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
            let error = backfill_accounting(&tx, LoadPolicy::Strict).unwrap_err();
            assert!(
                matches!(error, WalletStorageError::BlobDecode { .. }),
                "{column}: {error:?}"
            );
            backfill_accounting(&tx, LoadPolicy::Recovery).unwrap();
        }
    }
}
