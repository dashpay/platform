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
    /// Prepares the operations for deleting a document.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type_name`: The name of the document type.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_document_for_contract_with_named_type_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_with_named_type_operations
        {
            0 => self.delete_document_for_contract_with_named_type_operations_v0(
                document_id,
                contract,
                document_type_name,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
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
            1 => self.delete_document_for_contract_with_named_type_operations_v1(
                document_id,
                contract,
                document_type_name,
                &BlockInfo::default_with_time(block_time_ms),
                None,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_with_named_type_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Prepares a lifecycle-aware delete of a document named by its type, using
    /// the block and deleter that authored the state transition.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_document_for_contract_with_named_type_operations_with_lifecycle(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_with_named_type_operations
        {
            0 => self.delete_document_for_contract_with_named_type_operations_v0(
                document_id,
                contract,
                document_type_name,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            1 => self.delete_document_for_contract_with_named_type_operations_v1(
                document_id,
                contract,
                document_type_name,
                block_info,
                deleter_id,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_with_named_type_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
