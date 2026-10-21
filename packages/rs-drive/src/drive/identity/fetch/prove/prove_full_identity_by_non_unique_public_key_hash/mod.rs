mod v0;
mod v1;

use crate::drive::identity::non_unique_public_key_hash_proof::NonUniquePublicKeyHashProof;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generates a proof for an identity associated with a given non-unique public key hash.
    ///
    /// This function proves the first matching identity and its index selection at one root.
    /// It utilizes versioning to call the appropriate handler based on the provided
    /// `PlatformVersion`.
    ///
    /// # Arguments
    ///
    /// - `public_key_hash` - A 20-byte array representing the hash of the public key
    ///   for which the identity should be fetched.
    /// - `after` - An optional identity ID specifying the starting point for retrieval.
    ///   If provided, the function will return the identity that appears after the given ID,
    ///   ensuring that the specified identity itself is not included.
    /// - `transaction` - A transaction argument used for database operations.
    /// - `platform_version` - A reference to the platform version, ensuring that the
    ///   correct version-specific function is used.
    ///
    /// # Returns
    ///
    /// Returns separate index/identity proofs for generation 0, or one complete
    /// GroveDB proof for generation 1. The encoding follows the producer's table.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    /// - The identity retrieval operation fails.
    /// - Concurrent commits change the root on every bounded attempt.
    /// - The requested platform version is unknown or not supported.
    ///
    /// # Versioning
    ///
    /// - Generations 0 and 1 preserve the historical and current proof encodings.
    ///   Unsupported versions return `UnknownVersionMismatch`.
    pub fn prove_full_identity_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<NonUniquePublicKeyHashProof, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identity_by_non_unique_public_key_hash
        {
            0 => self
                .prove_full_identity_by_non_unique_public_key_hash_v0(
                    public_key_hash,
                    after,
                    transaction,
                    platform_version,
                )
                .map(NonUniquePublicKeyHashProof::Legacy),
            1 => self
                .prove_full_identity_by_non_unique_public_key_hash_v1(
                    public_key_hash,
                    after,
                    transaction,
                    platform_version,
                )
                .map(NonUniquePublicKeyHashProof::Current),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
