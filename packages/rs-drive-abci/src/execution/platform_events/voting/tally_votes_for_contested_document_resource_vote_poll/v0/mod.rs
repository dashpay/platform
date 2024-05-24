use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    FinalizedContenderWithSerializedDocument,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    pub(super) fn tally_votes_for_contested_document_resource_vote_poll_v0(
        &self,
        contested_document_resource_vote_poll: &ContestedDocumentResourceVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<FinalizedContenderWithSerializedDocument>, Error> {
        //todo: try to figure out how to do this without a clone
        //we start by only requesting the vote tally because we don't want to load all the documents
        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll: contested_document_resource_vote_poll.clone(),
            result_type: ContestedDocumentVotePollDriveQueryResultType::Documents,
            offset: None,
            limit: Some(
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_contenders_to_consider,
            ),
            start_at: None,
            order_ascending: true,
        };

        query
            .execute_no_proof(&self.drive, transaction, &mut vec![], platform_version)?
            .contenders
            .into_iter()
            .map(|contender| {
                let finalized: FinalizedContenderWithSerializedDocument = contender.try_into()?;
                Ok(finalized)
            })
            .collect::<Result<Vec<_>, Error>>()
    }
}
