use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::batch::storage_sponsor_of_document_type;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generation 0, except that the update hands the document's storage over as an update in
    /// a batch of drive operations does (protocol version 15): the stored bytes go to the
    /// owner the written flags name, and an owner change involving the contract's owner, on a
    /// document type offering gas sponsorship, is a full handover.
    pub(in crate::drive::document::update) fn update_document_for_contract_apply_and_add_to_operations_v1(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: Option<EstimatedLayers>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let full_handover_owners = storage_sponsor_of_document_type(
            document_and_contract_info.contract,
            document_and_contract_info.document_type,
        )
        .into_iter()
        .collect();
        self.update_document_for_contract_apply_and_add_to_operations_v0_body(
            document_and_contract_info,
            block_info,
            estimated_costs_only_with_layer_info,
            Some(full_handover_owners),
            transaction,
            drive_operations,
            platform_version,
        )
    }
}
