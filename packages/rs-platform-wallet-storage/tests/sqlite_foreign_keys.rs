#![allow(clippy::field_reassign_with_default)]

//! Native foreign-key enforcement and the delete cascade.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

/// PRAGMA foreign_keys is ON on the connection.
#[test]
fn tc045_foreign_keys_on() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let fk: i64 = conn
        .query_row("SELECT * FROM pragma_foreign_keys", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fk, 1, "foreign_keys pragma not ON");
}

/// insert into a child table without a wallet_metadata parent fails.
#[test]
fn tc046_orphan_child_insert_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    use rusqlite::params;
    let res = conn.execute(
        "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
         VALUES (?1, NULL, NULL)",
        params![[99u8; 32].as_slice()],
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("FOREIGN KEY"),
        "expected FOREIGN KEY constraint failure, got `{err}`"
    );
}

/// deleting wallet_metadata cascades.
#[test]
fn tc047_delete_wallet_cascade() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC0);
    ensure_wallet_meta(&persister, &w);
    // Insert one row into a child table.
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
             VALUES (?1, 1, 1)",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
    }
    let report = persister.delete_wallet(w).expect("delete_wallet");
    assert_eq!(report.wallet_id, w);
    assert!(report.backup_path.is_some());
    let conn = persister.lock_conn_for_test();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
}

/// deleting a core_transactions row sets `spent_in_txid = NULL` on UTXOs.
#[test]
fn tc048_setnull_on_tx_delete() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);
    let conn = persister.lock_conn_for_test();
    let txid = [4u8; 32];
    let outpoint = vec![0u8; 36];
    conn.execute(
        "INSERT INTO core_transactions (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, 1, NULL, NULL, 0, X'01')",
        rusqlite::params![w.as_slice(), &txid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
         VALUES (?1, ?2, 100, X'00', NULL, 0, 1, ?3)",
        rusqlite::params![w.as_slice(), &outpoint, &txid[..]],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        rusqlite::params![w.as_slice(), &txid[..]],
    )
    .unwrap();

    // The UTXO row must SURVIVE the tx delete — the single-column trigger
    // clears `spent_in_txid` only. A future change that turns it into a
    // cascading DELETE must fail here, not pass silently.
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![w.as_slice(), &outpoint],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "UTXO row must survive the transaction delete");

    let (wallet_id, value, account_index, spent_in): (Vec<u8>, i64, i64, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT wallet_id, value, account_index, spent_in_txid \
             FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![w.as_slice(), &outpoint],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(wallet_id, w.as_slice(), "wallet_id must be preserved");
    assert_eq!(value, 100, "value must be preserved");
    assert_eq!(account_index, 0, "account_index must be preserved");
    assert!(
        spent_in.is_none(),
        "spent_in_txid should have been set to NULL"
    );
}
