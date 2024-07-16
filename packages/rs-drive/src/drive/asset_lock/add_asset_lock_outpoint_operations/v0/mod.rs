//! Implements in Drive a function which adds operations to a given `outpoint` if it is present in the estimated costs.

use crate::drive::asset_lock::asset_lock_storage_path;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;

use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::platform_value::Bytes36;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::Element::Item;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds operations to a given `outpoint` if it is present in the estimated costs.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current object.
    /// * `outpoint` - An `OutPoint` reference to be potentially modified.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` that contains layer information.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `LowLevelDriveOperation` if successful, or an `Error` otherwise.
    pub(super) fn add_asset_lock_outpoint_operations_v0(
        &self,
        outpoint: &Bytes36,
        asset_lock_value: AssetLockValue,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_adding_asset_lock(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let item_bytes = if asset_lock_value.remaining_credit_value() > 0 {
            asset_lock_value.serialize_to_bytes()?
        } else {
            // If there is no credit value we don't actually put any system credits,
            // This is to save space, since the information can be retrieved from the core chain.
            vec![]
        };

        self.batch_insert(
            PathFixedSizeKeyRefElement((
                asset_lock_storage_path(),
                outpoint.as_slice(),
                Item(item_bytes, None),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
