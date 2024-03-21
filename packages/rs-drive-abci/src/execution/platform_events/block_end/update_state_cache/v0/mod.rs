use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive::grovedb::Transaction;
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

        //todo: verify this with an update
        let version = PlatformVersion::get(platform_version.protocol_version)?;

        PlatformVersion::set_current(version);

        // Persist block state

        self.store_platform_state(&block_platform_state, Some(transaction), platform_version)?;

        self.state.store(Arc::new(block_platform_state));

        Ok(())
    }
}
