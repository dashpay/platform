use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up after yes/no polls that ended, each given with its end date and its voters:
    /// their entries in the end date index, their vote trees, the voters' entries in the
    /// identity votes index, and what is left of the prefunded balance that paid for the votes,
    /// which goes to the processing pool. The stored info stays as the record of the decision.
    ///
    /// `leftover_end_date_entries` are the end date entries, by poll id, of polls an earlier
    /// entry already closed: only the entries go.
    pub(in crate::execution) fn clean_up_after_yes_no_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        vote_polls: &[(YesNoVotePoll, TimestampMillis, Vec<Identifier>)],
        leftover_end_date_entries: &[(Identifier, TimestampMillis)],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_yes_no_vote_polls_end
        {
            0 => self.clean_up_after_yes_no_vote_polls_end_v0(
                block_info,
                vote_polls,
                leftover_end_date_entries,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_yes_no_vote_polls_end".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
