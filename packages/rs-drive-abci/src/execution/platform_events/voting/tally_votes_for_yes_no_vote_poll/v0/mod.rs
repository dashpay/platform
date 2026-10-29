use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::yes_no_vote_poll_state_query::{
    YesNoVotePollState, YesNoVotePollStateDriveQuery,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn tally_votes_for_yes_no_vote_poll_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollState, Error> {
        let query = YesNoVotePollStateDriveQuery {
            vote_poll: vote_poll.clone(),
        };
        Ok(query.execute_no_proof(&self.drive, transaction, &mut vec![], platform_version)?)
    }
}
