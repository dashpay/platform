mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::identity::Identity;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its related information from storage based on a non unique public key hash.
    ///
    /// This function leverages the versioning system to direct the fetch operation to the appropriate handler based on the `DriveVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - A non-unique public key hash corresponding to the identity to be fetched.
    /// * `after` - An identity ID after which we want to get back our identity. Basically "don't get back this identity, get the next one"
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option` of the `Identity` if it exists, otherwise an `Error` if the fetch operation fails or the version is not supported.
    pub fn fetch_full_identity_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_full_identity_by_non_unique_public_key_hash
        {
            0 => self.fetch_full_identity_by_non_unique_public_key_hash_v0(
                public_key_hash,
                after,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
