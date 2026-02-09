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
    /// Adds a balance for a given address in the AddressBalances tree.
    /// This operation directly adds the balance for the address.
    /// The nonce stays the same. If there is no address the nonce becomes 0.
    ///
    /// # Parameters
    /// - `address`: The platform address
    /// - `balance`: The balance value to set
    /// - `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// - `drive_operations`: The list of drive operations to append to.
    /// * `transaction` - A `TransactionArg` object representing the database transaction to be used.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(())` if the operation was successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn add_balance_to_address(
        &self,
        address: PlatformAddress,
        balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let ops = self.add_balance_to_address_operations(
            address,
            balance,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(ops);
        Ok(())
    }

    /// Returns operations for adding a balance to an address.
    ///
    /// # Parameters
    /// - `address`: The platform address
    /// - `balance`: The balance value to add
    /// - `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// * `transaction` - A `TransactionArg` object representing the database transaction to be used.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(Vec<LowLevelDriveOperation>)` - The operations to add balance.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn add_balance_to_address_operations(
        &self,
        address: PlatformAddress,
        balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .add_balance_to_address
        {
            0 => self.add_balance_to_address_operations_v0(
                address,
                balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_balance_to_address_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
