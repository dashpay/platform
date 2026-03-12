mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the identity ID of a user by their public key hash.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `public_key_hash`: A 20-byte array representing the hash of the public key of the user.
    /// - `platform_version`: The platform version against which to verify the identity ID.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of a 32-byte array. The `RootHash` represents the root hash of GroveDB,
    /// and the `Option<[u8; 32]>` represents the identity ID of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    ///
    pub fn verify_identity_id_by_unique_public_key_hash(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_id_by_unique_public_key_hash
        {
            0 => Self::verify_identity_id_by_unique_public_key_hash_v0(
                proof,
                is_proof_subset,
                public_key_hash,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_id_by_unique_public_key_hash".to_string(),
                known_versions: vec![0],
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
    fn test_verify_identity_id_by_unique_public_key_hash_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_id_by_unique_public_key_hash = 255;

        let result = Drive::verify_identity_id_by_unique_public_key_hash(
            &[],
            false,
            [0u8; 20],
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identity_id_by_unique_public_key_hash" && known_versions == vec![0] && received == 255
            )
        );
    }
}
