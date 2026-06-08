#![allow(clippy::field_reassign_with_default)]

//! `commit_writes` continue-and-collect accounting: the LockPoisoned
//! short-circuit branch.
//!
//! `commit_writes` loops every dirty wallet and records each outcome in
//! the `CommitReport`. The documented exception (persister.rs:555-563)
//! is that a `PersistenceError::LockPoisoned` from any wallet's flush
//! aborts the loop early — the offending wallet lands in `failed`, and
//! every wallet NOT yet attempted is shovelled into `still_pending` so
//! the caller knows what was never tried.
//!
//! `src/sqlite/error.rs` carries a `TODO(qa)` noting this branch had no
//! automated end-to-end coverage. This test closes that gap
//! deterministically via the `force_next_flush_to_fail` injector
//! (a real panicking-thread mutex poison is non-deterministic; the
//! injector drives the exact same `PersistenceError::LockPoisoned`
//! through `flush_inner` -> `handle_flush_error`'s fatal branch).

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
