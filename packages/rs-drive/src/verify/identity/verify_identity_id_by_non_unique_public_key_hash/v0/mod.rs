use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the identity ID of a user by their public key hash.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `public_key_hash`: A 20-byte array representing the hash of the public key of the user.
    /// - `after`: A 32 byte array representing an identity after which we want to get the identity id.
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
    /// - The proof of authentication is not valid.
    /// - The public key hash does not correspond to a valid identity ID.
    /// - The proved key value is not for the correct path or key in unique key hashes.
    /// - More than one identity ID is found.
    ///
    #[inline(always)]
    pub(super) fn verify_identity_id_by_non_unique_public_key_hash_v0(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let mut path_query =
            Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after);
        path_query.query.limit = Some(1);
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        if proved_key_values.len() == 1 {
            let (path, key, _) = proved_key_values.remove(0);
            if path != non_unique_key_hashes_sub_tree_path_vec(public_key_hash) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in non unique key hashes"
                        .to_string(),
                )));
            }
            let identity_id = key.try_into().map_err(|_| {
                Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
            })?;
            Ok((root_hash, Some(identity_id)))
        } else {
            Ok((root_hash, None))
        }
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
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_identity_id_by_non_unique_public_key_hash() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a random identity");

        let identity_id = identity.id().to_buffer();

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

        // Build a proof for the non-unique key hash query
        let mut path_query =
            Drive::identity_id_by_non_unique_public_key_hash_query(non_unique_key_hash, None);
        path_query.query.limit = Some(1);
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should not error when proving identity id by non-unique public key hash");

        let (_root_hash, proved_identity_id) =
            Drive::verify_identity_id_by_non_unique_public_key_hash(
                proof.as_slice(),
                false,
                non_unique_key_hash,
                None,
                platform_version,
            )
            .expect("expected to verify identity id by non-unique public key hash");

        assert_eq!(proved_identity_id, Some(identity_id));
    }
}
