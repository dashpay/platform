use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::{
    YesNoVotePollResult, YesNoVotePollStoredInfo,
};
use dpp::voting::vote_polls::yes_no_vote_poll::{VotingPower, YesNoVotePoll};
use drive::grovedb::TransactionArg;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollState;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn keep_record_of_finished_yes_no_vote_poll_v0(
        &self,
        block_info: &BlockInfo,
        vote_poll: &YesNoVotePoll,
        mut stored_info: YesNoVotePollStoredInfo,
        tally: &YesNoVotePollState,
        total_voting_power: VotingPower,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollResult, Error> {
        let required_voting_power = vote_poll.required_voting_power(total_voting_power);
        let passed = vote_poll.passes(
            tally.yes_voting_power,
            tally.no_voting_power,
            total_voting_power,
        );
        let result = stored_info.finalize_vote_poll(
            tally.yes_voting_power,
            tally.no_voting_power,
            tally.abstain_voting_power,
            required_voting_power,
            passed,
            *block_info,
        )?;
        self.drive.insert_stored_info_for_yes_no_vote_poll(
            vote_poll,
            stored_info,
            transaction,
            platform_version,
        )?;
        Ok(result)
    }
}
