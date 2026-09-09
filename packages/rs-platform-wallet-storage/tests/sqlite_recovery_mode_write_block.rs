#![allow(clippy::field_reassign_with_default)]

//! `LoadPolicy::Recovery` makes the persister read-only.
//!
//! A recovery load hands back a degraded projection of the rows, so every
//! mutating entry point must refuse rather than risk committing that
//! projection over the good data. Reads, and `backup_to` (a read of the
//! source), stay available.

mod common;

use std::fs;

use common::{fresh_recovery_persister, wid, LoadPolicy, SqlitePersister, SqlitePersisterConfig};
use platform_wallet::changeset::{
    CoreChangeSet, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::masternode::{TrackedMasternode, TrackedMasternodeSnapshot};
use platform_wallet_storage::{KvError, KvStore, ObjectId, RetentionPolicy, WalletStorageError};

fn core_changeset() -> PlatformWalletChangeSet {
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(9),
        last_processed_height: Some(9),
        ..Default::default()
    });
    cs
}

/// Assert a `PersistenceError` from the trait boundary carries
/// `ReadOnlyRecoveryMode` naming `operation`.
#[track_caller]
fn assert_blocked(err: PersistenceError, operation: &str) {
    let PersistenceError::Backend { source, .. } = err else {
        panic!("expected a typed backend error, got {err:?}");
    };
    match source.downcast_ref::<WalletStorageError>() {
        Some(WalletStorageError::ReadOnlyRecoveryMode { operation: got }) => {
            assert_eq!(*got, operation, "blocked operation name");
        }
        other => panic!("expected ReadOnlyRecoveryMode, got {other:?}"),
    }
}

fn tracked_masternode(byte: u8) -> TrackedMasternode {
    TrackedMasternode {
        pro_tx_hash: [byte; 32],
        label: Some("rescue".to_string()),
        added_at: byte as u64,
        snapshot: TrackedMasternodeSnapshot::default(),
    }
}

#[test]
fn store_is_blocked_in_recovery() {
    let wallet = wid(0x10);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let err = persister
        .store(wallet, core_changeset())
        .expect_err("store must be refused in recovery mode");
    assert_blocked(err, "store");
}

#[test]
fn store_in_recovery_leaves_buffer_empty() {
    let wallet = wid(0x11);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let _ = persister.store(wallet, core_changeset());
    assert!(
        !persister.buffer_has_changeset_for_test(&wallet),
        "a refused store must not stage a changeset that a later write could drain"
    );
}

#[test]
fn flush_is_blocked_in_recovery() {
    let wallet = wid(0x12);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let err = persister
        .flush(wallet)
        .expect_err("flush must be refused in recovery mode");
    assert_blocked(err, "flush");
}

#[test]
fn commit_writes_is_blocked_in_recovery() {
    let wallet = wid(0x13);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let err = persister
        .commit_writes()
        .expect_err("commit_writes must return Err, never an empty CommitReport");
    assert_blocked(err, "commit_writes");
}

#[test]
fn delete_wallet_is_blocked_in_recovery() {
    let wallet = wid(0x14);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let err = SqlitePersister::delete_wallet(&persister, wallet)
        .expect_err("delete_wallet must be refused in recovery mode");
    assert!(matches!(
        err,
        WalletStorageError::ReadOnlyRecoveryMode {
            operation: "delete_wallet"
        }
    ));
}

#[test]
fn delete_wallet_skip_backup_is_blocked_in_recovery() {
    let wallet = wid(0x15);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let err = persister
        .delete_wallet_skip_backup(wallet)
        .expect_err("delete_wallet_skip_backup must be refused in recovery mode");
    assert!(matches!(
        err,
        WalletStorageError::ReadOnlyRecoveryMode {
            operation: "delete_wallet"
        }
    ));
}

#[test]
fn prune_backups_is_blocked_in_recovery() {
    let (persister, tmp, _path) = fresh_recovery_persister(|_| {});
    let dir = tmp.path().join("backups-to-prune");
    fs::create_dir_all(&dir).expect("create backup dir");
    let err = persister
        .prune_backups(&dir, RetentionPolicy::keep_last(1))
        .expect_err("prune must not shrink the rollback set during a rescue");
    assert!(matches!(
        err,
        WalletStorageError::ReadOnlyRecoveryMode {
            operation: "prune_backups"
        }
    ));
}

#[test]
fn persist_tracked_masternodes_is_blocked_in_recovery() {
    let (persister, _tmp, _path) = fresh_recovery_persister(|_| {});
    let err = persister
        .persist_tracked_masternodes(dashcore::Network::Testnet, &[tracked_masternode(0x21)])
        .expect_err("persist_tracked_masternodes must be refused in recovery mode");
    assert_blocked(err, "persist_tracked_masternodes");
}

/// The gate has to hold the DATA, not just return an error: `replace_all`
/// is whole-set, so an accepted write DELETEs the network's rows first. A
/// host mid-rescue whose in-memory list is empty or short would otherwise
/// zero the very set it opened Recovery to save.
#[test]
fn refused_persist_tracked_masternodes_leaves_the_stored_set_intact() {
    let network = dashcore::Network::Testnet;
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        strict
            .persist_tracked_masternodes(network, &[tracked_masternode(0x22)])
            .expect("seed the tracked set through the strict persister");
    });

    let _ = persister.persist_tracked_masternodes(network, &[]);

    let rows = persister
        .load_tracked_masternodes(network)
        .expect("reads stay available in recovery mode");
    assert_eq!(
        rows.len(),
        1,
        "a refused whole-set write must not delete the rows the rescue is trying to save"
    );
    assert_eq!(rows[0].pro_tx_hash, [0x22u8; 32]);
}

#[test]
fn kv_put_is_blocked_in_recovery() {
    let (persister, _tmp, _path) = fresh_recovery_persister(|_| {});
    let err = persister
        .put(&ObjectId::Global, "k", b"v")
        .expect_err("kv put must be refused in recovery mode");
    assert!(
        matches!(
            err,
            KvError::ReadOnlyRecoveryMode {
                operation: "kv_put"
            }
        ),
        "must be the dedicated variant, not a Sqlite catch-all: {err:?}"
    );
}

#[test]
fn kv_delete_is_blocked_in_recovery() {
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| strict.put(&ObjectId::Global, "k", b"v").expect("seed"));
    let err = persister
        .delete(&ObjectId::Global, "k")
        .expect_err("kv delete must be refused in recovery mode");
    assert!(
        matches!(
            err,
            KvError::ReadOnlyRecoveryMode {
                operation: "kv_delete"
            }
        ),
        "must be the dedicated variant, not a Sqlite catch-all: {err:?}"
    );
}

#[test]
fn kv_reads_are_allowed_in_recovery() {
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| strict.put(&ObjectId::Global, "k", b"v").expect("seed"));
    assert_eq!(
        persister
            .get(&ObjectId::Global, "k")
            .expect("get")
            .as_deref(),
        Some(&b"v"[..])
    );
    assert_eq!(
        persister
            .list_keys(&ObjectId::Global, None)
            .expect("list_keys"),
        vec!["k".to_string()]
    );
}

#[test]
fn backup_to_is_allowed_in_recovery() {
    let wallet = wid(0x16);
    let (persister, tmp, _path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let dest = tmp.path().join("rescue-snapshot.db");
    let written = persister
        .backup_to(&dest)
        .expect("snapshot-before-touching must stay available in recovery mode");
    assert!(written.exists());
}

#[test]
fn restore_over_open_recovery_db_still_returns_already_open() {
    let wallet = wid(0x17);
    let (persister, tmp, path) =
        fresh_recovery_persister(|strict| common::ensure_wallet_meta(strict, &wallet));
    let src = persister
        .backup_to(&tmp.path().join("source.db"))
        .expect("backup");
    let err = SqlitePersister::restore_from(&path, &src, Some(tmp.path()))
        .expect_err("restore must refuse a destination held open by a live persister");
    assert!(matches!(err, WalletStorageError::AlreadyOpen { .. }));
}

#[test]
fn recovery_without_auto_backup_dir_is_rejected() {
    let tmp = common::secure_tempdir().expect("tempdir");
    let cfg = SqlitePersisterConfig::new(tmp.path().join("wallet.db"))
        .with_load_policy(LoadPolicy::Recovery)
        .with_auto_backup_dir(None);
    // `SqlitePersister` is not `Debug`, so `expect_err` is unavailable.
    match SqlitePersister::open(cfg) {
        Err(WalletStorageError::AutoBackupDisabled { .. }) => {}
        Err(other) => panic!("expected AutoBackupDisabled, got {other:?}"),
        Ok(_) => panic!("recovery mode must keep a pre-migration rollback point"),
    }
}

#[test]
fn strict_persister_still_writes() {
    // Guards against the block leaking into the default policy.
    let wallet = wid(0x18);
    let (persister, _tmp, _path) = common::fresh_persister();
    common::ensure_wallet_meta(&persister, &wallet);
    persister
        .store(wallet, core_changeset())
        .expect("strict mode must still accept writes");
    persister.put(&ObjectId::Global, "k", b"v").expect("kv put");
}
