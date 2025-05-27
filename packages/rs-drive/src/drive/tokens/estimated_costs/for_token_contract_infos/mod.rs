mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for token contract infos changes based on the provided drive version.
    ///
    /// This method updates the `estimated_costs_only_with_layer_info` HashMap with entries
    /// representing the estimated costs for different layers of the token contract infos tree. The method
    /// adjusts its behavior depending on the provided `drive_version`, allowing it to support
    /// different versioned implementations for cost estimation.
    ///
    /// # Parameters
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap that holds
    ///   `KeyInfoPath` and `EstimatedLayerInformation` for each token contract info layer.
    /// - `drive_version`: The version of the drive to determine which estimation logic to apply.
    /// # Returns
    /// - `Ok(())` if the operation is successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the provided `drive_version` does not match
    ///   any known supported versions.
    ///
    /// # Errors
    /// This function will return an error if the provided `drive_version` does not match a known version.
    pub(crate) fn add_estimation_costs_for_token_contract_infos(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_token_contract_infos
        {
            0 => {
                Self::add_estimation_costs_for_token_contract_infos_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_token_contract_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
