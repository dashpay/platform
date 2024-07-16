use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds estimated storage costs for adding a document to primary storage based on platform version.
    ///
    /// This function uses the platform version to determine the appropriate method to estimate storage costs
    /// for adding a document to the primary storage. Currently, it supports version `0` and delegates the estimation
    /// to `add_estimation_costs_for_add_document_to_primary_storage_v0`.
    ///
    /// If an unsupported version is provided, an error indicating a version mismatch will be returned.
    ///
    /// # Arguments
    /// * `document_and_contract_info`: Information about the document and its associated contract.
    /// * `primary_key_path`: Key path where the document should be stored in primary storage.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a hashmap where the estimated layer
    ///   information will be stored for the given key path.
    /// * `platform_version`: Version of the platform being used, which determines the estimation method.
    ///
    /// # Returns
    /// * `Result<(), Error>`: Returns `Ok(())` if the operation succeeds. Returns an `Error` if the provided platform
    ///   version method is unsupported or if there's any other issue.
    ///
    /// # Errors
    /// * `DriveError::UnknownVersionMismatch`: Returned if the platform version method specified is unsupported.
    ///
    /// # Panics
    /// This function will not panic under normal circumstances. However, unexpected behavior may result
    /// from incorrect arguments or unforeseen edge cases.
    pub(crate) fn add_estimation_costs_for_add_contested_document_to_primary_storage<
        const N: usize,
    >(
        document_and_contract_info: &DocumentAndContractInfo,
        primary_key_path: [&[u8]; N],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .estimation_costs
            .add_estimation_costs_for_add_contested_document_to_primary_storage
        {
            0 => Self::add_estimation_costs_for_add_contested_document_to_primary_storage_v0(
                document_and_contract_info,
                primary_key_path,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::add_estimation_costs_for_add_contested_document_to_primary_storage"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
