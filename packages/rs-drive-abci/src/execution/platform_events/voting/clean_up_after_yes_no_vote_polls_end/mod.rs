use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;

/// A yes/no poll the block closed, as its clean-up needs it.
pub(in crate::execution) struct FinishedYesNoVotePoll {
    /// The poll's id, which keys its vote trees and its voters' entries in the identity votes
    /// index.
    pub poll_id: Identifier,
    /// The id of the prefunded balance its votes were paid from
    /// (`YesNoVotePoll::specialized_balance_id`).
    pub prefunded_balance_id: Identifier,
    /// The masternodes that voted in it.
    pub voters: Vec<Identifier>,
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up after yes/no polls that ended: their entries in the end date index, the vote
    /// trees of the polls this block closed, their voters' entries in the identity votes index,
    /// and what is left of the prefunded balances that paid for the votes, which goes to the
    /// processing pool. The stored info stays as the record of the decision.
    ///
    /// `end_date_entries` lists every due yes/no entry, including those of polls an earlier
    /// entry already closed, of which only the entry goes.
    ///
    /// # Parameters
    /// - `block_info`: the block that closes the polls, whose epoch gets what is left of their funds.
    /// - `end_date_entries`: the due yes/no entries of the end date index, by poll id.
    /// - `finished_vote_polls`: the polls this block closed.
    /// - `transaction`: the transaction to read and write in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// `Ok(())` once the clean-up is applied.
    pub(in crate::execution) fn clean_up_after_yes_no_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        end_date_entries: &[(Identifier, TimestampMillis)],
        finished_vote_polls: &[FinishedYesNoVotePoll],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_yes_no_vote_polls_end
        {
            Some(0) => self.clean_up_after_yes_no_vote_polls_end_v0(
                block_info,
                end_date_entries,
                finished_vote_polls,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_yes_no_vote_polls_end".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "clean_up_after_yes_no_vote_polls_end".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
