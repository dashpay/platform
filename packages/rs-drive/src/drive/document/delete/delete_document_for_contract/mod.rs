mod v0;

use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::data_contract::DataContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Deletes a document and returns the associated fee.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type_name`: The name of the document type.
    /// * `owner_id`: The owner ID of the document.
    /// * `block_info`: The block information.
    /// * `apply`: A flag indicating if the operation should be applied.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn delete_document_for_contract(
        &self,
        document_id: [u8; 32],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version.methods.document.delete.delete_document_for_contract {
            0 => self.delete_document_for_contract_v0(
                document_id,
                contract,
                document_type_name,
                owner_id,
                block_info,
                apply,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}