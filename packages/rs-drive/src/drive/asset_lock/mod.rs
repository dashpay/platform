mod estimation_costs;

use crate::drive::grove_operations::DirectQueryType::{StatefulDirectQuery, StatelessDirectQuery};
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::dashcore::{OutPoint, TxOut};
use dpp::platform_value::Bytes36;
use grovedb::batch::KeyInfoPath;
use grovedb::Element::Item;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The asset lock root storage path
pub(crate) fn asset_lock_storage_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions)]
}

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
    pub fn has_asset_lock_outpoint(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        self.has_asset_lock_outpoint_add_operations(true, &mut vec![], outpoint, transaction)
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
    pub fn has_asset_lock_outpoint_add_operations(
        &self,
        apply: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        outpoint: &Bytes36,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        let asset_lock_storage_path = asset_lock_storage_path();
        let query_type = if apply {
            StatefulDirectQuery
        } else {
            StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(36),
            }
        };
        self.grove_has_raw(
            asset_lock_storage_path,
            outpoint.as_slice(),
            query_type,
            transaction,
            drive_operations,
        )
    }

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
    pub fn add_asset_lock_outpoint_operations(
        &self,
        outpoint: &Bytes36,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_adding_asset_lock(estimated_costs_only_with_layer_info);
        }
        self.batch_insert(
            PathFixedSizeKeyRefElement((
                asset_lock_storage_path(),
                outpoint.as_slice(),
                Item(vec![], None),
            )),
            &mut drive_operations,
        )?;
        Ok(drive_operations)
    }
}
