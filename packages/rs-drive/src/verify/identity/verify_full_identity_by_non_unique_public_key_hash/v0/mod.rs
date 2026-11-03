use crate::drive::balances::balance_path;
use crate::drive::identity::{identity_key_tree_path, identity_path, IdentityRootStructure};
use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    /// Verifies one read-only lookup composition without changing state or consensus execution.
    pub(super) fn verify_full_identity_by_non_unique_public_key_hash_v0(
        proof: &[u8],
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        require_supported_grovedb_proof_envelope(proof, "non-unique identity proof")?;
        let grove_version = &platform_version.drive.grove_version;
        let index_path = non_unique_key_hashes_sub_tree_path_vec(public_key_hash);
        let mut index_query =
            Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after);
        index_query.query.limit = Some(1);
        // The candidate only reconstructs the query. Strict completeness below is authoritative.
        let (bootstrap_root, rows) =
            GroveDb::verify_subset_query(proof, &index_query, grove_version)?;
        let candidate: Option<[u8; 32]> = match rows.as_slice() {
            [] => None,
            [(path, key, Some(Element::Item(value, _)))]
                if path == &index_path && value.is_empty() =>
            {
                Some(key.as_slice().try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "non-unique index ID must be 32 bytes",
                    ))
                })?)
            }
            _ => return Err(Error::Proof(ProofError::CorruptedProof(
                "non-unique index proof must contain zero or one empty item at the requested path"
                    .to_string(),
            ))),
        };
        let query = match candidate {
            Some(id) => Self::full_identity_with_non_unique_public_key_hash_query(
                public_key_hash,
                id,
                after,
                &query_platform_version.drive.grove_version,
            )?,
            None => Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after),
        };
        let (root, rows) = GroveDb::verify_query(proof, &query, grove_version)?;
        if root != bootstrap_root {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "non-unique lookup verification roots differ".to_string(),
            )));
        }
        let balance_path = balance_path();
        let identity_paths = candidate
            .as_ref()
            .map(|id| (identity_path(id), identity_key_tree_path(id)));
        let mut authoritative_ids = Vec::new();
        for (path, key, element) in rows {
            if path == index_path {
                if !matches!(element, Some(Element::Item(ref value, _)) if value.is_empty()) {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "non-unique index entry must be an empty item".to_string(),
                    )));
                }
                let id: [u8; 32] = key.try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "non-unique index ID must be 32 bytes",
                    ))
                })?;
                authoritative_ids.push(id);
            } else if let (Some(id), Some((identity_path, identity_keys_path))) =
                (candidate, identity_paths)
            {
                let balance = path == balance_path && key == id;
                let revision = path == identity_path
                    && key.as_slice() == [IdentityRootStructure::IdentityTreeRevision as u8];
                let keys = path == identity_keys_path;
                if !balance && !revision && !keys {
                    return Err(Error::Proof(ProofError::TooManyElements(
                        "non-unique lookup proved an unrelated identity component",
                    )));
                }
            } else {
                return Err(Error::Proof(ProofError::TooManyElements(
                    "non-unique absence proof included an unrelated component",
                )));
            }
        }
        if authoritative_ids.as_slice() != candidate.as_slice() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "non-unique lookup must prove exactly its first eligible identity ID".to_string(),
            )));
        }
        let Some(id) = candidate else {
            return Ok((root, None));
        };
        // Reuse the existing decoder only for the authoritative ID and the same root.
        let (identity_root, identity) =
            Self::verify_full_identity_by_identity_id(proof, true, id, platform_version)?;
        if identity_root != root {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "full identity root differs from non-unique lookup root".to_string(),
            )));
        }
        let identity = identity.ok_or(Error::Proof(ProofError::IncompleteProof(
            "non-unique index references a missing full identity",
        )))?;
        Ok((root, Some(identity)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_full_identity_by_non_unique_public_key_hash() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

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

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                non_unique_key_hash,
                None,
                None,
                platform_version,
            )
            .expect("should not error when proving full identity by non-unique public key hash");

        let (_root_hash, proved_identity) =
            Drive::verify_full_identity_by_non_unique_public_key_hash(
                &proof,
                non_unique_key_hash,
                None,
                platform_version,
                platform_version,
            )
            .expect("expected to verify full identity by non-unique public key hash");

        assert_eq!(proved_identity, Some(identity));
    }

    #[test]
    fn should_prove_and_verify_absent_identity_by_non_unique_public_key_hash() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Use a key hash that is not associated with any identity
        let absent_key_hash = [0xFFu8; 20];

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                absent_key_hash,
                None,
                None,
                platform_version,
            )
            .expect("should not error when proving absent identity by non-unique public key hash");

        let (_root_hash, proved_identity) =
            Drive::verify_full_identity_by_non_unique_public_key_hash(
                &proof,
                absent_key_hash,
                None,
                platform_version,
                platform_version,
            )
            .expect("expected to verify absent identity by non-unique public key hash");

        assert_eq!(proved_identity, None);
    }
}
