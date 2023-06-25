mod v0;

use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::data_contract::document_type::DocumentType;
use crate::contract::Contract;
use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES,
    AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    DEFAULT_HASH_SIZE_U8,
};
use crate::drive::Drive;
use crate::drive::flags::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;

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
        document_type: &DocumentType,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.methods.document.delete.add_estimation_costs_for_remove_document_to_primary_storage {
            0 => Ok(Self::add_estimation_costs_for_remove_document_to_primary_storage_v0(
                primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_remove_document_to_primary_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}