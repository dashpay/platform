mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Deletes a document.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type_name`: The name of the document type.
    /// * `owner_id`: The owner ID of the document.
    /// * `estimated_costs_only_with_layer_info`: An optional hashmap with layer information for estimated costs.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: A mutable vector of low level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_document_for_contract_apply_and_add_to_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_apply_and_add_to_operations
        {
            0 => self.delete_document_for_contract_apply_and_add_to_operations_v0(
                document_id,
                contract,
                document_type_name,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                drive_operations,
                platform_version,
            ),
            // This signature carries only the block time. Generation 1 records
            // a lifecycle entry for a keep-history document with the block and
            // deleter that authored the delete, which only the
            // `_with_lifecycle` entry point receives, so it serves the types
            // that keep no history and refuses the rest instead of recording a
            // fabricated block.
            1 if contract
                .document_type_for_name(document_type_name)?
                .documents_keep_history() =>
            {
                Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "deleting a keep-history document needs the block and deleter that authored it",
                )))
            }
            1 => self.delete_document_for_contract_apply_and_add_to_operations_v1(
                document_id,
                contract,
                document_type_name,
                &BlockInfo::default_with_time(block_time_ms),
                None,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_apply_and_add_to_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Deletes a document and adds the operations to the given list, recording
    /// the lifecycle entry of a keep-history document with the block and deleter
    /// that authored the delete.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_document_for_contract_apply_and_add_to_operations_with_lifecycle(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_apply_and_add_to_operations
        {
            0 => self.delete_document_for_contract_apply_and_add_to_operations_v0(
                document_id,
                contract,
                document_type_name,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                drive_operations,
                platform_version,
            ),
            1 => self.delete_document_for_contract_apply_and_add_to_operations_v1(
                document_id,
                contract,
                document_type_name,
                block_info,
                deleter_id,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_apply_and_add_to_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
