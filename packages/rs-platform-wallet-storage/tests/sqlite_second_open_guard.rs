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

/// A refused second open must not touch the database file.
///
/// The registry claim is what makes that true, so it has to be taken before
/// the file is pre-created and before migrations run. Claiming afterwards
/// leaves a window in which two concurrent opens both compute their pending-
/// migration list from the same pre-migration history and both apply it — the
/// second pass rebuilding a table the first already rebuilt, on the one file
/// that is the wallet.
///
/// `AlreadyOpen` comes back under either ordering, so the error type proves
/// nothing here. The evidence is the side effect: the losing open leaves no
/// file behind.
#[test]
fn refused_second_open_does_not_touch_the_database_file() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let first = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("first open");

    // Unlink the database out from under the live handle. POSIX keeps its open
    // descriptors valid, so `first` still holds the claim while the path is
    // free — which isolates "did the refused open create and migrate a fresh
    // database?" from every other effect an open has.
    for suffix in ["", "-wal", "-shm"] {
        let mut name = path.clone().into_os_string();
        name.push(suffix);
        let _ = std::fs::remove_file(std::path::PathBuf::from(name));
    }
    assert!(
        !path.exists(),
        "fixture is broken: the database file must be gone before the second open"
    );

    let err = open_err(SqlitePersisterConfig::new(&path));
    assert!(
        matches!(err, WalletStorageError::AlreadyOpen { .. }),
        "expected AlreadyOpen, got {err:?}"
    );
    assert!(
        !path.exists(),
        "the refused open pre-created and migrated a database before checking \
         the registry — the claim is being taken too late"
    );

    drop(first);
}
