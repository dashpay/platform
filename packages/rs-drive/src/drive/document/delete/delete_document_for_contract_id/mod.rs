mod v0;

use grovedb::TransactionArg;

use dpp::block::block_info::BlockInfo;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;

impl Drive {
    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract_id`: The ID of the contract that contains the document.
    /// * `document_type_name`: The name of the document type.
    /// * `owner_id`: The owner ID of the document.
    /// * `block_info`: The block information.
    /// * `apply`: Boolean flag indicating if the operation should be applied.
    /// * `transaction`: The transaction argument.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn delete_document_for_contract_id(
        &self,
        document_id: Identifier,
        contract_id: Identifier,
        document_type_name: &str,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_id
        {
            0 => self.delete_document_for_contract_id_v0(
                document_id,
                contract_id,
                document_type_name,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
