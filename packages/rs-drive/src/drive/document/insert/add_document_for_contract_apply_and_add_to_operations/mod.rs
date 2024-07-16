mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Performs the operations to add a document to a contract.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: The document and contract info.
    /// * `params`: In v0 the params should be:
    /// *   `override_document`: Whether to override the document.
    /// *   `block_info`: The block info.
    /// *   `document_is_unique_for_document_type_in_batch`: Whether the document is unique for the document type in batch.
    /// *   `stateful`: Whether the operation is stateful.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn add_document_for_contract_apply_and_add_to_operations(
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
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_for_contract_apply_and_add_to_operations
        {
            0 => self.add_document_for_contract_apply_and_add_to_operations_v0(
                document_and_contract_info,
                override_document,
                block_info,
                document_is_unique_for_document_type_in_batch,
                stateful,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_for_contract_apply_and_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
