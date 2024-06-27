use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Keeps a record of the vote poll after it has finished
    pub(in crate::execution) fn keep_record_of_finished_contested_resource_vote_poll(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        contender_votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        winner_info: ContestedDocumentVotePollWinnerInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .keep_record_of_finished_contested_resource_vote_poll
        {
            0 => self.keep_record_of_finished_contested_resource_vote_poll_v0(
                block_platform_state,
                block_info,
                vote_poll,
                contender_votes,
                winner_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "keep_record_of_finished_contested_resource_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
