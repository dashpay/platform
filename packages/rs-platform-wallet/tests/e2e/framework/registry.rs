//! Persistent JSON-backed test-wallet registry at
//! `<workdir>/test_wallets.json`. Every `setup` inserts the seed
//! BEFORE returning the wallet so a panic between `setup` and
//! `teardown` leaves a recoverable trail for the next-run
//! [`super::cleanup::sweep_orphans`].
//!
//! Persistence: write-temp + rename via [`tempfile::NamedTempFile`]
//! (atomic on POSIX, `MOVEFILE_REPLACE_EXISTING` on Windows). NOT
//! fsync'd — the next-run sweep tolerates lost updates. A corrupt
//! JSON file is logged and treated as "no orphans".

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::{FrameworkError, FrameworkResult};

/// Stable wallet identifier (mirrors `platform_wallet::WalletId`).
/// Stored hex-encoded in JSON.
pub type WalletSeedHash = [u8; 32];

/// Lifecycle status of a registry entry. `Active` is steady state;
/// `Failed` flags a sweep error for next-startup retry.
///
/// A transient `Sweeping` state was considered for cross-process
/// progress signalling but isn't wired up — the per-slot workdir
/// lock already serialises the only writer that touches a given
/// registry path, so a second process never sees an in-flight sweep
/// from a peer. If we ever share a slot we'll need to add it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EntryStatus {
    #[default]
    Active,
    Failed,
}

/// One row in the registry. Holds enough to reconstruct the wallet
/// via `manager.create_wallet_from_seed_bytes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Hex-encoded 64-byte seed.
    pub seed_hex: String,
    /// Insertion time — debug breadcrumb only.
    pub created_at: SystemTime,
    pub status: EntryStatus,
    /// Free-form note (typically the test name).
    pub note: Option<String>,
}

/// JSON-backed registry guarded by a process-local mutex. File is
/// rewritten via write-temp + rename on every mutation; see module
/// docs for the durability / `fsync` contract.
pub struct PersistentTestWalletRegistry {
    path: PathBuf,
    state: Mutex<HashMap<WalletSeedHash, RegistryEntry>>,
}

impl PersistentTestWalletRegistry {
    /// Open or create the registry. Missing file → empty map;
    /// corrupt JSON is logged and replaced with an empty map
    /// (manual cleanup may be needed). On-disk keys are
    /// hex-encoded; in-memory keys are raw `[u8; 32]`.
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

    /// Path of the backing JSON file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Insert (or overwrite) an entry, persisting before mutating
    /// the in-memory map: the snapshot is built off the current state,
    /// written to disk, and only swapped in once the write succeeds.
    /// A failed write therefore leaves both memory and disk on the
    /// previous state — preserving the module's "persist before
    /// returning" contract under partial failure.
    /// Last-write-wins on duplicate.
    pub fn insert(&self, hash: WalletSeedHash, entry: RegistryEntry) -> FrameworkResult<()> {
        let snapshot = {
            let guard = self.state.lock();
            let mut snapshot = guard.clone();
            snapshot.insert(hash, entry);
            snapshot
        };
        atomic_write_json(&self.path, &snapshot)?;
        *self.state.lock() = snapshot;
        Ok(())
    }

    /// Remove an entry. Missing-key is OK — teardown is best-effort.
    /// Persists before mutating in-memory state (see [`Self::insert`]).
    pub fn remove(&self, hash: &WalletSeedHash) -> FrameworkResult<()> {
        let snapshot = {
            let guard = self.state.lock();
            let mut snapshot = guard.clone();
            snapshot.remove(hash);
            snapshot
        };
        atomic_write_json(&self.path, &snapshot)?;
        *self.state.lock() = snapshot;
        Ok(())
    }

    /// Update [`EntryStatus`]; no-op if the entry is absent. Persists
    /// before mutating in-memory state (see [`Self::insert`]).
    pub fn set_status(&self, hash: &WalletSeedHash, status: EntryStatus) -> FrameworkResult<()> {
        let snapshot = {
            let guard = self.state.lock();
            let mut snapshot = guard.clone();
            if let Some(entry) = snapshot.get_mut(hash) {
                entry.status = status;
            }
            snapshot
        };
        atomic_write_json(&self.path, &snapshot)?;
        *self.state.lock() = snapshot;
        Ok(())
    }

    /// Snapshot of all entries (Active / Failed). The startup sweep
    /// reconstructs each wallet, attempts to drain its credits, and
    /// drops the entry on success; a transient sweep failure flips
    /// the entry to `Failed` so the next run retries.
    pub fn list_orphans(&self) -> Vec<(WalletSeedHash, RegistryEntry)> {
        self.state
            .lock()
            .iter()
            .map(|(hash, entry)| (*hash, entry.clone()))
            .collect()
    }

    /// Status of the entry for `wallet_id`, or `None` if no entry
    /// exists. Cheaper than [`Self::list_orphans`] for tests that
    /// only need to assert on a single entry's lifecycle.
    pub fn get_status(&self, wallet_id: WalletSeedHash) -> Option<EntryStatus> {
        self.state.lock().get(&wallet_id).map(|entry| entry.status)
    }
}

/// Write-temp + rename JSON persist. On Windows
/// [`tempfile::NamedTempFile::persist`] uses `MoveFileEx` with
/// `MOVEFILE_REPLACE_EXISTING` so an existing destination is
/// overwritten (plain `std::fs::rename` fails there on overwrite).
/// No `fsync` — see module docs.
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

    // Same-filesystem temp file is required for atomic rename;
    // `persist` (not `persist_noclobber`) overwrites cross-platform.
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

/// In-memory `[u8; 32]` keys → hex strings for JSON.
fn encode_keys(state: &HashMap<WalletSeedHash, RegistryEntry>) -> HashMap<String, RegistryEntry> {
    state
        .iter()
        .map(|(hash, entry)| (hex::encode(hash), entry.clone()))
        .collect()
}

/// Inverse of [`encode_keys`] — drop malformed hex keys silently
/// so one bad entry doesn't take the whole registry down.
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
        // Reopen; entry must survive.
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
