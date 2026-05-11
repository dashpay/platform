#![allow(clippy::field_reassign_with_default)]

//! TC-016..TC-024 (subset) — buffer + flush semantics.
//!
//! Some adversarial cases (TC-021 partial-failure, TC-024 mid-flush
//! failure) require a fault-injection seam that the production code
//! exposes only behind `#[cfg(test)]`. The seam is documented in
//! `persister.rs::lock_conn_for_test`; tests that need to inject a
//! failure poison the DB through that handle and verify rollback.

mod common;

use std::collections::BTreeMap;

use common::{ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, ro_conn, wid};

use dashcore::hashes::Hash;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_sqlite::FlushMode;

fn core_with_height(synced_height: u32, last_processed_height: u32) -> CoreChangeSet {
    CoreChangeSet {
        synced_height: Some(synced_height),
        last_processed_height: Some(last_processed_height),
        ..Default::default()
    }
}

fn changeset(core: CoreChangeSet) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(core),
        ..Default::default()
    }
}

/// TC-017: Manual mode defers I/O.
#[test]
fn tc017_manual_defers_io() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(1);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(5, 5)))
        .unwrap();
    // Without a flush, the row count for core_sync_state for `w` is 0.
    let n: i64 = ro_conn(&path)
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
    persister.flush(w).unwrap();
    let n: i64 = ro_conn(&path)
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

/// TC-018: Immediate mode flushes inline.
#[test]
fn tc018_immediate_flushes_inline() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Immediate);
    let w = wid(2);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(5, 5)))
        .unwrap();
    let n: i64 = ro_conn(&path)
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

/// TC-019: commit_writes flushes every dirty wallet.
#[test]
fn tc019_commit_writes_flushes_dirty() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0x10);
    let b = wid(0x20);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    persister
        .store(a, changeset(core_with_height(5, 5)))
        .unwrap();
    persister
        .store(b, changeset(core_with_height(7, 7)))
        .unwrap();
    persister.commit_writes().unwrap();
    let conn = ro_conn(&path);
    let count_for = |id: &[u8; 32]| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![id.as_slice()],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(count_for(&a), 1);
    assert_eq!(count_for(&b), 1);
}

/// TC-020: commit_writes in Immediate mode is a no-op.
#[test]
fn tc020_commit_writes_noop_in_immediate() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Immediate);
    persister.commit_writes().unwrap();
}

/// TC-022: flush(A) doesn't write or clear B's buffer.
#[test]
fn tc022_flush_is_scoped() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0x30);
    let b = wid(0x31);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    persister
        .store(a, changeset(core_with_height(3, 3)))
        .unwrap();
    persister
        .store(b, changeset(core_with_height(4, 4)))
        .unwrap();
    persister.flush(a).unwrap();
    let conn = ro_conn(&path);
    let count_for = |id: &[u8; 32]| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![id.as_slice()],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(count_for(&a), 1);
    assert_eq!(count_for(&b), 0);
    persister.flush(b).unwrap();
    assert_eq!(count_for(&b), 1);
}

/// TC-016: property — N stores then flush == one merged store.
///
/// We use the monotonic-max merge on sync heights as the oracle.
#[test]
fn tc016_buffer_merge_oracle_smoke() {
    use proptest::prelude::*;
    let strategy = proptest::collection::vec((0u32..1_000_000, 0u32..1_000_000), 1..6);
    proptest!(ProptestConfig::with_cases(64), |(heights in strategy)| {
        let (persister, _tmp, _path) = fresh_persister();
        let w = wid(0x40);
        ensure_wallet_meta(&persister, &w);
        for &(sp, lp) in &heights {
            persister.store(w, changeset(core_with_height(sp, lp))).unwrap();
        }
        // Read back the persisted heights.
        let conn = persister.lock_conn_for_test();
        let (synced, lp): (Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT synced_height, last_processed_height FROM core_sync_state WHERE wallet_id = ?1",
                rusqlite::params![w.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        drop(conn);
        let expected_synced = heights.iter().map(|(s, _)| *s).max().unwrap_or(0);
        let expected_lp = heights.iter().map(|(_, l)| *l).max().unwrap_or(0);
        prop_assert_eq!(synced.unwrap_or(0) as u32, expected_synced);
        prop_assert_eq!(lp.unwrap_or(0) as u32, expected_lp);
    });
}

/// TC-001 (subset) — get_core_tx_record round-trips through `core_transactions`.
#[test]
fn tc001_get_core_tx_record_roundtrip() {
    use dashcore::blockdata::transaction::Transaction;
    use dashcore::Txid;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0x50);
    ensure_wallet_meta(&persister, &w);
    let txid = Txid::from_byte_array([9u8; 32]);
    let dummy_tx = Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    };
    let mut record = TransactionRecord::new(
        dummy_tx,
        key_wallet::account::AccountType::Standard {
            index: 0,
            standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
        },
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            42,
            dashcore::BlockHash::from_byte_array([3u8; 32]),
            1735689600,
        )),
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        Vec::new(),
        100,
    );
    record.txid = txid;
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        records: vec![record],
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    let got = persister.get_core_tx_record(w, &txid).unwrap();
    let got = got.expect("record present");
    assert_eq!(got.txid, txid);
    let info = got.context.block_info().expect("block info present");
    assert_eq!(info.height(), 42);
    let unknown = dashcore::Txid::from_byte_array([0u8; 32]);
    assert!(persister.get_core_tx_record(w, &unknown).unwrap().is_none());
}

/// TC-015: two wallets coexist without key collisions.
#[test]
fn tc015_two_wallets_in_one_db() {
    let (persister, _tmp, _path) = fresh_persister();
    let a = wid(0xA1);
    let b = wid(0xB2);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    // Distinct height per wallet so we can distinguish.
    persister
        .store(a, changeset(core_with_height(11, 11)))
        .unwrap();
    persister
        .store(b, changeset(core_with_height(22, 22)))
        .unwrap();
    let conn = persister.lock_conn_for_test();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM core_sync_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
    let h_a: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![a.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    let h_b: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![b.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(h_a, 11);
    assert_eq!(h_b, 22);
}

// Mark the unused `BTreeMap` import as used in case future expansion of
// this test file needs it.
#[allow(dead_code)]
fn _unused_btreemap() -> BTreeMap<u32, u32> {
    BTreeMap::new()
}
