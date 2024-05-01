use crate::drive::{unique_key_hashes_tree_path_vec, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::drive::verify::RootHash;

use grovedb::GroveDb;

impl Drive {
    /// Verifies the identity ID of a user by their public key hash.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `public_key_hash`: A 20-byte array representing the hash of the public key of the user.
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
    pub(super) fn verify_identity_id_by_public_key_hash_v0(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let mut path_query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        path_query.query.limit = Some(1);
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(proof, &path_query)?
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &path_query)?
        };

        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if path != unique_key_hashes_tree_path_vec() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in unique key hashes"
                        .to_string(),
                )));
            }
            if key != public_key_hash {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key in unique key hashes"
                        .to_string(),
                )));
            }
            let identity_id = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)?
                        .try_into()
                        .map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                        })
                })
                .transpose()?;
            Ok((root_hash, identity_id))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected maximum one identity id",
            )))
        }
    }
}
