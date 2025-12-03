mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Removes a balance from a given address in the AddressBalances tree.
    /// This operation subtracts the specified amount from the address's current balance.
    /// The nonce stays the same. If the amount to remove exceeds the current balance,
    /// the balance is clamped to 0.
    ///
    /// # Parameters
    /// - `address`: The platform address
    /// - `amount_to_remove`: The balance amount to subtract
    /// - `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// - `drive_operations`: The list of drive operations to append to.
    /// * `transaction` - A `TransactionArg` object representing the database transaction to be used.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(Credits)` - The amount that was actually removed from the balance.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn remove_balance_from_address(
        &self,
        address: PlatformAddress,
        amount_to_remove: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .remove_balance_from_address
        {
            0 => self.remove_balance_from_address_v0(
                address,
                amount_to_remove,
                estimated_costs_only_with_layer_info,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_balance_from_address".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
