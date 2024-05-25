use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContender;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Keeps a record of the vote poll after it has finished
    pub(in crate::execution) fn keep_record_of_vote_poll(
        &self,
        block_info: &BlockInfo,
        contender: &FinalizedContender,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .keep_record_of_vote_poll
        {
            0 => self.keep_record_of_vote_poll_v0(
                block_info,
                contender,
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "keep_record_of_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
