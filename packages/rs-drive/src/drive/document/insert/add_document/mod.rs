mod v0;

use crate::drive::Drive;
use crate::util::object_size_info::OwnedDocumentInfo;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a document using bincode serialization
    ///
    /// # Parameters
    /// * `owned_document_info`: The document info to be added.
    /// * `data_contract_id`: The identifier for the data contract.
    /// * `document_type_name`: The document type name.
    /// * `override_document`: Whether to override the document.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `transaction`: The transaction argument.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn add_document(
        &self,
        owned_document_info: OwnedDocumentInfo,
        data_contract_id: Identifier,
        document_type_name: &str,
        override_document: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.document.insert.add_document {
            0 => self.add_document_v0(
                owned_document_info,
                data_contract_id,
                document_type_name,
                override_document,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
