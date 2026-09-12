#![allow(clippy::field_reassign_with_default)]

//! Cross-process exclusion for `restore_from` relies on a
//! SQLite-native exclusive locking against the destination file.
//! An advisory `flock(2)` would not exclude rusqlite peers;
//! exclusive locking mode plus `BEGIN EXCLUSIVE` does.

mod common;

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

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

/// Padding is added in 16 MiB chunks; each entry is one scenario attempt.
/// A restore that finishes before any probe lands in the exclusion window
/// is rerun with more padding so a failure is never a matter of timing.
const PADDING_CHUNKS_PER_ATTEMPT: [usize; 3] = [4, 8, 12];
const PADDING_CHUNK_BYTES: i64 = 16 * 1024 * 1024;

fn pad_backup_for_observable_restore(backup_path: &Path, chunks: usize) {
    let conn = rusqlite::Connection::open(backup_path).unwrap();
    conn.execute_batch("CREATE TABLE restore_padding (payload BLOB NOT NULL)")
        .unwrap();
    for _ in 0..chunks {
        conn.execute(
            "INSERT INTO restore_padding VALUES (zeroblob(?1))",
            [PADDING_CHUNK_BYTES],
        )
        .unwrap();
    }
}

fn snapshot_dir(dir: &Path) -> HashSet<std::ffi::OsString> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

/// Wait until restore has staged its temp copy next to `destination`.
///
/// The destination and its `-wal`/`-shm`/`-journal` siblings are ignored:
/// for a missing destination, restore creates an owner-only placeholder
/// BEFORE it takes `BEGIN EXCLUSIVE`, so treating that placeholder as the
/// staged copy would let a probe run in the pre-lock window and succeed.
/// The staged temp is only created once the lock is held.
///
/// Returns `false` when restore finished before a staged copy was seen,
/// so the caller can rerun the scenario with more padding.
fn wait_for_staged_copy<T>(
    destination: &Path,
    existing: &HashSet<std::ffi::OsString>,
    restore: &std::thread::JoinHandle<T>,
) -> bool {
    let dir = destination.parent().unwrap();
    let destination_name = destination.file_name().unwrap().to_os_string();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let staged_file_exists = std::fs::read_dir(dir).unwrap().any(|entry| {
            let name = entry.unwrap().file_name();
            !existing.contains(&name)
                && !name
                    .as_encoded_bytes()
                    .starts_with(destination_name.as_encoded_bytes())
        });
        if staged_file_exists {
            return true;
        }
        if restore.is_finished() {
            return false;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for staged copy"
        );
        std::thread::yield_now();
    }
}

fn is_busy_or_locked<T>(result: &rusqlite::Result<T>) -> bool {
    matches!(
        result,
        Err(rusqlite::Error::SqliteFailure(ref error, _))
            if matches!(
                error.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
    )
}

/// Run `probe` against the destination repeatedly while `restore` is still
/// running. Returns `true` as soon as one probe observes busy/locked.
///
/// A probe that succeeds while restore is still running landed outside
/// the exclusion window (restore releases its lock just before the atomic
/// rename, see `restore_from`), so it is retried. Returns `false` once
/// restore finished without any probe being excluded; the caller then
/// reruns the scenario with more padding rather than guessing at timing.
/// Any other error is a real failure and panics.
fn probe_while_restore_runs<T, R: std::fmt::Debug>(
    restore: &std::thread::JoinHandle<T>,
    mut probe: impl FnMut() -> rusqlite::Result<R>,
) -> bool {
    loop {
        if restore.is_finished() {
            return false;
        }
        let result = probe();
        if is_busy_or_locked(&result) {
            return true;
        }
        if let Err(error) = result {
            panic!("peer probe must observe busy/locked or succeed; got {error:?}");
        }
        std::thread::yield_now();
    }
}

/// Run `scenario` with growing padding until one run observes the
/// exclusion. `scenario` returns `true` when a probe observed busy/locked
/// while restore was running, `false` when restore finished before any
/// probe landed in the exclusion window.
fn assert_exclusion_observed(what: &str, scenario: impl Fn(usize) -> bool) {
    for chunks in PADDING_CHUNKS_PER_ATTEMPT {
        if scenario(chunks) {
            return;
        }
        eprintln!(
            "{what}: restore finished before a probe landed in the exclusion window \
             with {chunks} padding chunks; retrying with more padding"
        );
    }
    panic!(
        "{what}: no probe observed busy/locked across {} attempts",
        PADDING_CHUNKS_PER_ATTEMPT.len()
    );
}

/// `restore_from` must hold a SQLite-native exclusive
/// lock through validation and staging. A peer rusqlite Connection (a
/// different process equivalent) opening the same DB and trying to
/// `BEGIN EXCLUSIVE` while restore is in flight must conflict.
///
/// We assert the exclusion by reverse: AFTER `restore_from` returns,
/// the peer can again take its own EXCLUSIVE — proving the persister
/// did NOT leave a dangling EXCLUSIVE behind. The positive (peer
/// conflict during the body) is implicitly covered: if the persister
/// failed to take EXCLUSIVE, the peer's EXCLUSIVE held below would
/// have blocked our restore — and busy-timeouts would surface as
/// `Err`. The negative path (a peer that HOLDS exclusive across
/// restore makes restore return BUSY) is covered separately below.
#[test]
fn restore_takes_and_releases_native_exclusive() {
    let (persister, tmp, db_path) = fresh_persister();
    seed_one_row(&persister, &wid(0xA1));
    let backup_dir = common::secure_tempdir().expect("backup dir");
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

/// When a peer holds EXCLUSIVE on the destination, `restore_from`
/// returns a busy error rather than silently steamrolling the peer's
/// write tx. An advisory flock would not see SQLite peers and would
/// proceed; the SQLite-native EXCLUSIVE must conflict.
#[test]
fn restore_blocks_when_peer_holds_exclusive() {
    let (persister, tmp, db_path) = fresh_persister();
    seed_one_row(&persister, &wid(0xA2));
    let backup_dir = common::secure_tempdir().expect("backup dir");
    let backup_path = persister.backup_to(backup_dir.path()).unwrap();
    drop(persister);

    // Peer opens a writer conn. `PRAGMA busy_timeout` is
    // connection-local (per the SQLite C API), so the 50ms set here
    // ONLY governs how this peer waits when acquiring EXCLUSIVE —
    // restore's own destination-lock connection sets its own busy
    // timeout independently. Keeping the peer's wait short means we
    // don't wedge the test on a deadlock during EXCLUSIVE acquisition.
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

#[test]
fn restore_excludes_plain_readers_for_restore_duration() {
    assert_exclusion_observed("plain reader", |padding_chunks| {
        let (persister, tmp, db_path) = fresh_persister();
        seed_one_row(&persister, &wid(0xA3));
        let backup_dir = common::secure_tempdir().expect("backup dir");
        let backup_path = persister.backup_to(backup_dir.path()).unwrap();
        drop(persister);

        // Keep the staged-copy phase observable long enough to probe the
        // lock without a test-only production hook.
        pad_backup_for_observable_restore(&backup_path, padding_chunks);

        let existing = snapshot_dir(tmp.path());
        let restore_db = db_path.clone();
        let restore_source = backup_path.clone();
        let restore = std::thread::spawn(move || {
            SqlitePersister::restore_from_skip_backup(&restore_db, &restore_source)
        });

        let observed = wait_for_staged_copy(&db_path, &existing, &restore)
            && probe_while_restore_runs(&restore, || {
                let reader = rusqlite::Connection::open(&db_path)?;
                reader.busy_timeout(Duration::ZERO)?;
                reader.query_row("SELECT COUNT(*) FROM wallets", [], |row| {
                    row.get::<_, i64>(0)
                })
            });

        restore.join().unwrap().expect("restore succeeds");
        drop(tmp);
        drop(backup_dir);
        observed
    });
}

#[test]
fn restore_excludes_peer_creating_missing_destination() {
    assert_exclusion_observed("peer creating a missing destination", |padding_chunks| {
        let (persister, tmp, _source_db_path) = fresh_persister();
        seed_one_row(&persister, &wid(0xA4));
        let backup_dir = common::secure_tempdir().expect("backup dir");
        let backup_path = persister.backup_to(backup_dir.path()).unwrap();
        drop(persister);
        pad_backup_for_observable_restore(&backup_path, padding_chunks);

        let destination = tmp.path().join("restored-missing.db");
        assert!(!destination.exists());
        let existing = snapshot_dir(tmp.path());
        let restore_destination = destination.clone();
        let restore_source = backup_path.clone();
        let restore = std::thread::spawn(move || {
            SqlitePersister::restore_from_skip_backup(&restore_destination, &restore_source)
        });

        let observed = wait_for_staged_copy(&destination, &existing, &restore)
            && probe_while_restore_runs(&restore, || {
                let peer = rusqlite::Connection::open(&destination)?;
                peer.busy_timeout(Duration::ZERO)?;
                peer.execute_batch("CREATE TABLE peer_write (value INTEGER)")
            });

        restore.join().unwrap().expect("restore succeeds");
        drop(tmp);
        drop(backup_dir);
        observed
    });
}

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

/// README must describe the SQLite-native exclusion, not a false
/// advisory-flock claim.
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
