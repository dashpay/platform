mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds cost estimation entries for a token's pre-programmed distribution tree.
    ///
    /// This function updates the provided `estimated_costs_only_with_layer_info` hashmap with estimation entries
    /// for each layer in the pre-programmed distribution tree associated with a specific token. The tree structure
    /// includes:
    ///
    /// - The root level of the pre-programmed distributions.
    /// - The token-specific subtree (keyed by `token_id`).
    /// - One sum tree per distribution time (each timestamp in `times`).
    ///
    /// The function selects the appropriate estimation logic based on the provided `drive_version`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32-byte identifier for the token whose pre-programmed distribution tree is being estimated.
    /// - `times`: A vector of timestamps (in milliseconds) for which pre-programmed distributions exist.
    /// - `estimated_costs_only_with_layer_info`: A mutable hashmap that maps `KeyInfoPath` to `EstimatedLayerInformation`.
    ///   This cache is used by Grovedb to track the estimated storage costs for each layer in the tree.
    /// - `drive_version`: The drive version that determines which estimation logic to use.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the estimation entries were successfully added.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the provided `drive_version` does not match any supported version.
    ///
    /// # Errors
    ///
    /// Returns an error if the `drive_version` is not recognized, ensuring that only supported estimation
    /// implementations are applied.
    pub(crate) fn add_estimation_costs_for_token_perpetual_distribution(
        token_id: Option<[u8; 32]>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_token_perpetual_distribution
        {
            0 => {
                Self::add_estimation_costs_for_token_perpetual_distribution_v0(
                    token_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_token_perpetual_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
