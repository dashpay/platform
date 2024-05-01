use crate::drive::balances::balance_path;

use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_key_tree_path, identity_path};
use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, IdentityV0, KeyID};
pub use dpp::prelude::{Identity, Revision};
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies the full identity of a user by their identity ID.
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
    /// an `Option` of `Identity`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<Identity>` represents the full identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid full identity.
    /// - The balance, revision, or keys information is missing or incorrect.
    ///
    #[inline(always)]
    pub(super) fn verify_full_identity_by_identity_id_v0(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
        _platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let path_query = Self::full_identity_query(&identity_id)?;
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        let mut balance = None;
        let mut revision = None;
        let mut keys = BTreeMap::<KeyID, IdentityPublicKey>::new();
        let balance_path = balance_path();
        let identity_path = identity_path(identity_id.as_slice());
        let identity_keys_path = identity_key_tree_path(identity_id.as_slice());
        for proved_key_value in proved_key_values {
            let (path, key, maybe_element) = proved_key_value;
            if path == balance_path {
                if key == identity_id {
                    if let Some(element) = maybe_element {
                        //this is the balance
                        let signed_balance = element.as_sum_item_value().map_err(Error::GroveDB)?;
                        if signed_balance < 0 {
                            return Err(Error::Proof(ProofError::Overflow(
                                "balance can't be negative",
                            )));
                        }
                        balance = Some(signed_balance as u64);
                        continue;
                    } else {
                        return Err(Error::Proof(ProofError::IncompleteProof(
                            "balance wasn't provided for the identity requested",
                        )));
                    }
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "balance wasn't for the identity requested".to_string(),
                    )));
                }
            } else if path == identity_path && key == vec![IdentityTreeRevision as u8] {
                if let Some(element) = maybe_element {
                    let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                    //this is the revision
                    revision = Some(Revision::from_be_bytes(item_bytes.try_into().map_err(
                        |_| {
                            Error::Proof(ProofError::IncorrectValueSize(
                                "revision should be 8 bytes",
                            ))
                        },
                    )?));
                    continue;
                } else {
                    return Err(Error::Proof(ProofError::IncompleteProof(
                        "revision wasn't provided for the identity requested",
                    )));
                }
            } else if path == identity_keys_path {
                if let Some(element) = maybe_element {
                    let item_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                    let key = IdentityPublicKey::deserialize_from_bytes(&item_bytes)?;
                    keys.insert(key.id(), key);
                } else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we received an absence proof for a key but didn't request one".to_string(),
                    )));
                }
            } else {
                return Err(Error::Proof(ProofError::TooManyElements(
                    "we got back items that we did not request",
                )));
            }
        }
        let maybe_identity = if balance.is_none() && revision.is_none() && keys.is_empty() {
            Ok(None)
        } else if balance.is_none() || revision.is_none() || keys.is_empty() {
            // that means that one has stuff and the others don't
            // this is an error
            Err(Error::Proof(ProofError::IncompleteProof(
                "identity proof is incomplete",
            )))
        } else {
            Ok(Some(
                IdentityV0 {
                    id: Identifier::from(identity_id),
                    public_keys: keys,
                    balance: balance.unwrap(),
                    revision: revision.unwrap(),
                }
                .into(),
            ))
        }?;
        Ok((root_hash, maybe_identity))
    }
}
