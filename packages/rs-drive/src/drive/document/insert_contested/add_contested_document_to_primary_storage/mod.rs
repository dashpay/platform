mod v0;

use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;

use dpp::version::PlatformVersion;

impl Drive {
    /// Adds a contested document to primary storage.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: Information about the document and contract.
    /// * `block_info`: The block info.
    /// * `insert_without_check`: Whether to insert the document without check.
    /// * `estimated_costs_only_with_layer_info`: Information about the estimated costs only with layer.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn add_contested_document_to_primary_storage(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        insert_without_check: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert_contested
            .add_contested_document_to_primary_storage
        {
            0 => self.add_contested_document_to_primary_storage_0(
                document_and_contract_info,
                insert_without_check,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contested_document_to_primary_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
