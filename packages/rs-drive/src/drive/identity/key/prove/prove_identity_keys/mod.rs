mod v0;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Prove the requested identity keys.
    ///
    /// This function takes an `IdentityKeysRequest` and a `TransactionArg` as arguments
    /// and returns a proof of the requested identity keys as a `Vec<u8>` or an error
    /// if the proof cannot be generated.
    ///
    /// # Arguments
    ///
    /// * `key_request` - An `IdentityKeysRequest` containing the details of the
    ///   requested identity keys, such as the identity ID, request type, limit, and offset.
    /// * `transaction` - A `TransactionArg` representing the current transaction.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - A proof of the requested identity keys as a `Vec<u8>` if the
    ///   proof is successfully generated.
    /// * `Err(Error)` - An error if the proof cannot be generated or the version is not supported.
    ///
    /// # Errors
    ///
    /// This function may return `UnknownVersionMismatch` error if the version is not supported.
    pub fn prove_identity_keys(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .prove
            .prove_identity_keys
        {
            0 => self.prove_identity_keys_v0(key_request, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
