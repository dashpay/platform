use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::util::object_size_info::DocumentInfo::DocumentAndSerialization;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Awards a document to the winner of a contest
    #[inline(always)]
    pub(super) fn award_document_to_winner_v0(
        &self,
        block_info: &BlockInfo,
        contender: FinalizedContender,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let FinalizedContender {
            identity_id,
            document,
            serialized_document,
            ..
        } = contender;
        // Let's start by getting the identity

        let owned_document_info = OwnedDocumentInfo {
            document_info: DocumentAndSerialization((document, serialized_document, None)),
            owner_id: Some(identity_id.to_buffer()),
        };

        // Let's insert the document into the state
        self.drive.add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info,
                contract: vote_poll.contract.as_ref(),
                document_type: vote_poll.document_type()?,
            },
            false,
            *block_info,
            true,
            transaction,
            platform_version,
            None,
        )?;
        Ok(())
    }
}
