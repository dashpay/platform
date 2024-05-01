use crate::drive::Drive;

use crate::error::Error;

use crate::drive::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the full identity of a user by their public key hash.
    ///
    /// This function takes a byte slice `proof` and a 20-byte array `public_key_hash` as arguments,
    /// then it verifies the identity of the user with the given public key hash.
    ///
    /// The `proof` should contain the proof of authentication from the user.
    /// The `public_key_hash` should contain the hash of the public key of the user.
    ///
    /// The function first verifies the identity ID associated with the given public key hash
    /// by calling `verify_identity_id_by_public_key_hash()`. It then uses this identity ID to verify
    /// the full identity by calling `verify_full_identity_by_identity_id()`.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `Identity`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<Identity>` represents the full identity of the user if it exists.
    ///
    /// If the verification fails at any point, it will return an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    ///
    /// * The proof of authentication is not valid.
    /// * The public key hash does not correspond to a valid identity ID.
    /// * The identity ID does not correspond to a valid full identity.
    ///
    #[inline(always)]
    pub(super) fn verify_full_identity_by_public_key_hash_v0(
        proof: &[u8],
        public_key_hash: [u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let (root_hash, identity_id) = Self::verify_identity_id_by_public_key_hash(
            proof,
            true,
            public_key_hash,
            platform_version,
        )?;
        let maybe_identity = identity_id
            .map(|identity_id| {
                Self::verify_full_identity_by_identity_id(
                    proof,
                    true,
                    identity_id,
                    platform_version,
                )
                .map(|(_, maybe_identity)| maybe_identity)
            })
            .transpose()?
            .flatten();
        Ok((root_hash, maybe_identity))
    }
}
