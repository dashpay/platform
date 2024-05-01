use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Performs the operations to add a document to a contract.
    #[inline(always)]
    pub(super) fn add_document_for_contract_apply_and_add_to_operations_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: &BlockInfo,
        document_is_unique_for_document_type_in_batch: bool,
        stateful: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if stateful {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        if document_is_unique_for_document_type_in_batch {
            let batch_operations = self.add_document_for_contract_operations(
                document_and_contract_info,
                override_document,
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
        } else {
            let batch_operations = self.add_document_for_contract_operations(
                document_and_contract_info,
                override_document,
                block_info,
                &mut Some(drive_operations),
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
}
