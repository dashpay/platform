mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::KeyOfType;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves the balances and nonces for multiple addresses from the AddressBalances tree.
    ///
    /// This function queries the GroveDB to prove the balances and nonces associated with multiple
    /// address keys. The method selects the appropriate version based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `keys_of_type`: An iterator over keys (containing key type and key data) to prove
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The proof bytes for the specified address balances and nonces.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_balances_with_nonces<'a, I>(
        &self,
        keys_of_type: I,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a KeyOfType>,
    {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_balances_with_nonces
        {
            0 => self.prove_balances_with_nonces_v0(keys_of_type, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_balances_with_nonces".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves the balances and nonces and adds corresponding operations to the drive.
    ///
    /// This function is similar to `prove_balances_with_nonces` but also adds operations to the drive
    /// for tracking costs.
    ///
    /// # Parameters
    /// - `keys_of_type`: An iterator over keys (containing key type and key data) to prove
    /// - `transaction`: The transaction argument used for the query.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The proof bytes for the specified address balances and nonces.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_balances_with_nonces_operations<'a, I>(
        &self,
        keys_of_type: I,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a KeyOfType>,
    {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_balances_with_nonces
        {
            0 => self.prove_balances_with_nonces_operations_v0(
                keys_of_type,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_balances_with_nonces_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
