use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies and retrieves full identities associated with a given public key hash.
    ///
    /// This function validates the provided proof to confirm the identity IDs corresponding to a non-unique public key hash
    /// and subsequently verifies the full identity data for each of those identity IDs.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The output collection type, which must implement `FromIterator<Identity>`.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice containing the proof for verifying the identities.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user's public key.
    /// - `limit`: An optional limit for the number of identities to retrieve.
    /// - `platform_version`: A reference to the `PlatformVersion`, which dictates the GroveDB version used for verification.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash from GroveDB after proof verification.
    /// - `T`: A collection of verified `Identity` instances, created from the retrieved identity IDs.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The provided proof is invalid or incomplete.
    /// - No full identity data is available for any of the retrieved identity IDs.
    /// - The proof includes incorrect paths or keys.
    ///
    pub(super) fn verify_full_identities_for_non_unique_public_key_hash_v0<
        T: FromIterator<Identity>,
    >(
        proof: &[u8],
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let (root_hash, identity_ids) = Self::verify_identity_ids_for_non_unique_public_key_hash(
            proof,
            true,
            public_key_hash,
            limit,
            platform_version,
        )?;
        let identities = identity_ids
            .into_iter()
            .map(|identity_id| {
                let identity = Self::verify_full_identity_by_identity_id(
                    proof,
                    true,
                    identity_id,
                    platform_version,
                )
                .map(|(_, maybe_identity)| maybe_identity)?;
                identity.ok_or(Error::Proof(ProofError::IncompleteProof(
                    "proof returned an identity id without identity information",
                )))
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, identities))
    }
}
