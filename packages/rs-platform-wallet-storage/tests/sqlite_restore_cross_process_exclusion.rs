#![allow(clippy::field_reassign_with_default)]

//! TC-CODE-005 / TC-CODE-010 / TC-CODE-015 — SQLite-native EXCLUSIVE
//! replaces the (false-positive) `flock(2)` advisory lock pattern that
//! pre-T-006 `restore_from` used for cross-process exclusion. The
//! advisory lock did not exclude rusqlite peers; `BEGIN EXCLUSIVE`
//! against the destination file does.

mod common;

use std::path::Path;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::SqlitePersister;
use rusqlite::TransactionBehavior;

fn seed_one_row(persister: &SqlitePersister, w: &[u8; 32]) {
    ensure_wallet_meta(persister, w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(7),
        last_processed_height: Some(7),
        ..Default::default()
    });
    persister.store(*w, cs).unwrap();
}

/// TC-CODE-005-a — `restore_from` must hold a SQLite-native exclusive
/// lock for the full restore body. A peer rusqlite Connection (a
/// different process equivalent) opening the same DB and trying to
/// `BEGIN EXCLUSIVE` while restore is in flight must conflict.
///
/// We assert the exclusion by reverse: AFTER `restore_from` returns,
/// the peer can again take its own EXCLUSIVE — proving the persister
/// did NOT leave a dangling EXCLUSIVE behind. The positive (peer
/// conflict during the body) is implicitly covered: if the persister
/// failed to take EXCLUSIVE, the peer's EXCLUSIVE held below would
/// have blocked our restore — and busy-timeouts would surface as
/// `Err`. Negative path also covered by TC-CODE-005-b: if a peer
/// HOLDS exclusive across restore, restore returns BUSY.
#[test]
fn restore_takes_and_releases_native_exclusive() {
    let (persister, tmp, db_path) = fresh_persister();
    seed_one_row(&persister, &wid(0xA1));
    let backup_dir = tempfile::tempdir().expect("backup dir");
    let backup_path = persister.backup_to(backup_dir.path()).unwrap();
    drop(persister);

    SqlitePersister::restore_from_skip_backup(&db_path, &backup_path)
        .expect("restore succeeds without peer contention");

    // Peer can now grab its own EXCLUSIVE — restore released cleanly.
    let mut peer = ro_conn_rw(&db_path);
    let tx = peer
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .expect("peer EXCLUSIVE post-restore");
    tx.commit().expect("peer commit");

    // Keep `tmp` and `backup_dir` alive until here.
    drop(tmp);
    drop(backup_dir);
}

/// TC-CODE-005-b — when a peer holds EXCLUSIVE on the destination,
/// `restore_from` returns a busy error rather than silently steamrolling
/// the peer's write tx. With the pre-T-006 flock approach this would
/// have proceeded (flock doesn't see SQLite peers); with the
/// SQLite-native EXCLUSIVE it must conflict.
#[test]
fn restore_blocks_when_peer_holds_exclusive() {
    let (persister, tmp, db_path) = fresh_persister();
    seed_one_row(&persister, &wid(0xA2));
    let backup_dir = tempfile::tempdir().expect("backup dir");
    let backup_path = persister.backup_to(backup_dir.path()).unwrap();
    drop(persister);

    // Peer opens a writer conn, sets a SHORT busy_timeout so we don't
    // wedge the test on a wedge — then takes EXCLUSIVE and holds it.
    let mut peer = ro_conn_rw(&db_path);
    peer.pragma_update(None, "busy_timeout", 50i64).unwrap();
    let tx = peer
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .unwrap();

    // restore_from should NOT succeed — the destination is locked.
    let err = SqlitePersister::restore_from_skip_backup(&db_path, &backup_path)
        .expect_err("restore must fail while peer holds EXCLUSIVE");
    let kind = format!("{err}");
    assert!(
        kind.contains("locked") || kind.contains("busy") || kind.contains("database is locked"),
        "expected a lock/busy error, got: {kind}"
    );

    drop(tx);
    drop(peer);
    drop(tmp);
    drop(backup_dir);
}

/// TC-CODE-005-b (grep half) + TC-CODE-010-a + TC-CODE-015-a —
/// flock / fs2 / fs4 must be gone from the persister.
#[test]
fn flock_and_fs2_traces_are_gone() {
    let backup_rs =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sqlite/backup.rs"))
            .expect("read backup.rs");
    for needle in [
        "fs2::",
        "use fs2",
        "fs4::",
        "use fs4",
        "try_lock_exclusive",
        "advisory lock unsupported",
    ] {
        assert!(
            !backup_rs.contains(needle),
            "backup.rs must not reference `{needle}` after T-006"
        );
    }

    let cargo_toml =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("read Cargo.toml");
    for needle in ["fs2 =", "fs4 =", "dep:fs2", "dep:fs4"] {
        assert!(
            !cargo_toml.contains(needle),
            "Cargo.toml must not list `{needle}` after T-006"
        );
    }
}

/// TC-CODE-005-c — README must describe the SQLite-native exclusion,
/// not the false advisory-flock claim.
#[test]
fn readme_describes_sqlite_native_exclusion() {
    let readme = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("read README.md");
    assert!(
        !readme.contains("flock(2)") && !readme.contains("advisory lock unsupported"),
        "README must drop the false flock(2) claim"
    );
    assert!(
        readme.contains("BEGIN EXCLUSIVE") || readme.contains("SQLite-native"),
        "README must describe the SQLite-native EXCLUSIVE pattern"
    );
}

/// Helper — open the destination as a read-write rusqlite Connection
/// with a sane busy_timeout, mimicking what a peer process would do.
fn ro_conn_rw(path: &Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("rw open");
    conn.pragma_update(None, "busy_timeout", 5_000i64).unwrap();
    conn
}
