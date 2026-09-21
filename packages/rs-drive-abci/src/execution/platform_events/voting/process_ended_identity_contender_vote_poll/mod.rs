use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollEndOutcome;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Ends the phase of an identity contender vote poll whose end date arrived: the join phase
    /// resolves the poll with at most one contender or starts the vote phase, the vote phase
    /// resolves it by plurality. Returns what the clean-up that follows acts on.
    pub(in crate::execution) fn process_ended_identity_contender_vote_poll(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityContenderVotePollEndOutcome, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .process_ended_identity_contender_vote_poll
        {
            0 => self.process_ended_identity_contender_vote_poll_v0(
                block_platform_state,
                block_info,
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "process_ended_identity_contender_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
