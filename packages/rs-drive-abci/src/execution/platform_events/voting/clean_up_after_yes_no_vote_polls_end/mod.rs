use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up after yes/no polls that ended: their votes and vote trees, the voters'
    /// entries in the identity votes index, and what is left of the prefunded balance that
    /// paid for the votes, which goes to the processing pool. The end date index was already
    /// cleared by the caller. The stored info stays as the record of the decision.
    pub(in crate::execution) fn clean_up_after_yes_no_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &YesNoVotePoll,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )>,
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
