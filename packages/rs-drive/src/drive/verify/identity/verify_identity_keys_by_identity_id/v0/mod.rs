use crate::drive::identity::identity_key_tree_path;
use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
pub use dpp::prelude::{Identity, Revision};
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies the identity keys of a user by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `PartialIdentity`. The `RootHash` represents the root hash of GroveDB,
    /// and the `Option<PartialIdentity>` represents the partial identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid partial identity.
    /// - The keys information is missing or incorrect.
    ///
    pub(super) fn verify_identity_keys_by_identity_id_v0(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
        _platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<PartialIdentity>), Error> {
        let key_request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let path_query = key_request.into_path_query();
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        let mut keys = BTreeMap::<KeyID, IdentityPublicKey>::new();
        let identity_keys_path = identity_key_tree_path(identity_id.as_slice());
        for proved_key_value in proved_key_values {
            let (path, _key, maybe_element) = proved_key_value;
            if path == identity_keys_path {
                if let Some(element) = maybe_element {
                    let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                    let key = IdentityPublicKey::deserialize_from_bytes(&item_bytes)?;
                    keys.insert(key.id(), key);
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we received an absence proof for a key but didn't request one",
                    )));
                }
            } else {
                return Err(Error::Proof(ProofError::TooManyElements(
                    "we got back items that we did not request",
                )));
            }
        }
        let maybe_identity = if keys.is_empty() {
            Ok::<Option<PartialIdentity>, Error>(None)
        } else {
            Ok(Some(PartialIdentity {
                id: Identifier::from(identity_id),
                balance: None,
                revision: None,
                loaded_public_keys: keys,
                not_found_public_keys: Default::default(),
            }))
        }?;
        Ok((root_hash, maybe_identity))
    }
}
