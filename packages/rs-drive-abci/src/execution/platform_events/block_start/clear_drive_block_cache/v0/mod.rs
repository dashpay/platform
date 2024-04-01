use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Clears the drive cache at the start of block processing. This does a few things like clearing
    /// the block data contract cache and the block platform versions cache.
    ///
    #[inline(always)]
    pub(super) fn clear_drive_block_cache_v0(&self) {
        self.drive.cache.data_contracts.clear_block_cache();

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();

        protocol_versions_counter.clear_block_cache()
    }
}
