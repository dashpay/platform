use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Generation 1, except that a poll's end date entries name whoever the document's storage
    /// flags name.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contested_document_for_contract_operations_v2(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        insert_without_check: bool,
        block_info: &BlockInfo,
        also_insert_vote_poll_stored_info: Option<ContestedDocumentVotePollStoredInfo>,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.add_contested_document_for_contract_operations_v1_body(
            document_and_contract_info,
            contested_document_resource_vote_poll,
            insert_without_check,
            block_info,
            also_insert_vote_poll_stored_info,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            true,
            transaction,
            platform_version,
        )
    }
}
