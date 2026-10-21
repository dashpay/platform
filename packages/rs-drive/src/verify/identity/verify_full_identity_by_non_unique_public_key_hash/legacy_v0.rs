use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};
use crate::error::{proof::ProofError, Error};
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    /// Verify separate index and full-identity proofs against one root.
    ///
    /// Query construction follows the producer; verification retains the caller's
    /// security floor. The returned root still requires quorum authentication.
    pub(super) fn verify_legacy_identity_by_non_unique_public_key_hash_v0(
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
                "legacy non-unique proof must bind its first eligible identity or prove absence"
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
                "legacy index and identity proof roots differ".to_owned(),
            )));
        }
        // Strict producer-query completeness above precedes the shared identity decoder.
        let (decoded_root, identity) =
            Self::verify_full_identity_by_identity_id(proof, true, id, platform_version)?;
        if decoded_root != root {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "legacy identity decoder root differs from index root".to_owned(),
            )));
        }
        let identity = identity.ok_or(Error::Proof(ProofError::IncompleteProof(
            "non-unique index references a missing full identity",
        )))?;
        Ok((root, Some(identity)))
    }
}
