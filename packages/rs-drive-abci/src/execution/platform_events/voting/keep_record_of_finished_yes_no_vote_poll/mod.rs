use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollResult;
use dpp::voting::vote_polls::yes_no_vote_poll::{VotingPower, YesNoVotePoll};
use drive::grovedb::TransactionArg;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollState;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Decides a finished yes/no poll from its tallies and the total voting power of the
    /// masternode list in the closing block, and writes the result into its stored info, which
    /// stays as the record of the decision after the votes are cleaned up.
    pub(in crate::execution) fn keep_record_of_finished_yes_no_vote_poll(
        &self,
        block_info: &BlockInfo,
        vote_poll: &YesNoVotePoll,
        tally: &YesNoVotePollState,
        total_voting_power: VotingPower,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollResult, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .keep_record_of_finished_yes_no_vote_poll
        {
            0 => self.keep_record_of_finished_yes_no_vote_poll_v0(
                block_info,
                vote_poll,
                tally,
                total_voting_power,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "keep_record_of_finished_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
