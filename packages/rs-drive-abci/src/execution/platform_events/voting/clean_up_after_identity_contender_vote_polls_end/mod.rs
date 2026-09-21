use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollEndOutcome;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up after identity contender vote polls whose phase ended: their end date entries,
    /// and for the ones that resolved, their contenders, their votes, the masternodes'
    /// references to those votes, and their prefunded balance, which goes to the processing
    /// pool.
    pub(in crate::execution) fn clean_up_after_identity_contender_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &IdentityContenderVotePoll,
            &TimestampMillis,
            &IdentityContenderVotePollEndOutcome,
        )>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_identity_contender_vote_polls_end
        {
            0 => self.clean_up_after_identity_contender_vote_polls_end_v0(
                block_info,
                vote_polls,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_identity_contender_vote_polls_end".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
