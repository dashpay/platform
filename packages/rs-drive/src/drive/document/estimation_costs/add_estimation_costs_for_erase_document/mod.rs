mod v0;

use std::collections::HashMap;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

impl Drive {
    /// Registers the layers an erase chunk touches.
    ///
    /// # Parameters
    /// * `document_id`: The document whose revisions are being removed.
    /// * `contract`: The contract that contains the document.
    /// * `document_type`: The type of the document, which must keep history.
    /// * `layers`: The estimated layer information the dry run builds.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn add_estimation_costs_for_erase_document(
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        layers: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .add_estimation_costs_for_erase_document
        {
            0 => Self::add_estimation_costs_for_erase_document_v0(
                document_id,
                contract,
                document_type,
                layers,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_erase_document".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
