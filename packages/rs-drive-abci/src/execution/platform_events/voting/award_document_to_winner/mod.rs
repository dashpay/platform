use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Awards a document to the winner of a contest, and returns whether it did. Version 0
    /// always does, or returns the insert's error; from version 1 (protocol version 15) a winner whose id or unique
    /// index values a document already holds is not awarded.
    pub(in crate::execution) fn award_document_to_winner(
        &self,
        block_info: &BlockInfo,
        contender: FinalizedContender,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .award_document_to_winner
        {
            0 => {
                self.award_document_to_winner_v0(
                    block_info,
                    contender,
                    vote_poll,
                    transaction,
                    platform_version,
                )?;
                Ok(true)
            }
            1 => self.award_document_to_winner_v1(
                block_info,
                contender,
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "award_document_to_winner".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
