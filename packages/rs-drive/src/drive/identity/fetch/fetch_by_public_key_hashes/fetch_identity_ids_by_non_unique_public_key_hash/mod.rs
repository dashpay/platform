mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches identity ids from storage based on a non-unique public key hash.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - A non-unique public key hash corresponding to the identity ids to be fetched.
    /// * `limit` - An optional limit.
    /// * `transaction` - Transaction arguments.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of identity ids if they exist, otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub fn fetch_identity_ids_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_identity_ids_by_non_unique_public_key_hash
        {
            0 => self.fetch_identity_ids_by_non_unique_public_key_hash_v0(
                public_key_hash,
                limit,
                after,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_ids_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    pub(crate) fn fetch_identity_ids_by_non_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_identity_ids_by_non_unique_public_key_hash
        {
            0 => self.fetch_identity_ids_by_non_unique_public_key_hash_operations_v0(
                public_key_hash,
                limit,
                after,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_ids_by_non_unique_public_key_hash_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
