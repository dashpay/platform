mod legacy;
mod v0;
mod v1;

use crate::drive::identity::non_unique_public_key_hash_proof::NonUniquePublicKeyHashProofRef;
use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the full identity of a user using their non-unique public key hash.
    ///
    /// The explicit proof encoding selects the historical v0 or current v1 verifier.
    /// Raw bytes mean the current encoding, including on deployed protocol 13 nodes.
    /// The caller's verification rules apply independently of the wire encoding.
    ///
    /// # Parameters
    ///
    /// - `proof`: An explicit legacy/current proof, or raw current-format bytes.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user's public key.
    /// - `after`: An optional 32-byte array specifying an identity after which
    ///   the search should begin when retrieving the identity.
    /// - `query_platform_version`: The producer's signed query construction version.
    /// - `platform_version`: A reference to the platform version, ensuring that
    ///   the verification security table is used independently of query construction.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash of GroveDB after verification.
    /// - `Option<Identity>`: The full identity of the user, if it exists.
    ///
    /// If no identity is found, the returned `Option<Identity>` will be `None`.
    ///
    /// # Errors
    ///
    /// This function returns an `Error` if:
    /// - The provided proof is invalid.
    /// - The public key hash does not correspond to a valid identity ID.
    /// - The identity ID exists but does not correspond to a valid full identity.
    /// - The provided platform version is unknown or unsupported.
    ///
    /// # Versioning
    ///
    /// - Generations 0 and 1 select the historical and current proof layouts.
    ///   Unsupported verifier versions return `UnknownVersionMismatch`.
    pub fn verify_full_identity_by_non_unique_public_key_hash<'a>(
        proof: impl Into<NonUniquePublicKeyHashProofRef<'a>>,
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
            .verify_full_identity_by_non_unique_public_key_hash
        {
            0 | 1 => match proof.into() {
                NonUniquePublicKeyHashProofRef::Legacy(proof) => {
                    Self::verify_legacy_identity_by_non_unique_public_key_hash(
                        &proof.identity_id_public_key_hash_proof,
                        proof.identity_proof.as_deref(),
                        public_key_hash,
                        after,
                        query_platform_version,
                        platform_version,
                    )
                }
                NonUniquePublicKeyHashProofRef::Current(proof) => {
                    Self::verify_full_identity_by_non_unique_public_key_hash_v1(
                        proof,
                        public_key_hash,
                        after,
                        query_platform_version,
                        platform_version,
                    )
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Verify legacy separate index and identity proofs against one authenticated root.
    ///
    /// The legacy wire encoding selects historical v0 behind strict proof guards.
    /// The caller's verification security floor is retained independently.
    pub fn verify_legacy_identity_by_non_unique_public_key_hash(
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
            .verify_full_identity_by_non_unique_public_key_hash
        {
            0 | 1 => Self::verify_legacy_identity_by_non_unique_public_key_hash_checked(
                index_proof,
                identity_proof,
                public_key_hash,
                after,
                query_platform_version,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_legacy_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;

    #[test]
    fn test_verify_full_identity_by_non_unique_public_key_hash_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_non_unique_public_key_hash = 255;

        let proof = vec![];

        let result = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            [0u8; 20],
            None,
            &platform_version,
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_full_identity_by_non_unique_public_key_hash" && known_versions == vec![0, 1] && received == 255
            )
        );
    }
}
