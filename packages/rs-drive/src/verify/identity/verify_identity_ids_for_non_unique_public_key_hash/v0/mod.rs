use crate::drive::{non_unique_key_hashes_sub_tree_path, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies and retrieves identity IDs for a user based on their public key hash.
    ///
    /// This function verifies the provided proof to confirm the identity ID(s) associated with a non-unique public key hash.
    /// It ensures the proof is valid and matches the expected structure and path within GroveDB.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the authentication proof to verify the identity.
    /// - `is_proof_subset`: A boolean indicating whether the proof represents a subset query.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user's public key.
    /// - `limit`: An optional limit for the number of identity IDs to retrieve.
    /// - `platform_version`: A reference to the `PlatformVersion` which determines the GroveDB version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The GroveDB root hash for the verified proof.
    /// - `Vec<[u8; 32]>`: A vector of identity IDs (each as a 32-byte array) corresponding to the public key hash, if any are found.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The authentication proof is invalid or corrupted.
    /// - The public key hash does not map to any valid identity ID.
    /// - The proof includes incorrect paths or keys in the non-unique key hash tree.
    /// - Multiple identity IDs are found when only one was expected.
    ///
    #[inline(always)]
    pub(super) fn verify_identity_ids_for_non_unique_public_key_hash_v0(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<[u8; 32]>), Error> {
        let mut path_query =
            Self::identity_ids_for_non_unique_public_key_hash_query(public_key_hash);
        path_query.query.limit = Some(
            limit.unwrap_or(
                platform_version
                    .drive_abci
                    .query
                    .max_returned_full_identities,
            ),
        );
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let identity_ids = proved_key_values
            .into_iter()
            .map(|(path, key, _)| {
                if path != non_unique_key_hashes_sub_tree_path(&public_key_hash) {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in non unique key hashes"
                        .to_string(),
                )));
                }
                key.try_into().map_err(|_| {
                    Error::Proof(ProofError::CorruptedProof(
                        "key should be 32 bytes in non unique key hash tree".to_string(),
                    ))
                })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;

        Ok((root_hash, identity_ids))
    }
}
