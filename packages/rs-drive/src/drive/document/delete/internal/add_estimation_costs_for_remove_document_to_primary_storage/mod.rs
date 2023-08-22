mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated costs for removing a document to primary storage.
    ///
    /// # Parameters
    /// * `primary_key_path`: The primary key path of the document.
    /// * `document_type`: The type of the document.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap for storing estimated costs with layer information.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn add_estimation_costs_for_remove_document_to_primary_storage(
        primary_key_path: [&[u8]; 5],
        document_type: DocumentTypeRef,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .add_estimation_costs_for_remove_document_to_primary_storage
        {
            0 => Self::add_estimation_costs_for_remove_document_to_primary_storage_v0(
                primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_remove_document_to_primary_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
