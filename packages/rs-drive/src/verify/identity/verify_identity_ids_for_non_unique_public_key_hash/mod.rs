mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies and retrieves identity IDs associated with a non-unique public key hash.
    ///
    /// This function checks the provided proof and confirms the validity of identity IDs for a given public key hash.
    /// It utilizes versioning to call the appropriate method based on the `PlatformVersion`.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to verify the identity data.
    /// - `is_proof_subset`: A boolean indicating if the proof represents a subset query.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user’s public key.
    /// - `limit`: An optional limit on the number of identity IDs to retrieve.
    /// - `platform_version`: A reference to the `PlatformVersion` struct, which selects the correct verification method version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash of GroveDB after proof verification.
    /// - `Vec<[u8; 32]>`: A vector of identity IDs (each as a 32-byte array) associated with the public key hash.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The provided proof is invalid or fails verification.
    /// - The public key hash does not correspond to any valid identity IDs.
    /// - The method version specified in `PlatformVersion` is not supported.
    ///
    pub fn verify_identity_ids_for_non_unique_public_key_hash(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<[u8; 32]>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_ids_for_non_unique_public_key_hash
        {
            0 => Self::verify_identity_ids_for_non_unique_public_key_hash_v0(
                proof,
                is_proof_subset,
                public_key_hash,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_ids_for_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
