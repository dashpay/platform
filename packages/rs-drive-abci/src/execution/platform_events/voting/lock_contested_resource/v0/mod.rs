use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Locks a contested resource after the lock vote wins
    #[inline(always)]
    pub(super) fn lock_contested_resource_v0(
        &self,
        block_info: &BlockInfo,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We need to lock the vote_poll

        self.drive
            .add_lock_for_contested_document_resource_vote_poll(
                vote_poll,
                transaction,
                platform_version,
            )?;

        Ok(())
    }
}
