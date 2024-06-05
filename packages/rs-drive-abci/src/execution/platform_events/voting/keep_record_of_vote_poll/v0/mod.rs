use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContender;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Keeps a record of the vote poll after it has finished
    #[inline(always)]
    pub(super) fn keep_record_of_vote_poll_v0(
        &self,
        block_info: &BlockInfo,
        contender: &FinalizedContender,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We want to store information about the vote poll in an efficient way
        Ok(())
    }
}
