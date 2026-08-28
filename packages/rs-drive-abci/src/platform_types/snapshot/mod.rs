//! ABCI state sync snapshot types.
//!
//! Snapshots are served directly from the rocksdb checkpoints Drive already creates
//! (`drive.checkpoints`, populated by `create_grovedb_checkpoint` after each qualifying
//! block is committed); there is no separate snapshot store.

use drive::drive::Checkpoint;
use drive::grovedb::replication::MultiStateSyncSession;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tenderdash_abci::proto::abci;

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
