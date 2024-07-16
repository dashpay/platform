mod v0;

use crate::drive::Drive;
use crate::util::object_size_info::DocumentAndContractInfo;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a document for contract operations using bincode serialization
    ///
    /// # Parameters
    /// * `owned_document_info`: The document info to be updated.
    /// * `data_contract_id`: The identifier for the data contract.
    /// * `document_type_name`: The document type name.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn update_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .update
            .update_document_for_contract_operations
        {
            0 => self.update_document_for_contract_operations_v0(
                document_and_contract_info,
                block_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
