use crate::drive::balances::{balance_path, balance_path_vec};
use crate::drive::identity::{identity_key_tree_path, identity_path_vec, IdentityRootStructure};
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
use grovedb::{GroveDb, PathQuery};
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
        key_request: IdentityKeysRequest,
        with_revision: bool,
        with_balance: bool,
        is_proof_subset: bool,
        _platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<PartialIdentity>), Error> {
        let identity_id = key_request.identity_id;
        let keys_path_query = key_request.into_path_query();
        let mut path_queries = vec![&keys_path_query];

        let revision_path_query = Drive::identity_revision_query(&identity_id);
        let balance_path_query = Drive::balance_for_identity_id_query(identity_id);

        if with_balance {
            path_queries.push(&balance_path_query);
        }
        if with_revision {
            path_queries.push(&revision_path_query);
        }

        let path_query = PathQuery::merge(path_queries)?;
        let (root_hash, proved_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };

        let mut loaded_public_keys = BTreeMap::<KeyID, IdentityPublicKey>::new();
        let mut balance = None;
        let mut revision = None;

        let identity_keys_path = identity_key_tree_path(identity_id.as_slice());
        let identity_balance_path = balance_path();
        let identity_path = identity_path_vec(&identity_id);

        for proved_key_value in proved_values {
            let (path, key, maybe_element) = proved_key_value;
            if path == identity_keys_path {
                if let Some(element) = maybe_element {
                    let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                    let key = IdentityPublicKey::deserialize_from_bytes(&item_bytes)?;
                    loaded_public_keys.insert(key.id(), key);
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we received an absence proof for a key but didn't request one".to_string(),
                    )));
                }
            } else if path == identity_balance_path && key == identity_id {
                if let Some(grovedb::Element::SumItem(identity_balance, _)) = maybe_element {
                    balance = Some(identity_balance as u64);
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "balance proof must be an existing sum item".to_string(),
                    )));
                }
            } else if path == identity_path
                && key == [IdentityRootStructure::IdentityTreeRevision as u8]
            {
                if let Some(element) = maybe_element {
                    let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                    revision = Some(Revision::from_be_bytes(
                        item_bytes.as_slice().try_into().map_err(|_| {
                            Error::GroveDB(grovedb::Error::WrongElementType(
                                "expecting 8 bytes of data for revision",
                            ))
                        })?,
                    ));
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we received an absence proof for a revision but didn't request one"
                            .to_string(),
                    )));
                }
            } else {
                return Err(Error::Proof(ProofError::TooManyElements(
                    "we got back items that we did not request",
                )));
            }
        }
        let maybe_identity = Some(PartialIdentity {
            id: Identifier::from(identity_id),
            balance,
            revision,
            loaded_public_keys,
            not_found_public_keys: Default::default(),
        });

        Ok((root_hash, maybe_identity))
    }
}
