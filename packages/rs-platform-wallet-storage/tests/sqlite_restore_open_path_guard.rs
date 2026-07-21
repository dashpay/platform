#![allow(clippy::field_reassign_with_default)]

//! `restore_from` must refuse to overwrite a database that a live
//! [`SqlitePersister`] in this process is still holding open — that
//! handle's write buffer / connection would silently diverge from the
//! restored bytes. The guard mirrors `open()`'s in-process open-path
//! registry and clears once the holder drops.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet_storage::{SqlitePersister, WalletStorageError};

/// While a persister holds the destination open, both restore entry
/// points return [`WalletStorageError::AlreadyOpen`]; after it drops, the
/// restore succeeds.
#[test]
fn restore_refuses_an_in_process_open_destination() {
    let (persister, _tmp, db_path) = fresh_persister();
    ensure_wallet_meta(&persister, &wid(0xA1));

    // A valid wallet-storage backup to restore from.
    let backup_dir = common::secure_tempdir().expect("backup dir");
    let backup_path = persister
        .backup_to(backup_dir.path())
        .expect("online backup");

    // The destination is still open in this process → refuse.
    let err = SqlitePersister::restore_from_skip_backup(&db_path, &backup_path)
        .expect_err("restore onto an open db must be refused");
    assert!(
        matches!(err, WalletStorageError::AlreadyOpen { .. }),
        "expected AlreadyOpen, got {err:?}"
    );

    // The safe-by-default entry point guards before the auto-backup too.
    let err = SqlitePersister::restore_from(&db_path, &backup_path, Some(backup_dir.path()))
        .expect_err("safe restore onto an open db must be refused");
    assert!(
        matches!(err, WalletStorageError::AlreadyOpen { .. }),
        "expected AlreadyOpen, got {err:?}"
    );

    // Once the holder drops, the open-path registry clears and the restore
    // goes through.
    drop(persister);
    SqlitePersister::restore_from_skip_backup(&db_path, &backup_path)
        .expect("restore succeeds after the holder closes");
}
