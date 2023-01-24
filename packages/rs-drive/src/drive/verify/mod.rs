use crate::drive::balances::{balance_path, balance_path_vec};
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_key_tree_path, identity_path};
use crate::drive::{identity_tree_path, unique_key_hashes_tree_path_vec, Drive};
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Identity, Revision};
use grovedb::operations::proof::verify::ProvedKeyValue;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{Element, GroveDb};
use std::collections::BTreeMap;

pub type RootHash = [u8; 32];

impl Drive {
    /// Verifies the identity with a public key hash
    pub fn verify_full_identity_by_public_key_hash(
        proof: &[u8],
        public_key_hash: [u8; 20],
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let (root_hash, identity_id) =
            Self::verify_identity_id_by_public_key_hash(proof, true, public_key_hash)?;
        let maybe_identity = identity_id
            .map(|identity_id| {
                Self::verify_full_identity_by_identity_id(proof, true, identity_id)
                    .map(|(_, maybe_identity)| maybe_identity)
            })
            .transpose()?
            .flatten();
        Ok((root_hash, maybe_identity))
    }

    /// Verifies the identity with a public key hash
    pub fn verify_full_identities_by_public_key_hashes<
        T: FromIterator<([u8; 20], Option<Identity>)>,
    >(
        proof: &[u8],
        public_key_hashes: &[[u8; 20]],
    ) -> Result<(RootHash, T), Error> {
        let (root_hash, identity_ids_by_key_hashes) =
            Self::verify_identity_ids_by_public_key_hashes::<Vec<(_, _)>>(
                proof,
                true,
                public_key_hashes,
            )?;
        let maybe_identity = identity_ids_by_key_hashes
            .into_iter()
            .map(|(key_hash, identity_id)| match identity_id {
                None => Ok((key_hash, None)),
                Some(identity_id) => {
                    let identity =
                        Self::verify_full_identity_by_identity_id(proof, true, identity_id)
                            .map(|(_, maybe_identity)| maybe_identity)?;
                    let identity = identity.ok_or(Error::Proof(ProofError::IncompleteProof(
                        "proof returned an identity id without identity information",
                    )))?;
                    Ok((key_hash, Some(identity)))
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, maybe_identity))
    }

    /// Verifies the identity with its identity id
    pub fn verify_full_identity_by_identity_id(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
    ) -> Result<(RootHash, Option<Identity>), Error> {
        let path_query = Self::full_identity_query(&identity_id)?;
        let (root_hash, mut proved_key_values) = if is_proof_subset {
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
                        "balance wasn't for the identity requested",
                    )));
                }
            } else {
                if path == identity_path && key == vec![IdentityTreeRevision as u8] {
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
                        let key = IdentityPublicKey::deserialize(&item_bytes)?;
                        keys.insert(key.id, key);
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
            Ok(Some(Identity {
                protocol_version: PROTOCOL_VERSION,
                id: Identifier::from(identity_id),
                public_keys: keys,
                balance: balance.unwrap(),
                revision: revision.unwrap(),
                asset_lock_proof: None,
                metadata: None,
            }))
        }?;
        Ok((root_hash, maybe_identity))
    }

    /// Verifies the identity id with a public key hash
    pub fn verify_identity_id_by_public_key_hash(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hash: [u8; 20],
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let path_query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if path != unique_key_hashes_tree_path_vec() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in unique key hashes",
                )));
            }
            if key != public_key_hash {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key in unique key hashes",
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
                "expected one identity id",
            )))
        }
    }

    /// Verifies the identity's balance with a identity id
    pub fn verify_identity_balance_for_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
    ) -> Result<(RootHash, Option<u64>), Error> {
        let path_query = Self::identity_balance_query(&identity_id);
        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = &proved_key_values.remove(0);
            if path != &balance_path() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in balances",
                )));
            }
            if key != &identity_id {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key in balances",
                )));
            }

            let signed_balance = maybe_element
                .as_ref()
                .map(|element| {
                    element
                        .as_sum_item_value()
                        .map_err(Error::GroveDB)?
                        .try_into()
                        .map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                        })
                })
                .transpose()?;
            Ok((root_hash, signed_balance))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one identity balance",
            )))
        }
    }

    /// Verifies the identity id with a public key hash
    pub fn verify_identity_ids_by_public_key_hashes<
        T: FromIterator<([u8; 20], Option<[u8; 32]>)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hashes: &[[u8; 20]],
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        if proved_key_values.len() == public_key_hashes.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 20] = proved_key_value
                        .1
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let maybe_element = proved_key_value.2;
                    match maybe_element {
                        None => Ok((key, None)),
                        Some(element) => {
                            let identity_id = element
                                .into_item_bytes()
                                .map_err(Error::GroveDB)?
                                .try_into()
                                .map_err(|_| {
                                    Error::Proof(ProofError::IncorrectValueSize(
                                        "value size is incorrect",
                                    ))
                                })?;
                            Ok((key, Some(identity_id)))
                        }
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount(
                "expected one identity id",
            )))
        }
    }
}
