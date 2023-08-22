mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for total system credits update.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for the total system credits update.
    ///
    /// # Parameters
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing the `KeyInfoPath` and `EstimatedLayerInformation`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_total_system_credits_update(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .platform_system
            .estimation_costs
            .for_total_system_credits_update
        {
            0 => {
                Self::add_estimation_costs_for_total_system_credits_update_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_total_system_credits_update".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
