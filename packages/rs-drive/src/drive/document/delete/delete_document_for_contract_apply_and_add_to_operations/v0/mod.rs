use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::version::drive_versions::DriveVersion;
use crate::contract::Contract;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Deletes a document.
    pub(super) fn delete_document_for_contract_apply_and_add_to_operations_v0(
        &self,
        document_id: [u8; 32],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        mut estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.delete_document_for_contract_with_named_type_operations(
            document_id,
            contract,
            document_type_name,
            None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            drive_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            drive_version,
        )
    }
}