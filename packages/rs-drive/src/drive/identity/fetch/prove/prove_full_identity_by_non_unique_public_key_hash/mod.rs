mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generates a proof for an identity associated with a given non-unique public key hash.
    ///
    /// This function retrieves an identity along with its associated proofs from storage.
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
    /// Returns a `Result` containing an [`IdentityAndNonUniquePublicKeyHashDoubleProof`], which
    /// includes both the proof of the identity and the proof linking the public key hash to
    /// an identity ID. If the operation fails or the platform version is unsupported, an `Error`
    /// is returned.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    /// - The identity retrieval operation fails.
    /// - The provided public key hash does not correspond to a known identity.
    /// - The requested platform version is unknown or not supported.
    ///
    /// # Versioning
    ///
    /// - Currently, only version `0` of `prove_full_identity_by_non_unique_public_key_hash`
    ///   is implemented. If an unsupported version is provided, an `UnknownVersionMismatch`
    ///   error is returned.
    pub fn prove_full_identity_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityAndNonUniquePublicKeyHashDoubleProof, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identity_by_non_unique_public_key_hash
        {
            0 => self.prove_full_identity_by_non_unique_public_key_hash_v0(
                public_key_hash,
                after,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
