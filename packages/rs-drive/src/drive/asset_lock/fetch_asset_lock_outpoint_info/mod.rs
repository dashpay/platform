//! Implements in Drive functions which check if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::platform_value::Bytes36;
use dpp::version::drive_versions::DriveVersion;

use dpp::asset_lock::StoredAssetLockInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Checks if a given `outpoint` is present as an asset lock in the transaction.
    ///
    /// # Arguments
    ///
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub fn fetch_asset_lock_outpoint_info(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<StoredAssetLockInfo, Error> {
        match drive_version
            .methods
            .asset_lock
            .fetch_asset_lock_outpoint_info
        {
            0 => self.fetch_asset_lock_outpoint_info_v0(outpoint, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_asset_lock_outpoint_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Checks if a given `outpoint` is present as an asset lock in the transaction and potentially applies operations to it.
    ///
    /// # Arguments
    ///
    /// * `apply` - A boolean which when true applies the operations to the asset lock.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` to be possibly executed.
    /// * `outpoint` - An `OutPoint` reference to be checked in the transaction.
    /// * `transaction` - The `TransactionArg` in which to check for the `outpoint`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is `Ok` if the outpoint exists in the transaction or an `Error` otherwise.
    pub(crate) fn fetch_asset_lock_outpoint_info_add_operations(
        &self,
        apply: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<StoredAssetLockInfo, Error> {
        match drive_version
            .methods
            .asset_lock
            .fetch_asset_lock_outpoint_info
        {
            0 => self.fetch_asset_lock_outpoint_info_add_operations_v0(
                apply,
                drive_operations,
                outpoint,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_asset_lock_outpoint_info_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
