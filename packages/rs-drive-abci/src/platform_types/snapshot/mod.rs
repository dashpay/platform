//! ABCI state sync snapshot types.
//!
//! Snapshots are served directly from the rocksdb checkpoints Drive already creates
//! (`drive.checkpoints`, populated by `create_grovedb_checkpoint` after each qualifying
//! block is committed); there is no separate snapshot store.

use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
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

/// Clears the restore sentinel at a point where the node is already self-consistent —
/// after a completed restore, or after a genesis initialization — WITHOUT being able to
/// fail the operation that got it there.
///
/// Propagating an I/O error from here would turn a fully successful restore (or a working
/// genesis) into a hard ABCI error over nothing but a `remove_file` hiccup. The cost of
/// failing to remove it is bounded and safe in the other direction: the next startup sees
/// a sentinel, wipes, and re-syncs. Loud, but never a wedge.
pub fn clear_restore_sentinel_best_effort(db_path: &Path) {
    if let Err(error) = clear_restore_sentinel(db_path) {
        tracing::error!(
            ?error,
            path = ?restore_sentinel_path(db_path),
            "[state_sync] could not clear the state sync restore sentinel; the node is \
             consistent, but the next restart will wipe and re-sync unnecessarily. Remove \
             the file by hand to avoid that.",
        );
    }
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
///
/// The checkpoint registry goes too, and it is not merely a cache: `Drive::open` populates
/// `drive.checkpoints` before any wipe can run, and `list_snapshots` serves whatever is in
/// it to peers. Left alone, a node that wiped and re-synced would keep offering snapshots
/// of the chain it just discarded. The entries are marked for deletion first so their
/// directories are removed when the last `Arc` drops, rather than leaking on disk.
pub fn reset_drive_caches_after_wipe(drive: &Drive) {
    *drive.cache.protocol_versions_counter.write() = Default::default();
    drive.cache.data_contracts.clear();
    *drive.cache.genesis_time_ms.write() = None;

    let checkpoints = drive.checkpoints.load();
    for checkpoint_info in checkpoints.values() {
        checkpoint_info.checkpoint.mark_for_deletion();
    }
    drive.checkpoints.store(Arc::new(BTreeMap::new()));
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

/// The grovedb state sync protocol versions this node can serve and consume.
///
/// Exactly one protocol version exists: state sync never shipped, so grovedb updates
/// its replication protocol in place and stays at version 1. This single supported-set
/// constant and the offered-snapshot validation against it exist so that any future
/// incompatible protocol change fails fast on both the serving and consuming side
/// instead of producing a corrupt restore. (`drive_abci.state_sync.protocol_version`
/// is the version stamped on snapshots this node offers.)
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

/// Encodes the Platform protocol version a snapshot was produced at into the ABCI
/// snapshot `metadata` field.
///
/// The consuming node cannot derive this: a node that state syncs has no saved state, so
/// its in-memory platform state is still at [`dpp::version::INITIAL_PROTOCOL_VERSION`]
/// and its Drive version table — including `grove_version` — is the wrong one for the
/// snapshot. grovedb's replication, tree opening and root-hash rules are version gated,
/// so serving and consuming MUST use the same table or the restore is decoded under
/// different rules than it was generated with.
///
/// The value is peer-supplied and therefore untrusted, which is safe: the restore is only
/// ever accepted against the light-client-verified app hash, so a lie produces a failed
/// verification and a `REJECT_SNAPSHOT`, never a silently wrong database.
pub fn encode_snapshot_metadata(protocol_version: ProtocolVersion) -> Vec<u8> {
    protocol_version.to_be_bytes().to_vec()
}

/// Decodes the Platform protocol version out of an ABCI snapshot's `metadata` field.
/// Returns `None` for anything that is not exactly the encoding above.
pub fn decode_snapshot_metadata(metadata: &[u8]) -> Option<ProtocolVersion> {
    <[u8; 4]>::try_from(metadata)
        .ok()
        .map(ProtocolVersion::from_be_bytes)
}

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
    /// The Platform version the snapshot was PRODUCED at, decoded from the offered
    /// snapshot's metadata (see [`encode_snapshot_metadata`]). Every grovedb call of this
    /// transfer — session start, chunk application, commit, verification and the final
    /// root hash — uses this version's table, not the fresh node's own.
    pub platform_version: &'static PlatformVersion,
    /// The grovedb state sync session
    pub state_sync_info: Pin<Box<MultiStateSyncSession<'db>>>,
}

/// How long a served checkpoint stays pinned after the last chunk request for it.
///
/// A state-syncing peer requests chunks continuously; if none arrived for this long the
/// transfer is considered abandoned and the pin is released, allowing a checkpoint that
/// pruning already marked for deletion to be removed from disk.
const SERVING_PIN_INACTIVITY_TTL: Duration = Duration::from_secs(600);

/// Absolute lifetime of a serving pin, regardless of activity.
///
/// The inactivity TTL alone is refreshable, so a peer that keeps touching a height keeps
/// its checkpoint alive forever; this deadline is deliberately NOT refreshable.
///
/// It is the backstop, not the primary bound — [`max_serving_pins`] is what actually
/// limits how many directories can be held back at once — so it is set well above any
/// plausible honest transfer rather than tight. A full mainnet-state restore measures in
/// seconds; six hours leaves enormous room for a slow or rate-limited peer whose
/// checkpoint gets pruned mid-transfer, while still bounding how long a pinned directory
/// can outlive its checkpoint.
const SERVING_PIN_MAX_LIFETIME: Duration = Duration::from_secs(6 * 3600);

/// How many pins are allowed on top of the number of snapshots the node retains.
///
/// The interesting pins are the ones for checkpoints pruning has ALREADY dropped from the
/// registry — those are the directories a pin holds back from deletion. There can only
/// ever be a handful of them legitimately (a transfer that started before the checkpoint
/// aged out), so the retained count plus this slack is generous.
const SERVING_PIN_SLACK: usize = 4;

/// Cap on how many checkpoints may be pinned for serving at once, given how many snapshots
/// the node is configured to retain.
///
/// Without a cap, a peer could keep one transfer alive per height it ever touched: pruning
/// keeps advancing, the peer keeps refreshing, and the number of checkpoint directories
/// held back from deletion grows without bound regardless of `MAX_NUM_SNAPSHOTS`. The cap
/// only ever bites on abuse; when it does, the least recently served pin goes first, and a
/// peer whose pin is evicted can still resolve the checkpoint from the registry if it is
/// still there.
pub fn max_serving_pins(max_num_snapshots: usize) -> usize {
    max_num_snapshots.saturating_add(SERVING_PIN_SLACK)
}

/// A checkpoint held back from deletion for a transfer in flight.
struct ServingPin {
    checkpoint: Arc<Checkpoint>,
    /// When the pin was first taken — bounds its absolute lifetime
    pinned_at: Instant,
    /// When a chunk was last successfully served from it — bounds its idle lifetime
    last_served: Instant,
}

impl ServingPin {
    fn is_live(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_served) < SERVING_PIN_INACTIVITY_TTL
            && now.saturating_duration_since(self.pinned_at) < SERVING_PIN_MAX_LIFETIME
    }
}

/// Keeps checkpoints that are actively being served to state-syncing peers alive.
///
/// Checkpoint pruning marks old checkpoints for deletion and drops them from the
/// registry; the directory is removed when the last `Arc<Checkpoint>` drops. Holding an
/// `Arc` clone here for every checkpoint a peer is currently downloading extends that
/// refcount, so a checkpoint cannot be deleted mid-transfer.
///
/// A pin must not be something a remote peer can hold open indefinitely, so it is bounded
/// three ways: [`SERVING_PIN_INACTIVITY_TTL`] since the last chunk actually served,
/// [`SERVING_PIN_MAX_LIFETIME`] since it was taken (not refreshable), and
/// [`max_serving_pins`] in total. Expiry also must not depend on peers making further
/// requests, or an abandoned transfer would hold its directory forever:
/// [`SnapshotManager::release_expired_pins`] runs once per block and every read of a pin
/// re-checks both deadlines.
#[derive(Default)]
pub struct SnapshotManager {
    /// Height -> the pin held for a transfer of that snapshot
    serving_pins: RwLock<BTreeMap<u64, ServingPin>>,
}

impl SnapshotManager {
    /// Creates a new snapshot manager with no active pins
    pub fn new() -> Self {
        Self::default()
    }

    /// Pins a checkpoint that is being served (or refreshes the pin of one that already
    /// is), dropping expired pins and holding the total to `max_pins` (see
    /// [`max_serving_pins`]).
    ///
    /// Call this only AFTER a chunk was successfully served: a request that could not be
    /// answered must not be able to keep a checkpoint alive.
    pub fn pin_for_serving(&self, height: u64, checkpoint: Arc<Checkpoint>, max_pins: usize) {
        let now = Instant::now();
        let mut pins = self
            .serving_pins
            .write()
            .expect("serving pins lock poisoned");
        retain_live_pins(&mut pins, now);

        if let Some(pin) = pins.get_mut(&height) {
            // Refresh the idle deadline only — `pinned_at` is deliberately untouched so
            // the absolute lifetime cannot be extended by activity.
            pin.last_served = now;
            return;
        }

        // Evict the least recently served pin to make room for a genuinely new one
        while pins.len() >= max_pins.max(1) {
            let Some(coldest) = pins
                .iter()
                .min_by_key(|(_, pin)| pin.last_served)
                .map(|(pinned_height, _)| *pinned_height)
            else {
                break;
            };
            tracing::warn!(
                evicted_height = coldest,
                new_height = height,
                "[state_sync] serving pin limit reached, releasing the least recently served pin",
            );
            pins.remove(&coldest);
        }

        pins.insert(
            height,
            ServingPin {
                checkpoint,
                pinned_at: now,
                last_served: now,
            },
        );
    }

    /// Returns a pinned checkpoint for the given height, if the pin is still held AND
    /// still live.
    ///
    /// Used to keep serving a snapshot whose checkpoint pruning has already dropped from
    /// the registry. An expired pin is dropped rather than returned: handing one out
    /// would let a peer resurrect (and then indefinitely refresh) a checkpoint whose
    /// transfer was abandoned long ago.
    pub fn pinned_checkpoint(&self, height: u64) -> Option<Arc<Checkpoint>> {
        let now = Instant::now();
        let mut pins = self
            .serving_pins
            .write()
            .expect("serving pins lock poisoned");
        retain_live_pins(&mut pins, now);
        pins.get(&height).map(|pin| Arc::clone(&pin.checkpoint))
    }

    /// Releases every pin that has passed either of its deadlines.
    ///
    /// Called once per block so an abandoned transfer cannot hold a pruned checkpoint
    /// directory on disk forever while waiting for a chunk request that never comes, and
    /// so an over-long one is cut off even while it keeps requesting.
    pub fn release_expired_pins(&self) {
        let now = Instant::now();
        let mut pins = self
            .serving_pins
            .write()
            .expect("serving pins lock poisoned");
        retain_live_pins(&mut pins, now);
    }

    /// Number of checkpoints currently pinned for serving.
    #[cfg(test)]
    pub fn pinned_count(&self) -> usize {
        self.serving_pins
            .read()
            .expect("serving pins lock poisoned")
            .len()
    }

    /// Test-only: the two deadlines of a pin, as `(pinned_at, last_served)`.
    #[cfg(test)]
    fn pin_instants(&self, height: u64) -> Option<(Instant, Instant)> {
        self.serving_pins
            .read()
            .expect("serving pins lock poisoned")
            .get(&height)
            .map(|pin| (pin.pinned_at, pin.last_served))
    }

    /// Test-only: backdates a pin's deadlines so expiry can be exercised without waiting.
    #[cfg(test)]
    fn backdate_pin(&self, height: u64, pinned_at: Option<Instant>, last_served: Option<Instant>) {
        let mut pins = self
            .serving_pins
            .write()
            .expect("serving pins lock poisoned");
        if let Some(pin) = pins.get_mut(&height) {
            if let Some(pinned_at) = pinned_at {
                pin.pinned_at = pinned_at;
            }
            if let Some(last_served) = last_served {
                pin.last_served = last_served;
            }
        }
    }
}

fn retain_live_pins(pins: &mut BTreeMap<u64, ServingPin>, now: Instant) {
    pins.retain(|_, pin| pin.is_live(now));
}

#[cfg(test)]
mod tests {
    use super::*;
    use drive::grovedb::GroveDb;

    /// Each checkpoint needs its own directory: rocksdb takes an exclusive lock on the
    /// one it opens.
    fn checkpoint_in(dir: &tempfile::TempDir, height: u64) -> Arc<Checkpoint> {
        let path = dir.path().join(height.to_string());
        let grove_db = GroveDb::open(&path).expect("should open grovedb");
        Arc::new(Checkpoint::new(grove_db, path))
    }

    /// An abandoned transfer stops making chunk requests, so the pin must expire on its
    /// own — both when swept from the block path and when a later request tries to read
    /// it (otherwise a peer could resurrect and then indefinitely refresh a checkpoint
    /// pruning already dropped).
    #[test]
    fn expired_pins_are_released_without_any_further_chunk_request() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let manager = SnapshotManager::new();
        manager.pin_for_serving(10, checkpoint_in(&dir, 10), max_serving_pins(3));
        assert!(manager.pinned_checkpoint(10).is_some());

        let Some(long_ago) = Instant::now().checked_sub(SERVING_PIN_INACTIVITY_TTL * 2) else {
            // Monotonic clock too young to backdate; nothing to assert on this platform.
            return;
        };
        manager.backdate_pin(10, None, Some(long_ago));

        assert!(
            manager.pinned_checkpoint(10).is_none(),
            "an expired pin must not be handed out",
        );

        manager.pin_for_serving(11, checkpoint_in(&dir, 11), max_serving_pins(3));
        manager.backdate_pin(11, None, Some(long_ago));
        manager.release_expired_pins();
        assert_eq!(
            manager.pinned_count(),
            0,
            "the block-driven sweep must release expired pins with no peer activity",
        );
    }

    /// The inactivity TTL is refreshable, so on its own it lets a peer hold a checkpoint
    /// forever by touching it periodically. The absolute lifetime is not refreshable and
    /// must cut such a pin off even while requests keep arriving.
    #[test]
    fn constant_touching_cannot_extend_a_pin_past_its_absolute_lifetime() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let manager = SnapshotManager::new();
        let checkpoint = checkpoint_in(&dir, 10);
        manager.pin_for_serving(10, Arc::clone(&checkpoint), max_serving_pins(3));
        let (pinned_at, _) = manager.pin_instants(10).expect("pin must exist");

        // Continued activity refreshes the idle deadline but must NOT reset `pinned_at`,
        // or the absolute deadline could be pushed out forever.
        manager.pin_for_serving(10, Arc::clone(&checkpoint), max_serving_pins(3));
        let (pinned_at_after_touch, last_served) =
            manager.pin_instants(10).expect("pin must exist");
        assert_eq!(
            pinned_at, pinned_at_after_touch,
            "serving another chunk must not extend the absolute lifetime",
        );
        assert!(last_served >= pinned_at);

        // Once that deadline passes the pin goes, even though it was just touched. The
        // peer can only get it back while the checkpoint is still in the registry — a
        // pruned one is gone for good, which is the retention this bounds.
        let Some(long_ago) = Instant::now().checked_sub(SERVING_PIN_MAX_LIFETIME * 2) else {
            return;
        };
        manager.backdate_pin(10, Some(long_ago), None);
        assert!(
            manager.pinned_checkpoint(10).is_none(),
            "an over-long pin must expire despite continued activity",
        );
    }

    /// A peer that keeps touching new heights as pruning advances must not be able to
    /// hold an unbounded number of checkpoint directories back from deletion.
    #[test]
    fn serving_pins_are_capped() {
        let dir = tempfile::tempdir().expect("should create temp dir");
        let manager = SnapshotManager::new();

        // The cap tracks how many snapshots the node retains, plus slack for transfers
        // that started before their checkpoint aged out.
        let max_pins = max_serving_pins(3);
        assert_eq!(max_pins, 3 + SERVING_PIN_SLACK);

        for height in 0..(max_pins as u64 + 4) {
            manager.pin_for_serving(height, checkpoint_in(&dir, height), max_pins);
        }

        assert_eq!(manager.pinned_count(), max_pins);
        assert!(
            manager.pinned_checkpoint(0).is_none(),
            "the least recently served pin must be evicted first",
        );
        assert!(manager.pinned_checkpoint(max_pins as u64 + 3).is_some());
    }

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
