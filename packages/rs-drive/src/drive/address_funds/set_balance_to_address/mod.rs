mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::identity::KeyOfTypeWithNonce;
use grovedb_epoch_based_storage_flags::StorageFlags;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Sets a balance for a given address in the AddressBalances tree.
    /// This operation directly sets (or overwrites) the balance for the address with the given nonce.
    ///
    /// # Parameters
    /// - `key_of_type_with_nonce`: The key (containing key type and key data) with its associated nonce
    /// - `balance`: The balance value to set
    /// - `drive_operations`: The list of drive operations to append to.
    /// - `storage_flags`: Storage flags to apply to the element
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(())` if the operation was successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn set_balance_to_address(
        &self,
        key_of_type_with_nonce: KeyOfTypeWithNonce,
        balance: Credits,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        storage_flags: StorageFlags,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .set_balance_to_address
        {
            0 => self.set_balance_to_address_v0(
                key_of_type_with_nonce,
                balance,
                drive_operations,
                storage_flags,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_balance_to_address".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
