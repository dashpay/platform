mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Checks if a key with a given public key hash already exists in the non-unique set.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - Public key hash to be checked.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a boolean value indicating the existence of the public key hash, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn has_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .identity
            .fetch
            .public_key_hashes
            .has_non_unique_public_key_hash
        {
            0 => {
                self.has_non_unique_public_key_hash_v0(public_key_hash, transaction, drive_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Checks if a key with a given public key hash already exists in the non-unique set.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - Public key hash to be checked.
    /// * `transaction` - Transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of drive operations.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a boolean value indicating the existence of the public key hash, otherwise an `Error` if the operation fails or the version is not supported.
    pub(crate) fn has_non_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .identity
            .fetch
            .public_key_hashes
            .has_non_unique_public_key_hash
        {
            0 => self.has_non_unique_public_key_hash_operations_v0(
                public_key_hash,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_non_unique_public_key_hash_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
