use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

impl Drive {
    /// The delete a consume makes from protocol version 15, `ForceDeleteReadDocument`'s, from
    /// what the create's lookup read, without reading the document again: built in estimation
    /// mode for a document of `document_type`, which stands in by its type's estimated size, then
    /// priced as fee validation prices a batch's operations.
    #[inline(always)]
    pub(super) fn consumed_document_delete_cost_v1(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut layers: Option<EstimatedLayers> = Some(EstimatedLayers::new());
        let batch = self.force_delete_read_document_for_contract_operations(
            Document::V0(DocumentV0::default()),
            contract,
            document_type,
            None,
            &mut layers,
            block_info.time_ms,
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
