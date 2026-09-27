#![allow(clippy::field_reassign_with_default)]

//! `commit_writes` LockPoisoned short-circuit accounting: a
//! `PersistenceError::LockPoisoned` from any wallet's flush aborts the
//! loop early — the offending wallet lands in `failed` and every
//! not-yet-attempted wallet is moved to `still_pending`.
//!
//! The report-accounting test uses the deterministic
//! `force_next_flush_to_fail` injector. The permanence test poisons the real
//! connection mutex from a panicking thread.

mod common;

use common::{ensure_wallet_meta, fresh_persister_with_mode, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{FlushMode, WalletStorageError};

fn changeset(synced: u32) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(CoreChangeSet {
            synced_height: Some(synced),
            last_processed_height: Some(synced),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Wallets flush in sorted-id order. Priming a `LockPoisoned` to fire on
/// the FIRST flush (wallet A) must:
///   - record A in `failed` (as LockPoisoned),
///   - move the not-yet-attempted wallets B and C into `still_pending`,
///   - leave `succeeded` empty,
///   - and `commit_writes` itself still returns `Ok(report)` (the loop
///     short-circuits cleanly, it does not propagate `Err`).
#[test]
fn lock_poisoned_short_circuit_fills_still_pending() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0xA0);
    let b = wid(0xB0);
    let c = wid(0xC0);
    for id in [&a, &b, &c] {
        ensure_wallet_meta(&persister, id);
    }
    persister.store(a, changeset(1)).unwrap();
    persister.store(b, changeset(2)).unwrap();
    persister.store(c, changeset(3)).unwrap();

    // Fires on the first flush_inner -> sorted order -> wallet A.
    persister.force_next_flush_to_fail(WalletStorageError::LockPoisoned);

    let report = persister
        .commit_writes()
        .expect("commit_writes must return Ok(report), not Err, on a LockPoisoned short-circuit");

    assert_eq!(
        report.failed.len(),
        1,
        "exactly one wallet (A) must be recorded as failed; report={report:?}"
    );
    assert_eq!(report.failed[0].0, a, "the failed wallet must be A");
    assert!(
        matches!(
            report.failed[0].1,
            platform_wallet::changeset::PersistenceError::LockPoisoned
        ),
        "A's failure must be LockPoisoned, got {:?}",
        report.failed[0].1
    );

    assert!(
        report.succeeded.is_empty(),
        "no wallet should have flushed after the short-circuit; report={report:?}"
    );

    let mut pending = report.still_pending.clone();
    pending.sort();
    assert_eq!(
        pending,
        vec![b, c],
        "B and C were never attempted and must land in still_pending; report={report:?}"
    );
    assert!(
        !report.is_ok(),
        "a report with failures must not be is_ok()"
    );
    assert!(
        !persister.buffer_has_changeset_for_test(&a),
        "the failed wallet's fatal changeset must be discarded"
    );
    assert!(persister.buffer_has_changeset_for_test(&b));
    assert!(persister.buffer_has_changeset_for_test(&c));

    // B and C must NOT be durable — the loop never reached them.
    let conn = common::ro_conn(&path);
    for id in [&b, &c] {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
                rusqlite::params![id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            n,
            0,
            "still_pending wallet {} must not have been flushed",
            hex::encode(id)
        );
    }
}

#[test]
fn real_connection_mutex_poison_is_permanent_and_drops_failed_changeset() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let persister = std::sync::Arc::new(persister);
    let wallet_id = wid(0xD0);
    let other_wallet_id = wid(0xD1);
    ensure_wallet_meta(&persister, &wallet_id);
    ensure_wallet_meta(&persister, &other_wallet_id);
    persister.store(wallet_id, changeset(42)).unwrap();
    persister.store(other_wallet_id, changeset(43)).unwrap();
    assert!(persister.buffer_has_changeset_for_test(&wallet_id));
    assert!(persister.buffer_has_changeset_for_test(&other_wallet_id));

    let poisoner = std::sync::Arc::clone(&persister);
    let panic_result = std::thread::spawn(move || {
        let _connection = poisoner.lock_conn_for_test();
        panic!("poison the SQLite connection mutex");
    })
    .join();
    assert!(panic_result.is_err(), "poisoning thread must panic");

    let flush_err = persister
        .flush(wallet_id)
        .expect_err("flush must surface the poisoned connection");
    assert!(matches!(
        flush_err,
        platform_wallet::changeset::PersistenceError::LockPoisoned
    ));
    assert!(
        !persister.buffer_has_changeset_for_test(&wallet_id),
        "the changeset drained by the fatal flush must be discarded"
    );
    assert!(
        !persister.buffer_has_changeset_for_test(&other_wallet_id),
        "connection poison must discard every wallet's buffered changeset"
    );

    assert!(matches!(
        persister.store(wallet_id, changeset(44)),
        Err(platform_wallet::changeset::PersistenceError::LockPoisoned)
    ));
    assert!(matches!(
        persister.flush(wallet_id),
        Err(platform_wallet::changeset::PersistenceError::LockPoisoned)
    ));
    assert!(matches!(
        persister.commit_writes(),
        Err(platform_wallet::changeset::PersistenceError::LockPoisoned)
    ));

    for attempt in 1..=3 {
        let load_err = persister
            .load()
            .expect_err("load must keep surfacing the poisoned connection");
        assert!(
            matches!(
                load_err,
                platform_wallet::changeset::PersistenceError::LockPoisoned
            ),
            "load attempt {attempt} returned {load_err:?}"
        );
    }
    assert!(matches!(
        persister.delete_wallet(wallet_id),
        Err(WalletStorageError::LockPoisoned)
    ));

    drop(persister);
    let reopened = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path)
            .with_flush_mode(FlushMode::Manual),
    )
    .expect("dropping the poisoned instance must release the same-path guard");
    assert!(!reopened.buffer_has_changeset_for_test(&wallet_id));
    let conn = reopened.lock_conn_for_test();
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rows, 0, "the discarded changeset must not replay on reopen");
}
