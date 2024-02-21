use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// This function adds estimation costs for an updated revision.
    ///
    /// It expects an identity in the form of an array of bytes, and a mutable reference to a HashMap containing
    /// the estimated costs with layer info. Additionally, it takes a reference to the drive version.
    ///
    /// Based on the version of the drive, it calls the appropriate function to handle cost estimation.
    ///
    /// # Parameters
    /// - `identity_id`: A 32-byte array representing the identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing
    ///   the `KeyInfoPath` and `EstimatedLayerInformation`.
    /// - `drive_version`: A reference to the `DriveVersion`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_update_nonce(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_update_revision
        {
            0 => {
                Self::add_estimation_costs_for_update_nonce_v0(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_update_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
