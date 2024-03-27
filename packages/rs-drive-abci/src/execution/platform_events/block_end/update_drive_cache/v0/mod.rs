use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the drive cache at the end of finalize block. This does a few things like merging
    /// the data contract cache and the platform versions cache.
    ///
    #[inline(always)]
    pub(super) fn update_drive_cache_v0(&self) {
        // Update global cache with updated contracts
        self.drive
            .cache
            .data_contracts
            .merge_and_clear_block_cache();

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();

        protocol_versions_counter.merge_block_cache()
    }
}
