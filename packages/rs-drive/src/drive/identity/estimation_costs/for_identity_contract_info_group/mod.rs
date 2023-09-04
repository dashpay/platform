mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for balances.
    ///
    /// It operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for different layers of the balance tree.
    ///
    /// # Parameters
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing
    ///   the `KeyInfoPath` and `EstimatedLayerInformation`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_contract_info_group(
        identity_id: &[u8; 32],
        group_id: &[u8],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_contract_info_group
        {
            0 => {
                Self::add_estimation_costs_for_contract_info_group_v0(
                    identity_id,
                    group_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
