use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn run_dao_platform_events_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Remove any votes that

        self.remove_votes_for_removed_masternodes(
            last_committed_platform_state,
            block_platform_state,
            transaction,
            platform_version,
        )?;

        // Check for any vote polls that might have ended

        self.check_for_ended_vote_polls(
            last_committed_platform_state,
            block_platform_state,
            block_info,
            transaction,
            platform_version,
        )?;

        Ok(())
    }
}
