mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Checks if a key with a given public key hash already exists in the non-unique set for a specific identity.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - Public key hash to be checked.
    /// * `identity_id` - Identity ID to be checked.
    /// * `transaction` - Transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of drive operations.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a boolean value indicating the existence of the public key hash, otherwise an `Error` if the operation fails or the version is not supported.
    pub(crate) fn has_non_unique_public_key_hash_already_for_identity_operations(
        &self,
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .identity
            .fetch
            .public_key_hashes
            .has_non_unique_public_key_hash_already_for_identity
        {
            0 => self.has_non_unique_public_key_hash_already_for_identity_operations_v0(
                public_key_hash,
                identity_id,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "has_non_unique_public_key_hash_already_for_identity_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
