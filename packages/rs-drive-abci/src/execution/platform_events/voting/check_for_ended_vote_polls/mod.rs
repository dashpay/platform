use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    pub(in crate::execution) fn check_for_ended_vote_polls(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .check_for_ended_vote_polls
        {
            0 => self.check_for_ended_vote_polls_v0(
                block_platform_state,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_for_ended_vote_polls".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
