mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

use std::iter::FromIterator;

impl Drive {
    /// Verifies the identity IDs of multiple identities by their public key hashes.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `public_key_hashes`: A slice of 20-byte arrays representing the public key hashes of the users.
    /// - `platform_version`: The platform version against which to verify the identity IDs.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// a generic collection `T` of tuples. Each tuple in `T` consists of a 20-byte array
    /// representing a public key hash and an `Option<[u8; 32]>`. The `Option<[u8; 32]>` represents
    /// the identity ID of the respective identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    ///
    pub fn verify_identity_ids_by_unique_public_key_hashes<
        T: FromIterator<([u8; 20], Option<[u8; 32]>)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hashes: &[[u8; 20]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_ids_by_unique_public_key_hashes
        {
            0 => Self::verify_identity_ids_by_unique_public_key_hashes_v0(
                proof,
                is_proof_subset,
                public_key_hashes,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_ids_by_unique_public_key_hashes".to_string(),
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
    fn test_verify_identity_ids_by_unique_public_key_hashes_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_ids_by_unique_public_key_hashes = 255;

        let result = Drive::verify_identity_ids_by_unique_public_key_hashes::<
            Vec<([u8; 20], Option<[u8; 32]>)>,
        >(&[], false, &[], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identity_ids_by_unique_public_key_hashes" && known_versions == vec![0] && received == 255
            )
        );
    }
}
