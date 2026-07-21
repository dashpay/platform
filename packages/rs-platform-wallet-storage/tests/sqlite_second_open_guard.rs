//! Second-open guard: a process-wide registry refuses a second
//! `SqlitePersister::open()` on the same canonical path while the first
//! is alive, so two in-process handles can't diverge (each owns an
//! independent `Mutex<Connection>` + write buffer). Dropping the first
//! releases the claim so a later open succeeds.

mod common;

use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

/// `SqlitePersister` is not `Debug`, so `Result::expect_err` can't be
/// used on an `open()` result — extract the error by matching instead.
fn open_err(cfg: SqlitePersisterConfig) -> WalletStorageError {
    match SqlitePersister::open(cfg) {
        Ok(_) => panic!("expected open() to fail"),
        Err(e) => e,
    }
}

#[test]
fn second_open_on_same_path_is_refused() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");

    let first = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("first open");

    let err = open_err(SqlitePersisterConfig::new(&path));
    assert!(
        matches!(err, WalletStorageError::AlreadyOpen { .. }),
        "expected AlreadyOpen, got {err:?}"
    );

    // Releasing the first handle frees the claim.
    drop(first);
    let _reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path))
        .expect("open after the first handle drops must succeed");
}

#[test]
fn distinct_paths_open_concurrently() {
    let tmp = common::secure_tempdir().unwrap();
    let a = tmp.path().join("a.db");
    let b = tmp.path().join("b.db");

    let _pa = SqlitePersister::open(SqlitePersisterConfig::new(&a)).expect("open a");
    // A different path is unaffected by the registry.
    let _pb = SqlitePersister::open(SqlitePersisterConfig::new(&b)).expect("open b");
}

#[test]
fn second_open_via_noncanonical_path_is_refused() {
    // A `.`-segmented path canonicalizes to the same key as the plain
    // path, so the registry still catches the second open.
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let _first = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("first open");

    let dotted = tmp.path().join(".").join("w.db");
    let err = open_err(SqlitePersisterConfig::new(&dotted));
    assert!(
        matches!(err, WalletStorageError::AlreadyOpen { .. }),
        "expected AlreadyOpen for the equivalent path, got {err:?}"
    );
}
