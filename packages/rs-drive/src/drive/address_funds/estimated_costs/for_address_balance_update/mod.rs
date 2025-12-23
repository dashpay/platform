mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for address balance updates.
    ///
    /// This function updates the provided HashMap with layer information for
    /// the address balances tree structure. It estimates the costs of updating
    /// balances in the AddressBalances sum tree.
    ///
    /// # Parameters
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing
    ///   the `KeyInfoPath` and `EstimatedLayerInformation`.
    /// - `drive_version`: The drive version to use for method selection.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    pub(crate) fn add_estimation_costs_for_address_balance_update(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .address_funds
            .cost_estimation
            .for_address_balance_update
        {
            0 => {
                Self::add_estimation_costs_for_address_balance_update_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_address_balance_update".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
