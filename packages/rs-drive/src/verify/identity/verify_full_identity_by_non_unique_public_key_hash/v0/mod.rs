use crate::drive::Drive;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use crate::error::proof::ProofError;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the full identity of a user using their non-unique public key hash.
    ///
    /// This function performs a two-step verification process:
    /// 1. It verifies the identity ID associated with the given public key hash
    ///    by calling [`verify_identity_id_by_non_unique_public_key_hash()`].
    /// 2. If an identity ID is found, it then verifies the full identity by calling
    ///    [`verify_full_identity_by_identity_id()`].
    ///
    /// # Arguments
    ///
    /// * `proof` - A proof containing both the identity proof (if applicable) and
    ///   the proof linking the public key hash to an identity ID.
    /// * `public_key_hash` - A 20-byte array representing the hash of the user's public key.
    /// * `after` - An optional 32-byte array used to specify a search point in the proof verification process.
    /// * `platform_version` - A reference to the platform version, ensuring compatibility.
    ///
    /// # Returns
    ///
    /// If verification is successful, returns a `Result` containing:
    /// - `RootHash` - The root hash of GroveDB after verification.
    /// - `Option<Identity>` - The full identity of the user, if it exists.
    ///
    /// If no identity is found, the returned `Option<Identity>` will be `None`.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    /// * The provided proof is invalid.
    /// * The public key hash does not correspond to a valid identity ID.
    /// * The identity ID exists but the associated identity proof is missing.
    /// * The identity verification process fails.
    ///
    /// # Inline Optimization
    ///
    /// This function is marked with `#[inline(always)]` to hint the compiler to
    /// aggressively inline it for performance optimization.
    #[inline(always)]
    pub(super) fn verify_full_identity_by_non_unique_public_key_hash_v0(
        proof: &IdentityAndNonUniquePublicKeyHashDoubleProof,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let (root_hash, identity_id) = Self::verify_identity_id_by_non_unique_public_key_hash(
            &proof.identity_id_public_key_hash_proof,
            false,
            public_key_hash,
            after,
            platform_version,
        )?;
        let maybe_identity = identity_id
            .map(|identity_id| {
                let Some(identity_proof) = &proof.identity_proof else {
                    return Err(Error::Proof(ProofError::IncompleteProof("identity is not in proof even though identity id is set from non unique public key hash")));
                };

                Self::verify_full_identity_by_identity_id(
                    identity_proof.as_slice(),
                    false,
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
