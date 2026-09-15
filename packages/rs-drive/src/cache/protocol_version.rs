use crate::drive::Drive;
use crate::error::cache::CacheError;
use crate::error::Error;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::TransactionArg;
use nohash_hasher::IntMap;
use platform_version::version::drive_versions::DriveVersion;

/// ProtocolVersion cache that handles both global and block data
#[derive(Default)]
pub struct ProtocolVersionsCache {
    /// The current global cache for protocol versions
    // TODO: If we persist this in the state and it should be loaded for correct
    //  use then it's not actually the cache. Move out of cache because it's confusing
    pub global_cache: IntMap<ProtocolVersion, u64>,
    block_cache: IntMap<ProtocolVersion, u64>,
    loaded: bool,
    is_global_cache_blocked: bool,
}

#[cfg(feature = "server")]
impl ProtocolVersionsCache {
    /// Create a new ProtocolVersionsCache instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the protocol versions cache from disk if needed
    pub fn load_if_needed(
        &mut self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if !self.loaded {
            self.global_cache = drive.fetch_versions_with_counter(transaction, drive_version)?;
            self.loaded = true;
        };
        Ok(())
    }

    /// Whether the global cache has been loaded from Drive.
    ///
    /// Until it has, the global cache is empty rather than a mirror of the persisted counters,
    /// and any tally over it would count zero votes.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Sets the protocol version to the block cache
    pub fn set_block_cache_version_count(&mut self, version: ProtocolVersion, count: u64) {
        self.block_cache.insert(version, count);
    }

    /// Tries to get a version from block cache if present
    /// if block cache doesn't have the version set
    /// then it tries get the version from global cache
    pub fn get(&self, version: &ProtocolVersion) -> Result<Option<&u64>, Error> {
        if self.is_global_cache_blocked {
            return Err(Error::Cache(CacheError::GlobalCacheIsBlocked));
        }

        let counter = if let Some(count) = self.block_cache.get(version) {
            Some(count)
        } else {
            self.global_cache.get(version)
        };

        Ok(counter)
    }

    /// Disable the global cache to do not allow get counters
    /// If global cache is blocked then [get] will return an error
    pub fn block_global_cache(&mut self) {
        self.is_global_cache_blocked = true;
    }

    /// Unblock the global cache
    /// This function enables the normal behaviour of [get] function
    pub fn unblock_global_cache(&mut self) {
        self.is_global_cache_blocked = false;
    }

    /// Merge block cache to global cache
    pub fn merge_block_cache(&mut self) {
        self.global_cache.extend(self.block_cache.drain());
    }

    /// Clears the global cache
    pub fn clear_global_cache(&mut self) {
        self.global_cache.clear();
    }

    /// Clear block cache
    pub fn clear_block_cache(&mut self) {
        self.block_cache.clear()
    }

    /// Collect versions passing threshold
    pub fn versions_passing_threshold(&self, required_upgraded_hpmns: u64) -> Vec<ProtocolVersion> {
        let mut cache = self.global_cache.clone();

        cache.extend(self.block_cache.iter());
        cache
            .into_iter()
            .filter_map(|(protocol_version, count)| {
                if count >= required_upgraded_hpmns {
                    Some(protocol_version)
                } else {
                    None
                }
            })
            .collect::<Vec<ProtocolVersion>>()
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use dpp::version::PlatformVersion;
    use std::path::Path;
    use tempfile::TempDir;

    /// Opens a Drive at `path`, commits votes for `protocol_version + 1` from three validators
    /// and for `protocol_version + 2` from a fourth, stores the protocol version the way a
    /// committed block does, and returns the counters as a node that never restarted holds them
    /// once the block is finalized.
    fn persist_votes(
        path: &Path,
        platform_version: &PlatformVersion,
    ) -> IntMap<ProtocolVersion, u64> {
        let (drive, _) =
            Drive::open(path, Some(DriveConfig::default())).expect("expected to open Drive");
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create the initial state structure");

        let next_version = platform_version.protocol_version + 1;
        let transaction = drive.grove.start_transaction();
        for validator in 1..=3u8 {
            drive
                .update_validator_proposed_app_version(
                    [validator; 32],
                    next_version,
                    Some(&transaction),
                    &platform_version.drive,
                )
                .expect("expected to record the vote");
        }
        drive
            .update_validator_proposed_app_version(
                [4; 32],
                next_version + 1,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("expected to record the vote");
        drive
            .store_current_protocol_version(platform_version.protocol_version, Some(&transaction))
            .expect("expected to store the protocol version");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the votes");

        let mut counter = drive.cache.protocol_versions_counter.write();
        counter.merge_block_cache();
        counter.global_cache.clone()
    }

    #[test]
    fn should_start_a_fresh_drive_with_nothing_loaded() {
        let tempdir = TempDir::new().expect("expected a temporary directory");

        let (drive, protocol_version) = Drive::open(tempdir.path(), Some(DriveConfig::default()))
            .expect("expected to open Drive");

        assert!(protocol_version.is_none());
        assert!(!drive.cache.protocol_versions_counter.read().is_loaded());
    }

    #[test]
    fn should_reopen_a_drive_with_the_persisted_votes_loaded() {
        let tempdir = TempDir::new().expect("expected a temporary directory");
        let platform_version = PlatformVersion::latest();
        let next_version = platform_version.protocol_version + 1;

        let warm_counters = persist_votes(tempdir.path(), platform_version);
        assert_eq!(warm_counters.get(&next_version), Some(&3));

        let (reopened, stored_version) = Drive::open(tempdir.path(), Some(DriveConfig::default()))
            .expect("expected to reopen Drive");
        assert_eq!(
            stored_version.map(|version| version.protocol_version),
            Some(platform_version.protocol_version)
        );

        let counter = reopened.cache.protocol_versions_counter.read();
        assert!(
            counter.is_loaded(),
            "a reopened Drive must present the persisted votes without waiting for the first vote of a block"
        );
        assert_eq!(counter.global_cache, warm_counters);
        assert_eq!(counter.versions_passing_threshold(3), vec![next_version]);
    }

    #[test]
    fn should_load_the_persisted_votes_into_a_cold_cache_through_the_transaction() {
        let tempdir = TempDir::new().expect("expected a temporary directory");
        let platform_version = PlatformVersion::latest();
        let next_version = platform_version.protocol_version + 1;

        let warm_counters = persist_votes(tempdir.path(), platform_version);
        let (reopened, _) = Drive::open(tempdir.path(), Some(DriveConfig::default()))
            .expect("expected to reopen Drive");

        // A cache that was never loaded, as after `drop_cache`, tallies zero votes.
        let mut cold_cache = ProtocolVersionsCache::new();
        assert!(!cold_cache.is_loaded());
        assert!(cold_cache.versions_passing_threshold(1).is_empty());

        let transaction = reopened.grove.start_transaction();
        cold_cache
            .load_if_needed(&reopened, Some(&transaction), &platform_version.drive)
            .expect("expected to load the persisted votes");

        assert!(cold_cache.is_loaded());
        assert_eq!(cold_cache.global_cache, warm_counters);
        assert_eq!(cold_cache.versions_passing_threshold(3), vec![next_version]);
    }
}
