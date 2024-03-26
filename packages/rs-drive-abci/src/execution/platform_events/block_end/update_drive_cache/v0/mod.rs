use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
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
    pub(super) fn update_drive_cache_v0(&self, block_execution_context: &BlockExecutionContext) {
        // Update global cache with updated contracts form this block
        self.drive
            .cache
            .data_contracts
            .merge_and_clear_block_cache();

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();

        if block_execution_context
            .epoch_info()
            .is_epoch_change_but_not_genesis()
        {
            // Clear previously proposed versions since we started a new epoch
            protocol_versions_counter.clear();
        } else {
            // Update proposed versions with new proposal from the current block
            protocol_versions_counter.merge_block_cache()
        }
    }
}
