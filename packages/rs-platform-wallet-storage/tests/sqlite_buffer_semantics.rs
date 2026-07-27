#![allow(clippy::field_reassign_with_default)]

//! Buffer + flush semantics.
//!
//! Some adversarial cases (partial-failure, mid-flush failure) require
//! a fault-injection seam that the production code
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
use platform_wallet_storage::FlushMode;

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

/// Manual mode defers I/O.
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

/// Immediate mode flushes inline.
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

/// commit_writes flushes every dirty wallet.
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
    let report = persister.commit_writes().unwrap();
    assert!(
        report.is_ok(),
        "commit_writes must succeed; report={report:?}"
    );
    assert_eq!(report.succeeded.len(), 2, "two wallets must flush");
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

/// commit_writes in Immediate mode is a no-op.
#[test]
fn tc020_commit_writes_noop_in_immediate() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Immediate);
    let report = persister.commit_writes().unwrap();
    assert!(report.succeeded.is_empty() && report.failed.is_empty());
}

/// flush(A) doesn't write or clear B's buffer.
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

/// property — N buffered stores then one flush == one merged
/// store.
///
/// Manual flush mode so the N stores accumulate in the buffer and the
/// single `flush` is what writes the merged delta; the monotonic-max
/// merge on sync heights is the oracle.
#[test]
fn tc016_buffer_merge_oracle_smoke() {
    use proptest::prelude::*;
    let strategy = proptest::collection::vec((0u32..1_000_000, 0u32..1_000_000), 1..6);
    proptest!(ProptestConfig::with_cases(64), |(heights in strategy)| {
        let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
        let w = wid(0x40);
        ensure_wallet_meta(&persister, &w);
        for &(sp, lp) in &heights {
            persister.store(w, changeset(core_with_height(sp, lp))).unwrap();
        }
        // One flush writes the merged delta accumulated in the buffer.
        persister.flush(w).unwrap();
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

///  (subset) — get_core_tx_record round-trips through `core_transactions`.
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

/// two wallets coexist without key collisions.
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

/// one `flush(wallet_id)` produces exactly one SQLite
/// transaction.
///
/// `rusqlite::Connection::commit_hook` registers a callback that fires
/// after every successful commit. We register it on the persister's
/// write connection, then drive a flush whose changeset touches
/// multiple sub-changesets (core sync state + wallet metadata +
/// platform addresses + token balances). The hook MUST fire exactly
/// once for the duration of the flush call, regardless of how many
/// tables were written.
#[test]
fn tc023_one_flush_is_one_transaction() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use dpp::prelude::Identifier;
    use key_wallet::Network;
    use platform_wallet::changeset::{TokenBalanceChangeSet, WalletMetadataEntry};

    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x90);
    ensure_wallet_meta(&persister, &w);
    // token_balances FK targets identities(identity_id); seed the
    // identity so the cross-area flush passes that constraint.
    let owner = Identifier::from([0xA1u8; 32]);
    common::ensure_identity(&persister, owner.as_bytes(), Some(&w));
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(core_with_height(7, 7));
    cs.wallet_metadata = Some(WalletMetadataEntry {
        network: Network::Testnet,
        wallet_group_id: w,
        birth_height: 1,
    });
    let mut balances = BTreeMap::new();
    let token = Identifier::from([0xA2u8; 32]);
    balances.insert((owner, token), 9u64);
    cs.token_balances = Some(TokenBalanceChangeSet {
        balances,
        ..Default::default()
    });
    persister.store(w, cs).unwrap();

    // Install the commit hook AFTER buffering (which only mutates
    // memory) and BEFORE flush.
    let commits = Arc::new(AtomicUsize::new(0));
    {
        let commits_clone = Arc::clone(&commits);
        let conn = persister.lock_conn_for_test();
        conn.commit_hook(Some(move || {
            commits_clone.fetch_add(1, Ordering::SeqCst);
            false
        }))
        .expect("install commit hook");
    }

    persister.flush(w).unwrap();

    // Remove the hook so the persister is reusable elsewhere.
    {
        let conn = persister.lock_conn_for_test();
        conn.commit_hook(None::<fn() -> bool>)
            .expect("remove commit hook");
    }

    assert_eq!(
        commits.load(Ordering::SeqCst),
        1,
        "expected exactly one COMMIT for the flush, got {}",
        commits.load(Ordering::SeqCst)
    );
}

// ---------------------------------------------------------------------------
// P2 — retry-safe flush
// ---------------------------------------------------------------------------

use platform_wallet::changeset::PersistenceError;
use platform_wallet_storage::WalletStorageError;
use rusqlite::ErrorCode;

fn make_busy_error() -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    ))
}

fn make_fatal_error() -> WalletStorageError {
    WalletStorageError::IntegrityCheckFailed {
        report: "simulated fatal".into(),
    }
}

fn install_commit_counter(
    persister: &platform_wallet_storage::SqlitePersister,
) -> std::sync::Arc<std::sync::atomic::AtomicUsize> {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let conn = persister.lock_conn_for_test();
    conn.commit_hook(Some(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        false
    }))
    .expect("install commit hook");
    counter
}

fn read_synced_height(path: &std::path::Path, w: &[u8; 32]) -> Option<i64> {
    use rusqlite::OptionalExtension;
    ro_conn(path)
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
}

/// happy-path flush is one transaction; second flush is a no-op.
#[test]
fn tc_p2_001_happy_path_one_tx_then_noop() {
    use std::sync::atomic::Ordering;
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(5, 5)))
        .unwrap();
    let commits = install_commit_counter(&persister);
    persister.flush(w).expect("first flush ok");
    persister.flush(w).expect("second flush ok (no-op)");
    assert_eq!(
        commits.load(Ordering::SeqCst),
        1,
        "expected exactly one COMMIT — buffer was empty on the second flush"
    );
    assert_eq!(read_synced_height(&path, &w), Some(5));
}

/// transient failure restores the buffer for retry.
#[test]
fn tc_p2_002_transient_failure_restores_buffer() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(7, 7)))
        .unwrap();
    persister.force_next_flush_to_fail(make_busy_error());
    let err = persister.flush(w).expect_err("first flush must fail");
    let msg = match err {
        PersistenceError::Backend { source, .. } => source.to_string(),
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    };
    assert!(
        msg.contains("flush failed transiently"),
        "expected FlushRetryable in message, got {msg}"
    );
    // No injected error this time → second flush commits the buffered data.
    persister.flush(w).expect("second flush ok");
    assert_eq!(read_synced_height(&path, &w), Some(7));
}

/// store-during-failed-flush merges via LWW.
///
/// Documented `Merge for CoreChangeSet` semantics (see
/// `platform_wallet/changeset/changeset.rs:150-220`): `synced_height`
/// and `last_processed_height` use monotonic-max merging, so the
/// final values are `max(A, B)` per field regardless of order.
#[test]
fn tc_p2_003_store_during_failed_flush_lww() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xC3);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(10, 10)))
        .unwrap();
    persister.force_next_flush_to_fail(make_busy_error());
    let _err = persister.flush(w).expect_err("first flush must fail");
    // B arrives between failed flush and retry.
    persister
        .store(w, changeset(core_with_height(20, 5)))
        .unwrap();
    persister.flush(w).expect("retry must succeed");
    assert_eq!(read_synced_height(&path, &w), Some(20));
    let lp: Option<i64> = {
        use rusqlite::OptionalExtension;
        ro_conn(&path)
            .query_row(
                "SELECT last_processed_height FROM core_sync_state WHERE wallet_id = ?1",
                rusqlite::params![w.as_slice()],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    };
    assert_eq!(lp, Some(10), "monotonic-max merge must keep 10");
}

/// fatal failure WIPES the buffer.
#[test]
fn tc_p2_004_fatal_failure_wipes_buffer() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xC4);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(9, 9)))
        .unwrap();
    persister.force_next_flush_to_fail(make_fatal_error());
    let _err = persister.flush(w).expect_err("first flush must fail");
    // Buffer wiped — second flush is a no-op, no row written.
    persister.flush(w).expect("second flush ok (no-op)");
    assert_eq!(
        read_synced_height(&path, &w),
        None,
        "fatal failure must drop the buffered changeset"
    );
}

/// `FlushMode::Immediate` surfaces `FlushRetryable`.
#[test]
fn tc_p2_006_immediate_surfaces_flush_retryable() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Immediate);
    let w = wid(0xC6);
    ensure_wallet_meta(&persister, &w);
    persister.force_next_flush_to_fail(make_busy_error());
    let err = persister
        .store(w, changeset(core_with_height(3, 3)))
        .expect_err("immediate store must surface the error");
    let msg = match err {
        PersistenceError::Backend { source, .. } => source.to_string(),
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    };
    assert!(
        msg.contains("flush failed transiently"),
        "Immediate mode must surface FlushRetryable, got {msg}"
    );
    // The store buffered the data via take_for_flush + restore. Issue
    // a flush directly — the second attempt commits.
    persister.flush(w).expect("retry ok");
    assert_eq!(read_synced_height(&path, &w), Some(3));
}

/// restore emits a structured `tracing::warn!`.
#[tracing_test::traced_test]
#[test]
fn tc_p2_007_warn_on_restore_with_structured_fields() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xC7);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(8, 8)))
        .unwrap();
    persister.force_next_flush_to_fail(make_busy_error());
    let _ = persister.flush(w).expect_err("first flush must fail");
    // tracing-test exposes a per-test buffer via `logs_contain`.
    assert!(
        logs_contain("flush failed transiently"),
        "WARN message missing"
    );
    assert!(
        logs_contain("error_kind=\"sqlite_busy\""),
        "structured error_kind missing"
    );
    assert!(
        logs_contain("restored_field_count=1"),
        "structured restored_field_count missing"
    );
    assert!(
        logs_contain(&hex::encode(w)),
        "structured wallet_id missing"
    );
}

/// Dropping a Manual-mode persister with uncommitted dirty wallets logs
/// a structured `tracing::error!`. We do NOT auto-flush from Drop.
#[tracing_test::traced_test]
#[test]
fn atom_007_drop_logs_uncommitted_manual_buffer() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xDD);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(11, 11)))
        .unwrap();
    // Buffer is now dirty. Dropping must emit the structured error.
    drop(persister);

    assert!(
        logs_contain("SqlitePersister dropped with uncommitted Manual-mode writes"),
        "drop must emit error line"
    );
    assert!(
        logs_contain("dirty_wallets=1"),
        "structured dirty_wallets field missing"
    );
}

/// An Immediate-mode persister never trips the Drop-time log — every
/// `store` is durable, so there is no uncommitted state by construction.
#[tracing_test::traced_test]
#[test]
fn atom_007_drop_silent_in_immediate_mode() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Immediate);
    let w = wid(0xDE);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(w, changeset(core_with_height(11, 11)))
        .unwrap();
    drop(persister);
    assert!(
        !logs_contain("SqlitePersister dropped with uncommitted"),
        "Immediate-mode drop must NOT log uncommitted state"
    );
}

/// `commit_writes` continues past per-wallet failures, returning a
/// CommitReport with each wallet's outcome. A failed wallet is recorded
/// in `failed`; the remaining wallets still flush.
///
/// We use `force_next_flush_to_fail` to make the FIRST wallet in
/// sorted-id order surface a fatal error. The remaining two wallets
/// must still flush (sorted-id ordering — A < B < C), and the report
/// must list 1 failure + 2 successes.
#[test]
fn atom_006_commit_writes_continues_past_per_wallet_failures() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0xA0);
    let b = wid(0xB0);
    let c = wid(0xC0);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    ensure_wallet_meta(&persister, &c);
    persister
        .store(a, changeset(core_with_height(1, 1)))
        .unwrap();
    persister
        .store(b, changeset(core_with_height(2, 2)))
        .unwrap();
    persister
        .store(c, changeset(core_with_height(3, 3)))
        .unwrap();

    // The injector fires on the FIRST flush_inner. Wallets are flushed
    // in sorted-id order, so it hits wallet A.
    persister.force_next_flush_to_fail(make_fatal_error());
    let report = persister
        .commit_writes()
        .expect("commit_writes itself must return Ok(report)");

    assert_eq!(report.failed.len(), 1, "wallet A must be in failed");
    assert_eq!(report.failed[0].0, a, "failed wallet must be A");
    assert_eq!(
        report.succeeded.len(),
        2,
        "B and C must still flush despite A's failure; report={report:?}"
    );
    assert!(report.succeeded.contains(&b) && report.succeeded.contains(&c));
    assert!(
        report.still_pending.is_empty(),
        "no LockPoisoned short-circuit on a fatal error path"
    );
    assert!(!report.is_ok());

    // Verify B and C are durable; A is not.
    assert_eq!(read_synced_height(&path, &a), None);
    assert_eq!(read_synced_height(&path, &b), Some(2));
    assert_eq!(read_synced_height(&path, &c), Some(3));
}
