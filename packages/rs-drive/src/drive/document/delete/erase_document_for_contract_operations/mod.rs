mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Prepares the operations for removing a bounded chunk of the retained
    /// revisions of a document that has already been deleted.
    ///
    /// # Parameters
    /// * `document_id`: The document whose revisions are being removed.
    /// * `contract`: The contract that contains the document.
    /// * `document_type`: The type of the document, which must keep history.
    /// * `block_info`: The block this erase belongs to.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub fn erase_document_for_contract_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
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
            .delete
            .erase_document_for_contract_operations
        {
            0 => self.erase_document_for_contract_operations_v0(
                document_id,
                contract,
                document_type,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "erase_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
