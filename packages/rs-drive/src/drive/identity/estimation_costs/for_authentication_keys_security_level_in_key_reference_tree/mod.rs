use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identity::SecurityLevel;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds estimation costs for authentication keys security level in key reference tree.
    ///
    /// It takes in the following parameters: `identity_id`, a mutable reference to a HashMap containing
    /// the estimated costs with layer info, and the `security_level`.
    ///
    /// Based on the `security_level`, it updates the provided `HashMap` with new estimated costs.
    ///
    /// # Parameters
    /// - `identity_id`: A 32-byte array representing the identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing
    ///   the `KeyInfoPath` and `EstimatedLayerInformation`.
    /// - `security_level`: The security level of the keys.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        security_level: SecurityLevel,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_authentication_keys_security_level_in_key_reference_tree
        {
            0 => Ok(Self::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree_v0(
                identity_id,
                estimated_costs_only_with_layer_info,
                security_level
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
