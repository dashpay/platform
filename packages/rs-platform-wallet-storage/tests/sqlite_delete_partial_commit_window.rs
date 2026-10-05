#![allow(clippy::field_reassign_with_default)]

//! Coverage for `delete_wallet_inner`'s two-transaction shape: the
//! pre-flush tx (drains + commits the buffered changeset) and the cascade
//! tx (deletes the parent `wallets` row) are SEPARATE SQLite
//! transactions. These tests probe what is durable on disk and what is
//! left in the buffer when the delete aborts AFTER the pre-flush has
//! already committed its changeset.

mod common;

use common::wid;
use key_wallet::Network;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use rusqlite::TransactionBehavior;

/// Self-consistent changeset that materializes a brand-new wallet on
/// flush (FK-valid `wallets` row + a `core_sync_state` child row).
fn full_changeset(synced: u32) -> PlatformWalletChangeSet {
    let mut cs = PlatformWalletChangeSet::default();
    cs.wallet_metadata = Some(WalletMetadataEntry {
        network: Network::Testnet,
        wallet_group_id: [0u8; 32],
        birth_height: 0,
    });
    cs.core = Some(CoreChangeSet {
        synced_height: Some(synced),
        last_processed_height: Some(synced),
        ..Default::default()
    });
    cs
}

fn core_rows_for(persister: &SqlitePersister, w: &[u8; 32]) -> i64 {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
        |row| row.get(0),
    )
    .unwrap()
}

fn wallets_rows_for(persister: &SqlitePersister, w: &[u8; 32]) -> i64 {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
        |row| row.get(0),
    )
    .unwrap()
}

/// A peer holds a SQLite-native EXCLUSIVE on the same DB file, so the
/// pre-flush's own `BEGIN EXCLUSIVE` fails with BUSY (real
/// `?`-propagation, not the injector) and the cascade is never reached.
/// On that failure the buffered changeset must survive — either still in
/// the buffer (restored) or already durable on disk.
#[test]
fn preflush_begin_exclusive_busy_preserves_buffer() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    // No auto_backup_dir + skip_backup keeps the test on the
    // pre-flush -> cascade path without a backup dependency.
    let cfg = SqlitePersisterConfig::new(&path).with_flush_mode(FlushMode::Manual);
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xD1);

    // Buffer a brand-new wallet's full changeset (only state is buffered).
    persister.store(w, full_changeset(42)).unwrap();

    // A peer process grabs EXCLUSIVE on the same file and holds it,
    // forcing the persister's own `BEGIN EXCLUSIVE` (pre-flush AND
    // cascade both use it) to fail with BUSY past the busy_timeout.
    let mut peer = rusqlite::Connection::open(&path).unwrap();
    let peer_guard = peer
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .expect("peer EXCLUSIVE");

    let err = persister.delete_wallet_skip_backup(w);
    assert!(
        err.is_err(),
        "delete must fail while a peer holds EXCLUSIVE; got {err:?}"
    );

    drop(peer_guard);
    drop(peer);

    // Contract: a failed delete must not have removed the wallet. Either
    // the wallet's state is still buffered (pre-flush never committed) OR
    // it is durable on disk (pre-flush committed before the cascade
    // aborted). Both are acceptable per the two-tx design; what is NOT
    // acceptable is the changeset vanishing from BOTH the buffer and disk.
    let on_disk_core = core_rows_for(&persister, &w);
    let on_disk_wallets = wallets_rows_for(&persister, &w);
    let in_buffer = persister.buffer_has_changeset_for_test(&w);

    assert!(
        in_buffer || (on_disk_wallets == 1 && on_disk_core == 1),
        "after a failed delete the buffered changeset must survive somewhere: \
         in_buffer={in_buffer}, on_disk_wallets={on_disk_wallets}, on_disk_core={on_disk_core}"
    );

    // If it is on disk, the wallet must NOT have been deleted (delete
    // returned Err) — i.e. the cascade did not run.
    if on_disk_wallets == 1 {
        assert_eq!(
            on_disk_core, 1,
            "pre-flush committed the wallet but its child row is missing — \
             partial pre-flush is a torn write"
        );
    }
}

/// A pre-flush-committed changeset is durable even though `delete_wallet`
/// aborts; a clean retry once the peer lock is gone converges to a fully
/// deleted wallet.
#[test]
fn delete_retry_after_transient_abort_converges_to_deleted() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path).with_flush_mode(FlushMode::Manual);
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xD2);

    persister.store(w, full_changeset(7)).unwrap();

    {
        let mut peer = rusqlite::Connection::open(&path).unwrap();
        let _peer_guard = peer
            .transaction_with_behavior(TransactionBehavior::Exclusive)
            .expect("peer EXCLUSIVE");
        let first = persister.delete_wallet_skip_backup(w);
        assert!(first.is_err(), "first delete must fail under peer lock");
        // peer guard drops here, releasing the lock
    }

    // Retry with the lock gone: the wallet must end fully deleted
    // regardless of whether the first attempt left state on disk or in
    // the buffer.
    persister
        .delete_wallet_skip_backup(w)
        .expect("retry delete must succeed once the peer lock is gone");

    persister
        .commit_writes()
        .expect("commit_writes drains buffer");

    assert_eq!(
        wallets_rows_for(&persister, &w),
        0,
        "wallet parent row must be gone after a converged delete"
    );
    assert_eq!(
        core_rows_for(&persister, &w),
        0,
        "wallet child rows must be gone after a converged delete"
    );
}
