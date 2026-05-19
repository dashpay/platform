use crate::drive::balances::balance_path;
use crate::drive::identity::{identity_key_tree_path, identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};

use dpp::prelude::Revision;
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
    #[inline(always)]
    pub(super) fn verify_identity_keys_by_identity_id_v0(
        proof: &[u8],
        key_request: IdentityKeysRequest,
        with_revision: bool,
        with_balance: bool,
        is_proof_subset: bool,
        platform_version: &PlatformVersion,
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

        let path_query = PathQuery::merge(path_queries, &platform_version.drive.grove_version)?;
        let (root_hash, proved_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
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
                    let item_bytes = element.into_item_bytes().map_err(Error::from)?;
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
                    let item_bytes = element.into_item_bytes().map_err(Error::from)?;
                    revision = Some(Revision::from_be_bytes(
                        item_bytes.as_slice().try_into().map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::identity::key::fetch::KeyRequestType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;

    #[test]
    fn should_prove_and_verify_identity_keys() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(5, Some(14), platform_version)
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

        let key_request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let proof = drive
            .prove_identity_keys(key_request.clone(), None, platform_version)
            .expect("should not error when proving identity keys");

        let (_root_hash, proved_partial_identity) = Drive::verify_identity_keys_by_identity_id(
            proof.as_slice(),
            key_request,
            false, // with_revision
            false, // with_balance
            false, // is_proof_subset
            platform_version,
        )
        .expect("expected to verify identity keys");

        let partial_identity = proved_partial_identity.expect("expected a partial identity");

        assert_eq!(partial_identity.id, identity.id());
        assert_eq!(partial_identity.loaded_public_keys, *identity.public_keys());
        assert_eq!(partial_identity.balance, None);
        assert_eq!(partial_identity.revision, None);
    }
}
