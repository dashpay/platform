#![allow(clippy::field_reassign_with_default)]

//! TC-045..TC-049 — native foreign-key enforcement and the delete cascade.

mod common;

use common::{ensure_identity, ensure_wallet_meta, fresh_persister, wid};

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

/// insert into a child table without a wallets parent fails.
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

/// deleting wallets cascades.
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
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, account_index, spent, spent_in_txid) \
         VALUES (?1, ?2, 100, X'00', 0, 1, ?3)",
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

/// TC-049: `identity_keys` rows carry TWO `ON DELETE CASCADE` parents
/// (`wallet_id -> wallets`, `identity_id -> identities`).
/// Deleting the wallet must purge the child via that dual-cascade — both
/// paths firing on one row is idempotent, not a double-free error.
#[test]
fn tc049_delete_wallet_cascades_identity_keys() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC4);
    let identity = [0xE4u8; 32];
    // Seed BOTH FK parents: the wallets row and a wallet-scoped
    // identities row, so the child satisfies both cascade chains.
    ensure_wallet_meta(&persister, &w);
    ensure_identity(&persister, &identity, Some(&w));
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, 0, X'01', ?3, NULL)",
            rusqlite::params![w.as_slice(), &identity[..], &[0u8; 20][..]],
        )
        .unwrap();
    }

    let before: i64 = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT COUNT(*) FROM identity_keys WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(before, 1, "seed row must exist before delete");

    let report = persister.delete_wallet(w).expect("delete_wallet");
    assert_eq!(report.wallet_id, w);

    let after: i64 = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT COUNT(*) FROM identity_keys WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after, 0, "dual cascade must purge the identity_keys row");
}
