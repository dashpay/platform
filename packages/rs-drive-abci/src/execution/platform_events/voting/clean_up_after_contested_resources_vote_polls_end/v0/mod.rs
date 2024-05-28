use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_contested_resources_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: &[ContestedDocumentResourceVotePoll],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];
        self.drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                vote_polls,
                block_info.time_ms,
                &mut operations,
                transaction,
                platform_version,
            )?;
        //todo()
        Ok(())
    }
}
