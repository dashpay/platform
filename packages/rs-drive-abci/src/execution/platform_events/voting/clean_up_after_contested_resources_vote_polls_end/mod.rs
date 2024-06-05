use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    pub(in crate::execution) fn clean_up_after_contested_resources_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        vote_polls: &[&ContestedDocumentResourceVotePoll],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_contested_resources_vote_poll_end
        {
            0 => self.clean_up_after_contested_resources_vote_polls_end_v0(
                block_info,
                vote_polls,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_contested_resources_vote_polls_end".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
