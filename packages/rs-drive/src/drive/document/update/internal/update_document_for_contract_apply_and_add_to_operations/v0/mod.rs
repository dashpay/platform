use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a document.
    pub(in crate::drive::document::update) fn update_document_for_contract_apply_and_add_to_operations_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        mut estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.update_document_for_contract_operations(
            document_and_contract_info,
            block_info,
            &mut None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }
}
