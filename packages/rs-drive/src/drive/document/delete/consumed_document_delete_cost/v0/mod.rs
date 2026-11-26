use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// The delete a consume makes, `ForceDeleteDocument`'s, built in estimation mode for a
    /// document of `document_type`, which stands in by its type's estimated size, then priced
    /// as fee validation prices a batch's operations.
    #[inline(always)]
    pub(super) fn consumed_document_delete_cost_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut layers: Option<EstimatedLayers> = Some(EstimatedLayers::new());
        let batch = self.force_delete_document_for_contract_operations(
            Identifier::default(),
            contract,
            document_type,
            block_info,
            None,
            None,
            &mut layers,
            None,
            platform_version,
        )?;
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            layers,
            None,
            batch,
            &mut operations,
            &platform_version.drive,
        )?;
        self.calculate_fee(None, Some(operations), &block_info.epoch, platform_version)
    }
}
