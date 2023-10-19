use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0OwnedGetters;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the drive cache at the end of finalize block. This does a few things like merging
    /// the data contract cache and the platform versions cache.
    ///
    ///
    /// # Arguments
    ///
    /// * `block_info` - Extended block information for the current block.
    /// * `transaction` - The transaction associated with the block.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If the state cache and quorums are successfully updated, it returns `Ok(())`.
    ///   If there is a problem with the update, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with updating the state cache
    /// and quorums or storing the ephemeral data.
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
