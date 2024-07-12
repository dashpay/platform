mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;

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
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract
        {
            0 => self.delete_document_for_contract_v0(
                document_id,
                contract,
                document_type_name,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
