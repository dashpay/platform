use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContestedDocumentVotePollDriveQueryExecutionResult;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Tally the votes for a contested resource vote poll
    pub(in crate::execution) fn tally_votes_for_contested_document_resource_vote_poll(
        &self,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FinalizedContestedDocumentVotePollDriveQueryExecutionResult, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .tally_votes_for_contested_document_resource_vote_poll
        {
            0 => self.tally_votes_for_contested_document_resource_vote_poll_v0(
                contested_document_resource_vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "tally_votes_for_contested_resource_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
