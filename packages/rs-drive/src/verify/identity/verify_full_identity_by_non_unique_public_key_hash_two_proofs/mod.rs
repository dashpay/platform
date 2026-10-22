mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the separate index and identity proofs that answer a
    /// `getIdentityByNonUniquePublicKeyHash` V0 request.
    ///
    /// The index proof must prove exactly the first index entry after `after`, or that
    /// there is none, and the identity proof must prove that identity at the same root.
    /// An identity proof without an index entry, or an index entry without its
    /// identity, is refused.
    ///
    /// # Parameters
    ///
    /// - `index_proof`: The proof of the first index entry after `after`.
    /// - `identity_proof`: The proof of the full identity that entry names, if any.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user's public key.
    /// - `after`: An optional 32-byte array specifying an identity after which
    ///   the search should begin when retrieving the identity.
    /// - `query_platform_version`: The producer's signed version; it selects how the
    ///   proved queries are built.
    /// - `platform_version`: The verifier's version; it selects this method's generation
    ///   and the GroveDB rules the proofs are verified with.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash of GroveDB after verification. It still needs the
    ///   quorum signature over it.
    /// - `Option<Identity>`: The full identity of the user, if it exists.
    ///
    /// # Errors
    ///
    /// This function returns an `Error` if:
    /// - Either proof is invalid, incomplete, or proves something other than the query.
    /// - The two proofs have different roots.
    /// - The provided platform version is unknown or unsupported.
    pub fn verify_full_identity_by_non_unique_public_key_hash_two_proofs(
        index_proof: &[u8],
        identity_proof: Option<&[u8]>,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_non_unique_public_key_hash_two_proofs
        {
            0 => Self::verify_full_identity_by_non_unique_public_key_hash_two_proofs_v0(
                index_proof,
                identity_proof,
                public_key_hash,
                after,
                query_platform_version,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identity_by_non_unique_public_key_hash_two_proofs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_full_identity_by_non_unique_public_key_hash_two_proofs_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_non_unique_public_key_hash_two_proofs = 255;

        let result = Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
            &[],
            None,
            [0u8; 20],
            None,
            &platform_version,
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_full_identity_by_non_unique_public_key_hash_two_proofs" && known_versions == vec![0] && received == 255
            )
        );
    }
}
