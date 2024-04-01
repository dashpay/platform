//! Implements in Drive functions which check if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it (version 0).

use crate::drive::asset_lock::asset_lock_storage_path;
use crate::drive::grove_operations::DirectQueryType::{StatefulDirectQuery, StatelessDirectQuery};
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;

use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::platform_value::Bytes36;
use dpp::serialization::PlatformDeserializable;
use grovedb::TransactionArg;

impl Drive {
    /// Checks if a given `outpoint` is present as an asset lock in the transaction.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current object.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(super) fn fetch_asset_lock_outpoint_info_v0(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<StoredAssetLockInfo, Error> {
        self.fetch_asset_lock_outpoint_info_add_operations(
            true,
            &mut vec![],
            outpoint,
            transaction,
            drive_version,
        )
    }

    /// Checks if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current object.
    /// * `apply` - A boolean which when true applies the operations to the asset lock.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` to be possibly executed.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(super) fn fetch_asset_lock_outpoint_info_add_operations_v0(
        &self,
        apply: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<StoredAssetLockInfo, Error> {
        let asset_lock_storage_path = asset_lock_storage_path();
        let query_type = if apply {
            StatefulDirectQuery
        } else {
            StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(36),
            }
        };

        Ok(self
            .grove_get_raw_optional(
                (&asset_lock_storage_path).into(),
                outpoint.as_slice(),
                query_type,
                transaction,
                drive_operations,
                drive_version,
            )?
            .map(|element| {
                let item_bytes = element.as_item_bytes()?;
                if item_bytes.is_empty() {
                    Ok::<StoredAssetLockInfo, Error>(StoredAssetLockInfo::FullyConsumed)
                } else {
                    Ok(StoredAssetLockInfo::PartiallyConsumed(
                        AssetLockValue::deserialize_from_bytes(item_bytes)?,
                    ))
                }
            })
            .transpose()?
            .unwrap_or(StoredAssetLockInfo::NotPresent))
    }
}
