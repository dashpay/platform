use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};
use crate::error::{proof::ProofError, Error};
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    /// Verifies separate index and full-identity proofs against one root. Serves
    /// request V0; query-only, never reached from block execution.
    ///
    /// Query construction follows the producer; verification retains the caller's
    /// security floor. The returned root still requires quorum authentication.
    pub(super) fn verify_full_identity_by_non_unique_public_key_hash_two_proofs_v0(
        index_proof: &[u8],
        identity_proof: Option<&[u8]>,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        require_supported_grovedb_proof_envelope(index_proof, "non-unique index proof")?;
        if let Some(proof) = identity_proof {
            require_supported_grovedb_proof_envelope(proof, "full identity proof")?;
        }
        let grove_version = &platform_version.drive.grove_version;
        let mut query =
            Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after);
        query.query.limit = Some(1);
        let (root, rows) = GroveDb::verify_query(index_proof, &query, grove_version)?;
        let id: [u8; 32] = match rows.as_slice() {
            [] if identity_proof.is_none() => return Ok((root, None)),
            [(path, key, Some(Element::Item(value, _)))]
                if path == &non_unique_key_hashes_sub_tree_path_vec(public_key_hash)
                    && value.is_empty() =>
            {
                key.as_slice().try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "non-unique index ID must be 32 bytes",
                    ))
                })?
            }
            _ => return Err(Error::Proof(ProofError::CorruptedProof(
                "two-proof non-unique answer must bind its first eligible identity or prove absence"
                    .to_owned(),
            ))),
        };
        let proof = identity_proof.ok_or(Error::Proof(ProofError::IncompleteProof(
            "non-unique index references an identity without its proof",
        )))?;
        let query = Self::full_identity_query(&id, &query_platform_version.drive.grove_version)?;
        let (identity_root, _) = GroveDb::verify_query(proof, &query, grove_version)?;
        if identity_root != root {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "two-proof index and identity proof roots differ".to_owned(),
            )));
        }
        // Strict producer-query completeness above precedes the shared identity decoder.
        // The decoder rebuilds both queries from the tables it is given; the producer's
        // tables make it read exactly the queries checked above, whose verification
        // already applied this verifier's GroveDB version to the same bytes.
        let (decoded_root, identity) =
            Self::verify_full_identity_by_non_unique_public_key_hash_two_proofs_unguarded(
                &IdentityAndNonUniquePublicKeyHashDoubleProof {
                    identity_id_public_key_hash_proof: index_proof.to_vec(),
                    identity_proof: Some(proof.to_vec()),
                },
                public_key_hash,
                after,
                query_platform_version,
            )?;
        if decoded_root != root {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "two-proof identity decoder root differs from index root".to_owned(),
            )));
        }
        let identity = identity.ok_or(Error::Proof(ProofError::IncompleteProof(
            "non-unique index references a missing full identity",
        )))?;
        // The decoder read the identity under the producer's tables; it must be the one
        // the strictly checked index entry names.
        if identity.id().to_buffer() != id {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "two-proof identity differs from the identity its index entry names".to_owned(),
            )));
        }
        Ok((root, Some(identity)))
    }

    /// Verifies the full identity of a user using their non-unique public key hash.
    ///
    /// Both inner `verify_query` calls are strict, but this decoder does not check
    /// the answer's shape or how the two proofs pair up: it ignores the index row's
    /// value instead of requiring an empty Item, it returns absence even when an
    /// identity proof is attached, and it does not check that the decoded identity
    /// has the ID the index entry names. The guarded verifier above makes those
    /// checks and only uses this function to assemble the identity.
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
    fn verify_full_identity_by_non_unique_public_key_hash_two_proofs_unguarded(
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

                let (identity_root_hash, maybe_identity) =
                    Self::verify_full_identity_by_identity_id(
                        identity_proof.as_slice(),
                        false,
                        identity_id,
                        platform_version,
                    )?;

                // Both proofs must come from the same tree state. If the root
                // hashes differ, the identity proof may be from a stale or
                // fabricated state that does not match the quorum-signed root.
                if identity_root_hash != root_hash {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "identity proof root hash does not match the public key hash proof root hash".to_string(),
                    )));
                }

                Ok(maybe_identity)
            })
            .transpose()?
            .flatten();
        Ok((root_hash, maybe_identity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;

    /// Request V0 is served at every protocol version.
    fn served_versions() -> [&'static PlatformVersion; 2] {
        [
            PlatformVersion::get(13).expect("protocol 13"),
            PlatformVersion::latest(),
        ]
    }

    fn prove_and_verify(
        drive: &Drive,
        key_hash: [u8; 20],
        platform_version: &PlatformVersion,
    ) -> Option<Identity> {
        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                key_hash,
                None,
                None,
                platform_version,
            )
            .expect("should not error when proving full identity by non-unique public key hash");

        Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs_v0(
            &proof.identity_id_public_key_hash_proof,
            proof.identity_proof.as_deref(),
            key_hash,
            None,
            platform_version,
            platform_version,
        )
        .expect("expected to verify full identity by non-unique public key hash")
        .1
    }

    #[test]
    fn should_prove_and_verify_full_identity_by_non_unique_public_key_hash() {
        for platform_version in served_versions() {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add an identity");

            // Find a non-unique key hash from the identity
            let non_unique_key_hash = identity
                .public_keys()
                .values()
                .find(|pk| !pk.key_type().is_unique_key_type())
                .expect("expected a non-unique key")
                .public_key_hash()
                .expect("expected to hash data");

            assert_eq!(
                prove_and_verify(&drive, non_unique_key_hash, platform_version),
                Some(identity),
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    #[test]
    fn should_prove_and_verify_absent_identity_by_non_unique_public_key_hash() {
        for platform_version in served_versions() {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            // Use a key hash that is not associated with any identity
            assert_eq!(
                prove_and_verify(&drive, [0xFFu8; 20], platform_version),
                None,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
