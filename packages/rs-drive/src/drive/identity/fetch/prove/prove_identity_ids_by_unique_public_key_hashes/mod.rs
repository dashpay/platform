mod v0;

use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;
use dpp::version::drive_versions::DriveVersion;
use crate::error::drive::DriveError;

impl Drive {
    /// Proves identity ids against public key hashes.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `public_key_hashes` - The public key hashes for which to prove the identity ids.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identity ids, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn prove_identity_ids_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version.methods.identity.prove.prove_identity_ids_by_unique_public_key_hashes {
            0 => self.prove_identity_ids_by_unique_public_key_hashes_v0(public_key_hashes, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_ids_by_unique_public_key_hashes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}