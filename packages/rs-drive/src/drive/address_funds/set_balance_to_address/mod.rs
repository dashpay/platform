mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Sets a balance for a given address in the AddressBalances tree.
    /// This operation directly sets (or overwrites) the balance for the address with the given nonce.
    ///
    /// # Parameters
    /// - `address`: The platform address
    /// - `nonce`: The nonce for the address
    /// - `balance`: The balance value to set
    /// - `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// - `drive_operations`: The list of drive operations to append to.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(())` if the operation was successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn set_balance_to_address(
        &self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let ops = self.set_balance_to_address_operations(
            address,
            nonce,
            balance,
            estimated_costs_only_with_layer_info,
            platform_version,
        )?;
        drive_operations.extend(ops);
        Ok(())
    }

    /// Returns operations for setting a balance to an address.
    ///
    /// # Parameters
    /// - `address`: The platform address
    /// - `nonce`: The nonce for the address
    /// - `balance`: The balance value to set
    /// - `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(Vec<LowLevelDriveOperation>)` - The operations to set balance.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn set_balance_to_address_operations(
        &self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .set_balance_to_address
        {
            0 => self.set_balance_to_address_operations_v0(
                address,
                nonce,
                balance,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_balance_to_address_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
