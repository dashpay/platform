mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::KeyOfType;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves the balance and nonce for a given address from the AddressBalances tree.
    ///
    /// This function queries the GroveDB to prove the balance and nonce associated with a specific
    /// address key. The method selects the appropriate version based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `key_of_type`: The key (containing key type and key data) to prove
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The proof bytes for the specified address balance and nonce.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_balance_and_nonce(
        &self,
        key_of_type: &KeyOfType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove_balance_and_nonce
        {
            0 => self.prove_balance_and_nonce_v0(key_of_type, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_balance_and_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves the balance and nonce and adds corresponding operations to the drive.
    ///
    /// This function is similar to `prove_balance_and_nonce` but also adds operations to the drive
    /// for tracking costs.
    ///
    /// # Parameters
    /// - `key_of_type`: The key (containing key type and key data) to prove
    /// - `transaction`: The transaction argument used for the query.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: The proof bytes for the specified address balance and nonce.
    /// - `Err(Error)`: If an error occurs during the proof generation.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_balance_and_nonce_operations(
        &self,
        key_of_type: &KeyOfType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .prove
            .prove_balance_and_nonce
        {
            0 => self.prove_balance_and_nonce_operations_v0(
                key_of_type,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_balance_and_nonce_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
