mod v0;

use crate::drive::identity::key::fetch::{IdentityKeysRequest, IdentityPublicKeyResult};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Fetch keys matching the request for a specific Identity
    ///
    /// This method allows for querying of specific keys associated with an identity. The `key_request`
    /// defines the types and parameters of the keys to be fetched.
    ///
    /// # Arguments
    ///
    /// * `key_request` - An `IdentityKeysRequest` object containing the details of the key search.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for fetching the keys.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// * `Result<T, Error>` - If successful, returns a `T` object where `T` implements `IdentityPublicKeyResult`.
    ///   If an error occurs during the key fetching, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the key fetching fails or the version is not supported.
    pub fn fetch_identity_keys<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .fetch
            .fetch_identity_keys
        {
            0 => self.fetch_identity_keys_v0(key_request, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Operations for fetching keys matching the request for a specific Identity
    ///
    /// This method fetches the operations that will be used to fetch the requested identity keys.
    ///
    /// # Arguments
    ///
    /// * `key_request` - An `IdentityKeysRequest` object containing the details of the key search.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for fetching the keys.
    /// * `drive_operations` - A mutable reference to a vector that will hold the resulting drive operations.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// * `Result<T, Error>` - If successful, returns a `T` object where `T` implements `IdentityPublicKeyResult`.
    ///   If an error occurs during the operation fetching, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the operation fetching fails or the version is not supported.
    pub(crate) fn fetch_identity_keys_operations<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .fetch
            .fetch_identity_keys
        {
            0 => self.fetch_identity_keys_operations_v0(
                key_request,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_keys_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
