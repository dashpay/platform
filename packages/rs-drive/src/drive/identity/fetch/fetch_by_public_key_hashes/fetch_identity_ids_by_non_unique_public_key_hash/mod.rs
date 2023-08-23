mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches identity ids from storage based on a non-unique public key hash.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - A non-unique public key hash corresponding to the identity ids to be fetched.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of identity ids if they exist, otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub fn fetch_identity_ids_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        match drive_version
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_identity_ids_by_non_unique_public_key_hash
        {
            0 => self.fetch_identity_ids_by_non_unique_public_key_hash_v0(
                public_key_hash,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_ids_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
