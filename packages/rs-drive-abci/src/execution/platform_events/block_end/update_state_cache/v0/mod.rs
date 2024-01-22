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
    /// Updates the state cache at the end of finalize block. This is done by overriding the current
    /// platform state cache with the block execution state cache.
    ///
    /// This function takes an `ExtendedBlockInfo` and a `Transaction` as input and updates the
    /// state cache and quorums based on the given block information. It handles protocol version
    /// updates and sets the current and next epoch protocol versions.
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
    pub(super) fn update_state_cache_v0(
        &self,
        extended_block_info: ExtendedBlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut block_execution_context = self.block_execution_context.write().unwrap();

        let block_execution_context = block_execution_context.take().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("there should be a block execution context"),
        ))?;

        let mut state_cache = self.state.write().unwrap();

        *state_cache = block_execution_context.block_platform_state_owned();

        if let Some(next_validator_set_quorum_hash) =
            state_cache.take_next_validator_set_quorum_hash()
        {
            state_cache.set_current_validator_set_quorum_hash(next_validator_set_quorum_hash);
        }

        state_cache.set_last_committed_block_info(Some(extended_block_info));

        state_cache.set_genesis_block_info(None);

        //todo: verify this with an update
        let version = PlatformVersion::get(platform_version.protocol_version)?;

        PlatformVersion::set_current(version);

        // Persist state cache
        self.store_platform_state(&state_cache, Some(transaction), platform_version)?;

        Ok(())
    }
}
