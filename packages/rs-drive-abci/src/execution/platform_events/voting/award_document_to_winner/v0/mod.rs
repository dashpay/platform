use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
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
        // The contest names a contender by its owner's id, which is who the document goes
        // to. In a vote among documents (protocol version 15) it names the contender by its
        // document id instead, and the owner is the one the document itself names. No
        // contract registered before that version declares the resolution, so every other
        // contest awards its winner as it always did.
        let owner_id = if vote_poll
            .index()?
            .contested_index
            .as_ref()
            .is_some_and(|contested| contested.resolution.contenders_are_documents())
        {
            document.owner_id()
        } else {
            identity_id
        };

        let owned_document_info = OwnedDocumentInfo {
            document_info: DocumentAndSerialization((document, serialized_document, None)),
            owner_id: Some(owner_id.to_buffer()),
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
        )?;
        Ok(())
    }
}
