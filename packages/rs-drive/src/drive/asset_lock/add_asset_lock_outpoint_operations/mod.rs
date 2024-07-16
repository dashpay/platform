//! Implements in Drive a function which adds operations to a given `outpoint` if it is present in the estimated costs.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::platform_value::Bytes36;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds operations to a given `outpoint` if it is present in the estimated costs.
    ///
    /// # Arguments
    ///
    /// * `outpoint` - An `OutPoint` reference to be potentially modified.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` that contains layer information.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `LowLevelDriveOperation` if successful, or an `Error` otherwise.
    pub fn add_asset_lock_outpoint_operations(
        &self,
        outpoint: &Bytes36,
        asset_lock_value: AssetLockValue,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .asset_lock
            .add_asset_lock_outpoint
        {
            0 => self.add_asset_lock_outpoint_operations_v0(
                outpoint,
                asset_lock_value,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_asset_lock_outpoint_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
