mod v0;
mod v1;

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
    /// # Version dispatch
    ///
    /// - **v0** (protocol v11, the feature's initial release) uses
    ///   `AllItems(...)` for the CLEAR_ADDRESS_POOL leaf layer. This
    ///   undercharges by ~10 bytes per insert because the actual leaves
    ///   are `ItemWithSumItem(nonce, balance, flags)` whose `i64`
    ///   sum_value isn't accounted for. The undercharge is
    ///   consensus-locked to v11 prod — switching to a corrected
    ///   formula would change fees for already-applied state.
    /// - **v1** (protocol v12+) uses `AllItemsWithSumItem(...)` so the
    ///   sum_value byte cost is included. Unblocked by grovedb #674.
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
            1 => {
                Self::add_estimation_costs_for_address_balance_update_v1(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_address_balance_update".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
