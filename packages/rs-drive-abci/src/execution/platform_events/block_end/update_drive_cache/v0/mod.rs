use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the drive cache at the end of finalize block. This does a few things like merging
    /// the data contract cache and the platform versions cache.
    ///
    pub(super) fn update_drive_cache_v0(&self) {
        let mut drive_cache = self.drive.cache.write().unwrap();

        // Update global cache with updated contracts
        drive_cache.cached_contracts.merge_block_cache();
        // This is unnecessary since we clear block cache before every proposal execution
        drive_cache.cached_contracts.clear_block_cache();

        drive_cache.protocol_versions_counter.merge_block_cache()
    }
}
