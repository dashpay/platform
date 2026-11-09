use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Deletes a document and returns the associated fee.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_v0(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = (!apply).then(EstimatedLayers::new);
        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            estimated_costs_only_with_layer_info,
            block_info.time_ms,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = self.calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            platform_version,
        )?;
        Ok(fees)
    }
}
