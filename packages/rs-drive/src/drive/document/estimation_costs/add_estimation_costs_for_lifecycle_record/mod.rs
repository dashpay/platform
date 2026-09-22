mod v0;

use std::collections::HashMap;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

impl Drive {
    /// Registers the layers a lifecycle record write or removal touches.
    ///
    /// The record is written by a delete of a keep-history document and
    /// removed by the terminal erase chunk, so both estimate through this one
    /// entry point; its own version slot keeps a later change to the layer
    /// counts or record size from moving the estimates of the generations
    /// that already rely on it.
    ///
    /// # Parameters
    /// * `contract`: The contract that contains the document type.
    /// * `document_type`: The type whose lifecycle tree holds the record.
    /// * `layers`: The estimated layer information the dry run builds.
    /// * `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn add_estimation_costs_for_lifecycle_record(
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
            .add_estimation_costs_for_lifecycle_record
        {
            Some(0) => {
                Self::add_estimation_costs_for_lifecycle_record_v0(contract, document_type, layers)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_lifecycle_record".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_lifecycle_record".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
