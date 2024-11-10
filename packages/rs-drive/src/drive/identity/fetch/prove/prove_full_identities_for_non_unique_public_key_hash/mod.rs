mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the complete identities for a given non-unique public key hash, returning proofs of the identities.
    ///
    /// This function selects the appropriate handler based on the provided `PlatformVersion` using versioning.
    ///
    /// # Arguments
    ///
    /// * `public_key_hash` - A 20-byte array representing the public key hash for which identities are requested.
    /// * `transaction` - The transaction context for the operation.
    /// * `platform_version` - A reference to the platform version, determining the method version to call.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identities. If the operation fails or the
    /// version is unsupported, an `Error` is returned.
    pub fn prove_full_identities_for_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identities_for_non_unique_public_key_hash
        {
            0 => self.prove_full_identities_for_non_unique_public_key_hash_v0(
                public_key_hash,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identities_for_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
