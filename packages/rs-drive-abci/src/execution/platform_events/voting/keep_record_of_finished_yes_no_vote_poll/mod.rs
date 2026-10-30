use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::{
    YesNoVotePollResult, YesNoVotePollStoredInfo,
};
use dpp::voting::vote_polls::yes_no_vote_poll::VotingPower;
use drive::grovedb::TransactionArg;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollState;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Decides a finished yes/no poll by the rule it opened with, kept in its stored info, from
    /// its tallies and the total voting power of the masternode list in the closing block, and
    /// writes the result into the stored info, which stays as the record of the decision after
    /// the votes are cleaned up.
    ///
    /// # Parameters
    /// - `block_info`: the block that closes the poll.
    /// - `stored_info`: the stored info of the poll that ended, started and not yet finished.
    /// - `tally`: its tallies.
    /// - `total_voting_power`: the masternode list's total voting power in the closing block, 0 for a fixed minimum.
    /// - `transaction`: the transaction to read and write in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The poll's result, as recorded.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) fn keep_record_of_finished_yes_no_vote_poll(
        &self,
        block_info: &BlockInfo,
        stored_info: YesNoVotePollStoredInfo,
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
            Some(0) => self.keep_record_of_finished_yes_no_vote_poll_v0(
                block_info,
                stored_info,
                tally,
                total_voting_power,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "keep_record_of_finished_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "keep_record_of_finished_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
