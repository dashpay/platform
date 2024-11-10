mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::identity::Identity;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Retrieves all full identities associated with a given non-unique public key hash.
    ///
    /// This function fetches all identity data associated with a specified public key hash,
    /// using the version specified in `PlatformVersion` to determine the correct method implementation.
    ///
    /// # Parameters
    ///
    /// - `public_key_hash`: A 20-byte array representing the non-unique public key hash to look up.
    /// - `limit`: An optional limit on the number of identities to retrieve.
    /// - `transaction`: A `TransactionArg` representing the transaction context.
    /// - `platform_version`: A reference to the `PlatformVersion`, which selects the method version for identity fetching.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `Vec<Identity>` where:
    /// - Each `Identity` represents a verified identity associated with the public key hash.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The provided public key hash does not correspond to any identities.
    /// - The method version specified in `PlatformVersion` is unsupported.
    ///
    pub fn fetch_full_identities_for_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .public_key_hashes
            .fetch_full_identities_for_non_unique_public_key_hash
        {
            0 => self.fetch_full_identities_for_non_unique_public_key_hash_v0(
                public_key_hash,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identities_for_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
