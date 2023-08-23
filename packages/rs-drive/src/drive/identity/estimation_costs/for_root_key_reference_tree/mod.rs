use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds estimation costs for root key reference tree for a given identity id.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for the root key reference tree of the specified identity id.
    ///
    /// # Parameters
    /// - `identity_id`: An array of 32 bytes representing the unique identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing the `KeyInfoPath` and `EstimatedLayerInformation`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_root_key_reference_tree(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_root_key_reference_tree
        {
            0 => {
                Self::add_estimation_costs_for_root_key_reference_tree_v0(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_root_key_reference_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
