use crate::error::Error;
use crate::execution::types::block_state_info;
use dpp::version::PlatformVersion;

use crate::platform_types::block_proposal;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Creates epoch info from the platform state and the block proposal
    pub(super) fn gather_epoch_info_v0(
        &self,
        block_proposal: &block_proposal::v0::BlockProposal,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<EpochInfoV0, Error> {
        // Start by getting information from the state
        let state = self.state.read().unwrap();

        let last_block_time_ms = state.last_committed_block_time_ms();

        // Init block execution context
        let block_state_info = block_state_info::v0::BlockStateInfoV0::from_block_proposal(
            block_proposal,
            last_block_time_ms,
        );

        // destructure the block proposal
        let block_proposal::v0::BlockProposal {
            height,
            block_time_ms,
            ..
        } = &block_proposal;
        let genesis_time_ms =
            self.get_genesis_time(*height, *block_time_ms, transaction, platform_version)?;

        EpochInfoV0::from_genesis_time_and_block_info(
            genesis_time_ms,
            &block_state_info,
            self.config.execution.epoch_time_length_s,
        )
    }
}
