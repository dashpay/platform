use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollState;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The tallies of a yes/no poll, read from its vote sum trees, with its stored info.
    pub(in crate::execution) fn tally_votes_for_yes_no_vote_poll(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollState, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .tally_votes_for_yes_no_vote_poll
        {
            0 => self.tally_votes_for_yes_no_vote_poll_v0(vote_poll, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "tally_votes_for_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
