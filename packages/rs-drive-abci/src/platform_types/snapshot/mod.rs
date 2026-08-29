//! ABCI state sync snapshot types.
//!
//! Snapshots are served directly from the rocksdb checkpoints Drive already creates
//! (`drive.checkpoints`, populated by `create_grovedb_checkpoint` after each qualifying
//! block is committed); there is no separate snapshot store.

use drive::drive::{Checkpoint, Drive};
use drive::grovedb::replication::MultiStateSyncSession;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tenderdash_abci::proto::abci;

/// Name of the marker file that records "a state sync restore is in progress".
///
/// ## Why a plain file, and not aux storage
///
/// The obvious home would be grovedb's aux column family, which is not part of the
/// provable tree. It cannot be used here: `GroveDb::wipe()` clears the aux column family
/// along with `default`, `roots` and `meta`
/// (`grovedb/storage/src/rocksdb_storage/storage.rs`, `wipe()` iterates all four). Since
/// wiping is exactly what both the offer path and the recovery path do, a sentinel living
/// in aux would be destroyed by the very operations it exists to survive, and its
/// lifetime would depend on subtle ordering between the write and the wipe.
///
/// A file next to the database has none of those problems: it is outside everything
/// grovedb touches, it survives any wipe, it costs one `stat` at startup, and an operator
/// can see it. It is deliberately NOT in the provable tree either — it is node-local
/// recovery bookkeeping and must never affect the app hash.
pub const RESTORE_IN_PROGRESS_FILE_NAME: &str = "state_sync_restore_in_progress";

/// Path of the restore sentinel for a given database directory.
pub fn restore_sentinel_path(db_path: &Path) -> PathBuf {
    db_path.join(RESTORE_IN_PROGRESS_FILE_NAME)
}

/// Records that a state sync restore has started and the database is therefore allowed to
/// be inconsistent until it finishes.
///
/// Written BEFORE the wipe, so the window in which the database has been destroyed but
/// nothing marks it as such is empty. The contents are for operators only; the code cares
/// solely about the file's presence.
pub fn write_restore_sentinel(
    db_path: &Path,
    app_hash: &[u8; 32],
    height: u64,
) -> std::io::Result<()> {
    std::fs::create_dir_all(db_path)?;
    std::fs::write(
        restore_sentinel_path(db_path),
        format!(
            "state sync restore in progress\nheight: {}\napp_hash: {}\n",
            height,
            hex::encode(app_hash)
        ),
    )
}

/// Clears the restore sentinel. Only ever called once the node is in a self-consistent
/// state: after a restore has fully completed, after startup recovery has wiped, or after
/// a genesis initialization.
pub fn clear_restore_sentinel(db_path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(restore_sentinel_path(db_path)) {
        Ok(()) => Ok(()),
        // Absent is the normal case on every path that clears defensively.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Whether a restore was in progress when this node last stopped.
pub fn restore_sentinel_exists(db_path: &Path) -> bool {
    restore_sentinel_path(db_path).exists()
}

/// Drops every Drive cache that was derived from grovedb.
///
/// A wipe destroys the state these caches were built from. Left in place they would be
/// silently merged into whatever replaces it: `ProtocolVersionsCache` in particular keeps
/// a `loaded` flag, so `load_if_needed` would never re-read the new version counters and
/// the next block would write vote counts derived from the wiped chain — an immediate app
/// hash fork. Resetting the counter wholesale (rather than `clear_global_cache`) is
/// deliberate: it clears that flag too, so the cache reloads on first use.
///
/// `system_data_contracts` is deliberately NOT cleared — those are compiled-in,
/// version-keyed contracts that never come from grovedb.
pub fn reset_drive_caches_after_wipe(drive: &Drive) {
    *drive.cache.protocol_versions_counter.write() = Default::default();
    drive.cache.data_contracts.clear();
    *drive.cache.genesis_time_ms.write() = None;
}

/// Wipes grovedb and drops the caches derived from it, leaving the node an empty but
/// entirely self-consistent slate.
///
/// This is the single place both the offer path and the crash-recovery path go through,
/// so the two can never drift apart.
pub fn wipe_drive_for_restore(drive: &Drive) -> Result<(), drive::error::Error> {
    drive.grove.wipe()?;
    reset_drive_caches_after_wipe(drive);
    Ok(())
}

/// The grovedb state sync wire protocol versions this node can serve and consume.
///
/// This is THE single supported-set constant: when grovedb wire version 2 lands, add it
/// here and add a `DriveAbciStateSyncVersions` const selecting it in rs-platform-version
/// (`drive_abci.state_sync.protocol_version` is the version stamped on snapshots this
/// node offers).
pub const SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS: &[u16] = &[1];

/// Maximum accepted size (in bytes) of a single snapshot chunk, enforced before any
/// grovedb decode of peer-supplied data (issue #3773).
pub const MAX_STATE_SYNC_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Maximum accepted size (in bytes) of a chunk id, enforced before any grovedb decode
/// of peer-supplied data (issue #3773). Chunk ids are packed vectors of 32-byte subtree
/// prefixes plus short traversal instructions, so well-formed ids stay far below this.
pub const MAX_STATE_SYNC_CHUNK_ID_SIZE: usize = 64 * 1024;

/// Maximum number of subtrees processed in a single batch of a grovedb state sync
/// session on the consuming side.
pub const STATE_SYNC_SUBTREES_BATCH_SIZE: usize = 64;

/// A state sync transfer in progress on the consuming side.
pub struct SnapshotFetchingSession<'db> {
    /// The snapshot being restored
    pub snapshot: abci::Snapshot,
    /// The light-client-verified app hash for the snapshot height, from Tenderdash
    pub app_hash: [u8; 32],
    /// The grovedb state sync wire protocol version this transfer speaks — taken from
    /// the offered snapshot's `version`, validated against
    /// [`SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS`], and used for every chunk of the
    /// transfer.
    pub wire_version: u16,
    /// The grovedb state sync session
    pub state_sync_info: Pin<Box<MultiStateSyncSession<'db>>>,
}

/// How long a served checkpoint stays pinned after the last chunk request for it.
///
/// A state-syncing peer requests chunks continuously; if none arrived for this long the
/// transfer is considered abandoned and the pin is released, allowing a checkpoint that
/// pruning already marked for deletion to be removed from disk.
const SERVING_PIN_INACTIVITY_TTL: Duration = Duration::from_secs(600);

/// Keeps checkpoints that are actively being served to state-syncing peers alive.
///
/// Checkpoint pruning marks old checkpoints for deletion and drops them from the
/// registry; the directory is removed when the last `Arc<Checkpoint>` drops. Holding an
/// `Arc` clone here for every checkpoint a peer is currently downloading extends that
/// refcount, so a checkpoint cannot be deleted mid-transfer. Pins are released after
/// [`SERVING_PIN_INACTIVITY_TTL`] of inactivity.
#[derive(Default)]
pub struct SnapshotManager {
    /// Height -> (pinned checkpoint, instant of the most recent chunk request)
    serving_pins: RwLock<BTreeMap<u64, (Arc<Checkpoint>, Instant)>>,
}

impl SnapshotManager {
    /// Creates a new snapshot manager with no active pins
    pub fn new() -> Self {
        Self::default()
    }

    /// Pins a checkpoint that is being served (or refreshes the pin of one that already
    /// is), and drops pins whose transfers have been inactive for longer than the TTL.
    pub fn pin_for_serving(&self, height: u64, checkpoint: Arc<Checkpoint>) {
        let now = Instant::now();
        let mut pins = self
            .serving_pins
            .write()
            .expect("serving pins lock poisoned");
        pins.retain(|_, (_, last_served)| {
            now.saturating_duration_since(*last_served) < SERVING_PIN_INACTIVITY_TTL
        });
        pins.insert(height, (checkpoint, now));
    }

    /// Returns a pinned checkpoint for the given height, if the pin is still held.
    ///
    /// Used to keep serving a snapshot whose checkpoint pruning has already dropped
    /// from the registry.
    pub fn pinned_checkpoint(&self, height: u64) -> Option<Arc<Checkpoint>> {
        self.serving_pins
            .read()
            .expect("serving pins lock poisoned")
            .get(&height)
            .map(|(checkpoint, _)| Arc::clone(checkpoint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_wire_versions_include_the_version_platform_versions_stamp() {
        use dpp::version::PlatformVersion;
        // Every platform version stamps its state_sync.protocol_version on the
        // snapshots it offers; the supported set must accept what we serve.
        for platform_version in dpp::version::ALL_VERSIONS
            .map(PlatformVersion::get)
            .filter_map(Result::ok)
        {
            assert!(
                SUPPORTED_STATE_SYNC_PROTOCOL_VERSIONS
                    .contains(&platform_version.drive_abci.state_sync.protocol_version),
                "platform version {} stamps unsupported state sync wire version {}",
                platform_version.protocol_version,
                platform_version.drive_abci.state_sync.protocol_version
            );
        }
    }
}
