#![allow(clippy::field_reassign_with_default)]

//! Strict-by-default load vs. opt-in recovery, site by site.
//!
//! Each tolerable-today inconsistency gets a pair: it aborts the load under
//! [`LoadPolicy::Strict`] with a named error, and under
//! [`LoadPolicy::Recovery`] it is logged, counted on
//! [`SqlitePersister::last_load_degradation`], and the documented degraded
//! projection is served instead.

mod common;

use common::{ensure_wallet_meta, fresh_persister, fresh_recovery_persister, wid};
use dashcore::hashes::Hash;
use dashcore::Txid;
use platform_wallet::changeset::{
    CoreChangeSet, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{LoadSite, SqlitePersister, WalletStorageError};
use rusqlite::params;

/// Downcast a trait-boundary error to the storage error it wraps.
#[track_caller]
fn typed(err: PersistenceError) -> WalletStorageError {
    let PersistenceError::Backend { source, .. } = err else {
        panic!("expected a typed backend error, got {err:?}");
    };
    *source
        .downcast::<WalletStorageError>()
        .expect("backend source must be a WalletStorageError")
}

/// Assert exactly `expected` tolerated events were counted at `site`, and
/// that no other site fired.
#[track_caller]
fn assert_only_site(persister: &SqlitePersister, site: LoadSite, expected: u32) {
    let degradation = persister.last_load_degradation();
    assert!(degradation.degraded, "load must report itself degraded");
    assert_eq!(
        degradation.by_site.get(&site).copied(),
        Some(expected),
        "per-site count for {site}: {:?}",
        degradation.by_site
    );
    assert_eq!(
        degradation.by_site.len(),
        1,
        "no other site may fire: {:?}",
        degradation.by_site
    );
    assert_eq!(degradation.total, expected);
}

/// Seed a `core_sync_state` row, then overwrite its chain lock with bytes
/// that are not a `ChainLock`.
fn seed_corrupt_chain_lock(persister: &SqlitePersister, wallet: &WalletId) {
    ensure_wallet_meta(persister, wallet);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(11),
        last_processed_height: Some(11),
        ..Default::default()
    });
    persister.store(*wallet, cs).expect("seed core sync state");
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "UPDATE core_sync_state SET last_applied_chain_lock = ?1 WHERE wallet_id = ?2",
        params![&[0xFFu8; 5][..], wallet.as_slice()],
    )
    .expect("plant corrupt chain lock");
}

/// Seed one blob-bearing `core_transactions` row, then drift its typed
/// `height` column away from the height inside the blob.
fn seed_drifted_transaction(persister: &SqlitePersister, wallet: &WalletId) {
    ensure_wallet_meta(persister, wallet);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        records: vec![confirmed_record()],
        ..Default::default()
    });
    persister.store(*wallet, cs).expect("seed transaction");
    let conn = persister.lock_conn_for_test();
    let updated = conn
        .execute(
            "UPDATE core_transactions SET height = 999 WHERE wallet_id = ?1",
            params![wallet.as_slice()],
        )
        .expect("drift typed height");
    assert_eq!(updated, 1, "seed must have written exactly one row");
}

fn drifted_txid() -> Txid {
    Txid::from_byte_array([0x7Au8; 32])
}

/// A record whose blob says height 300, so a drifted typed column is
/// unambiguous.
fn confirmed_record() -> key_wallet::managed_account::transaction_record::TransactionRecord {
    use dashcore::{BlockHash, Transaction};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    let mut record = TransactionRecord::new(
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            300,
            BlockHash::from_byte_array([0x21u8; 32]),
            1_735_689_600,
        )),
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        Vec::new(),
        100,
    );
    record.txid = drifted_txid();
    record
}

// ── (a) chain-lock blob ─────────────────────────────────────────────────

#[test]
fn corrupt_chain_lock_blob_is_fatal_under_strict() {
    let wallet = wid(0x20);
    let (persister, _tmp, _path) = fresh_persister();
    seed_corrupt_chain_lock(&persister, &wallet);

    let err = typed(
        persister
            .load()
            .expect_err("a corrupt chain lock must abort a strict load"),
    );
    assert!(
        matches!(err, WalletStorageError::BincodeDecode { .. }),
        "expected the upstream decode error to survive, got {err:?}"
    );
    assert!(
        !persister.is_degraded(),
        "a failed load must not report a partial tally"
    );
}

#[test]
fn corrupt_chain_lock_blob_is_tolerated_in_recovery() {
    let wallet = wid(0x21);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_corrupt_chain_lock(strict, &wallet));

    let state = persister.load().expect("recovery must complete the load");
    let loaded = state.wallets.get(&wallet).expect("wallet must rehydrate");
    assert!(
        loaded
            .wallet_info
            .metadata
            .last_applied_chain_lock
            .is_none(),
        "the undecodable chain lock must be dropped, not guessed at"
    );
    assert_eq!(
        loaded.wallet_info.metadata.synced_height, 11,
        "the rest of the sync state must survive"
    );
    assert_only_site(&persister, LoadSite::ChainLockBlob, 1);
}

// ── (c) core-transaction typed-column drift ─────────────────────────────

#[test]
fn core_transaction_column_drift_is_fatal_under_strict() {
    let wallet = wid(0x22);
    let (persister, _tmp, _path) = fresh_persister();
    seed_drifted_transaction(&persister, &wallet);

    let err = typed(
        persister
            .load()
            .expect_err("typed columns disagreeing with the blob must abort a strict load"),
    );
    assert!(
        matches!(
            err,
            WalletStorageError::CoreTransactionEntryMismatch {
                typed_height: Some(999),
                blob_height: Some(300),
                ..
            }
        ),
        "expected CoreTransactionEntryMismatch, got {err:?}"
    );
}

#[test]
fn core_transaction_column_drift_is_tolerated_in_recovery() {
    let wallet = wid(0x23);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_drifted_transaction(strict, &wallet));

    persister.load().expect("recovery must complete the load");
    assert_only_site(&persister, LoadSite::CoreTransactionColumnDrift, 1);
}

#[test]
fn get_core_tx_record_never_writes() {
    // The read path used to repair drifted typed columns in place. A `&self`
    // read on the persistence trait must not mutate the database at all —
    // this pins the row bytes across a drift read.
    let wallet = wid(0x24);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_drifted_transaction(strict, &wallet));

    let before = transaction_row(&persister, &wallet);
    let record = persister
        .get_core_tx_record(wallet, &drifted_txid())
        .expect("recovery must still serve the point read")
        .expect("blob-bearing row must return its record");
    assert_eq!(
        record.height(),
        Some(300),
        "the blob stays authoritative for the returned record"
    );
    assert_eq!(
        transaction_row(&persister, &wallet),
        before,
        "a read must leave the row byte-identical"
    );
}

#[test]
fn get_core_tx_record_drift_is_fatal_under_strict() {
    let wallet = wid(0x25);
    let (persister, _tmp, _path) = fresh_persister();
    seed_drifted_transaction(&persister, &wallet);

    let err = typed(
        persister
            .get_core_tx_record(wallet, &drifted_txid())
            .expect_err("a drifted row must not be served silently under strict"),
    );
    assert!(
        matches!(err, WalletStorageError::CoreTransactionEntryMismatch { .. }),
        "expected CoreTransactionEntryMismatch, got {err:?}"
    );
}

/// `(txid, height, record_blob)` of the wallet's single transaction row.
fn transaction_row(
    persister: &SqlitePersister,
    wallet: &WalletId,
) -> (Vec<u8>, Option<i64>, Vec<u8>) {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT txid, height, record_blob FROM core_transactions WHERE wallet_id = ?1",
        params![wallet.as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("transaction row must exist")
}

// ── (b) shielded viewing-key row ────────────────────────────────────────

/// One valid viewing key plus one whose blob is a byte short of the fixed
/// 96-byte width.
#[cfg(feature = "shielded")]
fn seed_corrupt_viewing_key(
    persister: &SqlitePersister,
    valid_wallet: &WalletId,
    corrupt_wallet: &WalletId,
) {
    ensure_wallet_meta(persister, valid_wallet);
    ensure_wallet_meta(persister, corrupt_wallet);
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
         VALUES (?1, ?2, ?3)",
        params![valid_wallet.as_slice(), 1_i64, &[0xE5_u8; 96]],
    )
    .expect("insert valid viewing key");
    conn.execute(
        "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
         VALUES (?1, ?2, ?3)",
        params![corrupt_wallet.as_slice(), 2_i64, &[0xF6_u8; 95]],
    )
    .expect("insert corrupt viewing key");
}

#[cfg(feature = "shielded")]
#[test]
fn corrupt_shielded_viewing_key_row_is_fatal_under_strict() {
    let valid_wallet = wid(0x28);
    let corrupt_wallet = wid(0x29);
    let (persister, _tmp, _path) = fresh_persister();
    seed_corrupt_viewing_key(&persister, &valid_wallet, &corrupt_wallet);

    let err = typed(
        persister
            .load()
            .expect_err("a corrupt viewing-key row must abort a strict load"),
    );
    assert!(
        matches!(err, WalletStorageError::BlobDecode { .. }),
        "expected BlobDecode for the short viewing key, got {err:?}"
    );
}

/// Relocated from `sqlite_shielded_viewing_keys.rs`: skipping one corrupt
/// row is now recovery-only behaviour, not the default.
#[cfg(feature = "shielded")]
#[test]
fn corrupt_shielded_viewing_key_row_is_skipped_in_recovery() {
    use platform_wallet::wallet::shielded::SubwalletId;
    let valid_wallet = wid(0x2A);
    let corrupt_wallet = wid(0x2B);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_corrupt_viewing_key(strict, &valid_wallet, &corrupt_wallet)
    });

    let state = persister
        .load()
        .expect("one corrupt viewing key must not fail a recovery load");
    assert!(state.wallets.contains_key(&valid_wallet));
    assert!(state.wallets.contains_key(&corrupt_wallet));
    assert_eq!(
        state
            .shielded
            .viewing_keys
            .get(&SubwalletId::new(valid_wallet, 1)),
        Some(&vec![0xE5; 96])
    );
    assert!(!state
        .shielded
        .viewing_keys
        .contains_key(&SubwalletId::new(corrupt_wallet, 2)));
    assert_only_site(&persister, LoadSite::ShieldedViewingKeyRow, 1);
}

// ── flag semantics ──────────────────────────────────────────────────────

#[test]
fn degraded_flag_is_false_on_a_clean_load() {
    let wallet = wid(0x26);
    let (persister, _tmp, _path) = fresh_persister();
    ensure_wallet_meta(&persister, &wallet);

    persister.load().expect("clean load");
    assert!(!persister.is_degraded());
    assert_eq!(persister.last_load_degradation().total, 0);
    assert!(persister.last_load_degradation().by_site.is_empty());
}

#[test]
fn degraded_counts_are_per_load_not_cumulative() {
    let wallet = wid(0x27);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_corrupt_chain_lock(strict, &wallet));

    persister.load().expect("first recovery load");
    assert_only_site(&persister, LoadSite::ChainLockBlob, 1);
    persister.load().expect("second recovery load");
    assert_only_site(
        &persister,
        LoadSite::ChainLockBlob,
        1, // replaced, not summed — otherwise a repaired DB could never read clean
    );
}
