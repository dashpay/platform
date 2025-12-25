use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;
use std::sync::Arc;

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
    /// * `checkpointed` - Whether a checkpoint was created for this block.
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
    #[inline(always)]
    pub(super) fn update_state_cache_v0(
        &self,
        extended_block_info: ExtendedBlockInfo,
        mut block_platform_state: PlatformState,
        checkpointed: bool,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Update block state and store it in shared lock

        if let Some(next_validator_set_quorum_hash) =
            block_platform_state.take_next_validator_set_quorum_hash()
        {
            block_platform_state
                .set_current_validator_set_quorum_hash(next_validator_set_quorum_hash);
        }

        block_platform_state.set_last_committed_block_info(Some(extended_block_info));

        block_platform_state.set_genesis_block_info(None);

        // Persist block state

        self.store_platform_state(&block_platform_state, Some(transaction), platform_version)?;

        let block_platform_state = Arc::new(block_platform_state);

        // If a checkpoint was created, store the platform state for that checkpoint
        if checkpointed {
            let block_height = block_platform_state.last_committed_block_height();
            let checkpoints = self.drive.checkpoints.load();
            let keep_n = platform_version.drive_abci.checkpoints.num_checkpoints as usize;
            let max_old_to_keep = keep_n.saturating_sub(1);
            let to_skip = checkpoints.len().saturating_sub(max_old_to_keep);

            let current_checkpoint_states = self.checkpoint_platform_states.load();
            let mut new_checkpoint_states = BTreeMap::new();

            // Add the current platform state for this new checkpoint
            new_checkpoint_states.insert(block_height, block_platform_state.clone());

            // Copy only the states for checkpoints we're keeping
            for (height, state) in current_checkpoint_states.iter().skip(to_skip) {
                new_checkpoint_states.insert(*height, state.clone());
            }

            // Atomically swap in the new checkpoint states map
            self.checkpoint_platform_states
                .store(Arc::new(new_checkpoint_states));
        }

        self.state.store(block_platform_state);

        Ok(())
    }
}
