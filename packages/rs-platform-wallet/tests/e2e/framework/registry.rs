//! Persistent test-wallet registry.
//!
//! JSON-backed file under `<workdir>/test_wallets.json` that records
//! every test wallet `setup` produces, **before** the wallet is
//! returned to the test body. If the test panics (or the process is
//! killed) between `setup` and `teardown`, the registry retains the
//! seed and the next process startup runs [`super::cleanup::sweep_orphans`]
//! to recover the funds. On the happy path,
//! [`super::cleanup::teardown_one`] removes the entry.
//!
//! Persistence is best-effort atomic: each mutation writes to a
//! sibling `*.tmp` via [`tempfile::NamedTempFile`] and persists it
//! over the live file. On POSIX this is `rename(2)` (atomic
//! within a single filesystem); on Windows `tempfile::persist`
//! uses `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING` so updates
//! still overwrite cleanly. The contents are NOT `fsync`'d — a
//! crash between the rename and the OS flushing the page cache
//! could lose the most recent update; the next-run sweep
//! tolerates that by treating a missing-but-previously-known
//! wallet as already-cleaned-up. A corrupted JSON file is
//! treated as "no orphans" — the framework logs a warning and
//! starts fresh rather than failing init.
//!
//! Wave 3a delivers the full registry implementation. Higher waves
//! drive the file from `E2eContext::init` (sweep) and
//! `SetupGuard::{setup, teardown}` (insert / remove).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::{FrameworkError, FrameworkResult};

/// Stable wallet identifier — the `WalletId` derived from the seed.
/// Mirrors `platform_wallet::WalletId` (`[u8; 32]`) so the registry
/// can be reasoned about without depending on the in-memory wallet
/// type. Stored hex-encoded in JSON.
pub type WalletSeedHash = [u8; 32];

/// Lifecycle status of a registry entry.
///
/// `Active` is the steady state. `Sweeping` is set transiently during
/// the cleanup sweep so a second process can tell the wallet is
/// already being handled. `Failed` indicates the previous sweep
/// errored (timeout, network glitch); the next startup retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EntryStatus {
    #[default]
    Active,
    Sweeping,
    Failed,
}

/// One row in the registry — enough information to reconstruct the
/// wallet from scratch (seed bytes) and explain the entry's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Hex-encoded 64-byte seed. The wallet itself is not persisted —
    /// it's reconstructible via
    /// `manager.create_wallet_from_seed_bytes(seed_bytes, ...)`.
    pub seed_hex: String,
    /// When the entry was inserted. `SystemTime` serialises as a
    /// non-portable struct via serde's default impl — fine for a
    /// debug breadcrumb.
    pub created_at: SystemTime,
    /// Lifecycle status. See [`EntryStatus`].
    pub status: EntryStatus,
    /// Free-form note set by the inserter (typically the test name).
    pub note: Option<String>,
}

/// JSON-backed test-wallet registry guarded by a process-local mutex
/// so concurrent in-process inserts/removes serialise safely. The
/// file itself is rewritten on every change via
/// [`tempfile::NamedTempFile::persist`] (write-temp + rename) so
/// cross-process visibility is consistent at file granularity on
/// POSIX and Windows alike. See module docs for the durability /
/// `fsync` contract.
pub struct PersistentTestWalletRegistry {
    path: PathBuf,
    state: Mutex<HashMap<WalletSeedHash, RegistryEntry>>,
}

impl PersistentTestWalletRegistry {
    /// Open or create the registry at `path`.
    ///
    /// A missing file is treated as an empty registry. A corrupt
    /// file is logged and replaced with an empty map — losing a
    /// stale registry on parse failure is preferable to refusing to
    /// start the test process. Worst case: the user manually sweeps
    /// any leftover wallets.
    ///
    /// On-disk shape uses hex-encoded `WalletSeedHash` strings as
    /// keys because JSON only allows string-keyed objects;
    /// in-memory the keys are raw `[u8; 32]` for fast hashing /
    /// equality.
    pub fn open(path: PathBuf) -> FrameworkResult<Self> {
        let state = match fs::read(&path) {
            Ok(bytes) if bytes.is_empty() => HashMap::new(),
            Ok(bytes) => serde_json::from_slice::<HashMap<String, RegistryEntry>>(&bytes)
                .map(decode_keys)
                .unwrap_or_else(|err| {
                    tracing::warn!(
                        "test-wallet registry at {} is corrupt ({err}); starting fresh — \
                         orphans from prior runs may need manual cleanup",
                        path.display()
                    );
                    HashMap::new()
                }),
            Err(err) if err.kind() == io::ErrorKind::NotFound => HashMap::new(),
            Err(err) => {
                return Err(FrameworkError::Io(format!(
                    "reading registry {}: {err}",
                    path.display()
                )));
            }
        };
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    /// Path of the JSON file backing this registry. Useful for log
    /// breadcrumbs and tests that want to assert on durability.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Insert (or overwrite) an entry, persisting the new map to
    /// disk before returning. Overwrite-on-duplicate is intentional:
    /// the same seed surfacing twice in one process is almost always
    /// a test bug, but failing the insert would risk leaking the
    /// new entry. Last-write-wins lets the sweep proceed.
    pub fn insert(&self, hash: WalletSeedHash, entry: RegistryEntry) -> FrameworkResult<()> {
        let snapshot = {
            let mut guard = self.state.lock();
            guard.insert(hash, entry);
            guard.clone()
        };
        atomic_write_json(&self.path, &snapshot)
    }

    /// Remove an entry. Missing-key is silently OK: teardown runs in
    /// "best effort" mode and a missing entry simply means the
    /// happy path already cleaned up.
    pub fn remove(&self, hash: &WalletSeedHash) -> FrameworkResult<()> {
        let snapshot = {
            let mut guard = self.state.lock();
            guard.remove(hash);
            guard.clone()
        };
        atomic_write_json(&self.path, &snapshot)
    }

    /// Update the [`EntryStatus`] of an existing entry. No-op when
    /// the entry isn't present.
    pub fn set_status(&self, hash: &WalletSeedHash, status: EntryStatus) -> FrameworkResult<()> {
        let snapshot = {
            let mut guard = self.state.lock();
            if let Some(entry) = guard.get_mut(hash) {
                entry.status = status;
            }
            guard.clone()
        };
        atomic_write_json(&self.path, &snapshot)
    }

    /// Snapshot of every active or failed entry — i.e. wallets the
    /// startup sweep must drain back to the bank.
    ///
    /// Sweeping-status entries are included as well: a previous
    /// process may have crashed mid-sweep without resetting the
    /// status, in which case the new process should pick it up.
    pub fn list_orphans(&self) -> Vec<(WalletSeedHash, RegistryEntry)> {
        self.state
            .lock()
            .iter()
            .map(|(hash, entry)| (*hash, entry.clone()))
            .collect()
    }
}

/// Cross-platform write-temp + rename JSON persist.
///
/// Serialises `state` to a sibling `NamedTempFile` and persists it
/// over `path`. On POSIX this is `rename(2)`; on Windows
/// [`tempfile::NamedTempFile::persist`] uses `MoveFileEx` with
/// `MOVEFILE_REPLACE_EXISTING`, so an already-existing destination
/// is overwritten cleanly (a plain [`std::fs::rename`] would fail
/// with `ERROR_ALREADY_EXISTS` on Windows after the first write).
///
/// No `fsync` is issued — see the module docs for the durability
/// contract.
fn atomic_write_json(
    path: &Path,
    state: &HashMap<WalletSeedHash, RegistryEntry>,
) -> FrameworkResult<()> {
    use std::io::Write;

    let on_disk = encode_keys(state);
    let bytes = serde_json::to_vec_pretty(&on_disk).map_err(|err| {
        FrameworkError::Io(format!("serialising registry to {}: {err}", path.display()))
    })?;
    let parent = path.parent().ok_or_else(|| {
        FrameworkError::Io(format!(
            "registry path {} has no parent directory",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| FrameworkError::Io(format!("creating {}: {err}", parent.display())))?;

    // `NamedTempFile::new_in(parent)` keeps the temp file on the
    // same filesystem as `path`, which is required for atomic
    // rename. Persisting via `persist` (not `persist_noclobber`)
    // overwrites the destination cross-platform.
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|err| {
        FrameworkError::Io(format!("creating temp file in {}: {err}", parent.display()))
    })?;
    tmp.write_all(&bytes).map_err(|err| {
        FrameworkError::Io(format!("writing temp file {}: {err}", tmp.path().display()))
    })?;
    tmp.as_file_mut().flush().map_err(|err| {
        FrameworkError::Io(format!(
            "flushing temp file {}: {err}",
            tmp.path().display()
        ))
    })?;
    tmp.persist(path).map_err(|err| {
        FrameworkError::Io(format!("persisting temp file -> {}: {err}", path.display()))
    })?;
    Ok(())
}

/// Translate the in-memory `[u8; 32]` keys into hex strings for the
/// JSON-on-disk representation.
fn encode_keys(state: &HashMap<WalletSeedHash, RegistryEntry>) -> HashMap<String, RegistryEntry> {
    state
        .iter()
        .map(|(hash, entry)| (hex::encode(hash), entry.clone()))
        .collect()
}

/// Inverse of [`encode_keys`] — reject malformed hex keys silently
/// (a single corrupt entry shouldn't take the whole registry down).
/// The companion `tracing::warn!` lives in `open` so the caller
/// sees one log line per startup, not one per malformed entry.
fn decode_keys(state: HashMap<String, RegistryEntry>) -> HashMap<WalletSeedHash, RegistryEntry> {
    state
        .into_iter()
        .filter_map(|(hex_key, entry)| {
            let bytes = hex::decode(&hex_key).ok()?;
            let hash: WalletSeedHash = bytes.try_into().ok()?;
            Some((hash, entry))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn entry() -> RegistryEntry {
        RegistryEntry {
            seed_hex: "00".repeat(64),
            created_at: SystemTime::UNIX_EPOCH,
            status: EntryStatus::Active,
            note: Some("test".into()),
        }
    }

    #[test]
    fn missing_file_opens_empty() {
        let dir = tmp_dir();
        let reg = PersistentTestWalletRegistry::open(dir.path().join("test_wallets.json")).unwrap();
        assert!(reg.list_orphans().is_empty());
    }

    #[test]
    fn insert_remove_round_trip_persists() {
        let dir = tmp_dir();
        let path = dir.path().join("test_wallets.json");
        let hash: WalletSeedHash = [7u8; 32];

        {
            let reg = PersistentTestWalletRegistry::open(path.clone()).unwrap();
            reg.insert(hash, entry()).unwrap();
        }
        // Reopen — entry must survive.
        {
            let reg = PersistentTestWalletRegistry::open(path.clone()).unwrap();
            assert_eq!(reg.list_orphans().len(), 1);
            reg.remove(&hash).unwrap();
        }
        let reg = PersistentTestWalletRegistry::open(path).unwrap();
        assert!(reg.list_orphans().is_empty());
    }

    #[test]
    fn corrupt_file_falls_back_to_empty() {
        let dir = tmp_dir();
        let path = dir.path().join("test_wallets.json");
        std::fs::write(&path, b"not valid json").unwrap();
        let reg = PersistentTestWalletRegistry::open(path).unwrap();
        assert!(reg.list_orphans().is_empty());
    }
}
