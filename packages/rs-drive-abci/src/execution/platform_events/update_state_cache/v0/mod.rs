use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::ExtendedBlockInfo;
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
    pub fn update_state_cache_v0(
        &self,
        block_info: ExtendedBlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let mut block_execution_context = self.block_execution_context.write().unwrap();

        let block_execution_context = block_execution_context.take().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("there should be a block execution context"),
        ))?;

        let mut state_cache = self.state.write().unwrap();

        *state_cache = block_execution_context.block_platform_state;

        if let Some(next_validator_set_quorum_hash) =
            state_cache.next_validator_set_quorum_hash.take()
        {
            state_cache.current_validator_set_quorum_hash = next_validator_set_quorum_hash;
        }

        state_cache.last_committed_block_info = Some(block_info);

        state_cache.initialization_information = None;

        // Persist ephemeral data
        self.store_ephemeral_state_v0(&state_cache, transaction)?;

        Ok(())
    }
}
