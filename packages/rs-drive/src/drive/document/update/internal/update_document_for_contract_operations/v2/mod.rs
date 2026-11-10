use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Generation 1, except that an update naming a new owner rewrites the references of the
    /// indexes it leaves unchanged with that owner, so all of the document's bytes move.
    pub(in crate::drive::document::update) fn update_document_for_contract_operations_v2(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.update_document_for_contract_operations_v1_body(
            document_and_contract_info,
            block_info,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            true,
            transaction,
            platform_version,
        )
    }
}
