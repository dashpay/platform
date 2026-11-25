//! One live shielded store per file, across processes: a child process
//! cannot open a file this process's store owns, and can once it is dropped.
//!
//! Its own test binary: spawning a child forks this process, and a fork
//! briefly shares every open lock file with the child, which would make a
//! concurrent drop-and-reopen in another test of the same binary see the
//! file as still owned.

#![cfg(feature = "shielded")]

use std::path::{Path, PathBuf};

use platform_wallet::wallet::shielded::{FileBackedShieldedStore, FileShieldedStoreError};

const PROBE_PATH: &str = "PLATFORM_WALLET_STORE_LOCK_PROBE_PATH";
const PROBE_EXPECT: &str = "PLATFORM_WALLET_STORE_LOCK_PROBE_EXPECT";

fn temp_store_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_store_owner_lock_{nanos}.sqlite"))
}

fn remove_store_files(path: &Path) {
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let mut file = path.as_os_str().to_owned();
        file.push(suffix);
        let _ = std::fs::remove_file(file);
    }
}

fn assert_already_open<T>(result: Result<T, FileShieldedStoreError>, path: &Path) {
    match result {
        Err(FileShieldedStoreError::AlreadyOpen { path: refused }) => assert_eq!(refused, path),
        Err(other) => panic!("expected AlreadyOpen, got {other}"),
        Ok(_) => panic!("a second open of a live store must be refused"),
    }
}

/// Run in a child process by `another_process_cannot_open_a_live_store`;
/// a no-op otherwise.
#[test]
fn store_lock_probe_child() {
    let Ok(path) = std::env::var(PROBE_PATH) else {
        return;
    };
    let path = PathBuf::from(path);
    let opened = FileBackedShieldedStore::open_path(&path, 100);
    match std::env::var(PROBE_EXPECT).as_deref() {
        Ok("refused") => assert_already_open(opened, &path),
        _ => drop(opened.expect("the file is free")),
    }
}

#[test]
fn another_process_cannot_open_a_live_store() {
    let path = temp_store_path();
    let probe = |expect: &str| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "store_lock_probe_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(PROBE_PATH, &path)
            .env(PROBE_EXPECT, expect)
            .output()
            .expect("spawn the test binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "the child expecting {expect} failed: {stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    let store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
    probe("refused");
    drop(store);
    probe("opens");
    remove_store_files(&path);
}
