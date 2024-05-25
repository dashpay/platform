use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContender;
use drive::query::VotePollsByEndDateDriveQuery;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_vote_poll_end_v0(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
    }
}
