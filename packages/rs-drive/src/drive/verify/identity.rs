use crate::drive::balances::balance_path;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_key_tree_path, identity_path};
use crate::drive::{unique_key_hashes_tree_path_vec, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::fee::credits::Credits;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
pub use dpp::prelude::{Identity, Revision};
use dpp::serialization_traits::PlatformDeserializable;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies the full identity of a user by their public key hash.
    ///
    /// This function takes a byte slice `proof` and a 20-byte array `public_key_hash` as arguments,
    /// then it verifies the identity of the user with the given public key hash.
    ///
    /// The `proof` should contain the proof of authentication from the user.
    /// The `public_key_hash` should contain the hash of the public key of the user.
    ///
    /// The function first verifies the identity ID associated with the given public key hash
    /// by calling `verify_identity_id_by_public_key_hash()`. It then uses this identity ID to verify
    /// the full identity by calling `verify_full_identity_by_identity_id()`.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `Identity`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<Identity>` represents the full identity of the user if it exists.
    ///
    /// If the verification fails at any point, it will return an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    ///
    /// * The proof of authentication is not valid.
    /// * The public key hash does not correspond to a valid identity ID.
    /// * The identity ID does not correspond to a valid full identity.
    ///
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

    /// Verifies the full identities of multiple users by their public key hashes.
    ///
    /// This function is a generalization of `verify_full_identity_by_public_key_hash`,
    /// which works with a slice of public key hashes instead of a single hash.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the users.
    /// - `public_key_hashes`: A reference to a slice of 20-byte arrays, each representing
    ///    a hash of a public key of a user.
    ///
    /// # Generic Parameters
    ///
    /// - `T`: The type of the collection to hold the results, which must be constructible
    ///    from an iterator of tuples of a 20-byte array and an optional `Identity`.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and `T`.
    /// The `RootHash` represents the root hash of GroveDB, and `T` represents
    /// the collection of the public key hash and associated identity (if it exists) for each user.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the public key hashes do not correspond to a valid identity ID.
    /// - Any of the identity IDs do not correspond to a valid full identity.
    ///
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
    pub fn verify_full_identity_by_identity_id(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
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
                        "balance wasn't for the identity requested",
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
    pub fn verify_identity_keys_by_identity_id(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
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
        if proved_key_values.is_empty() {
            return Ok((root_hash, None));
        }
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
                "expected maximum one identity id",
            )))
        }
    }

    /// Verifies the balance of an identity by their identity ID.
    ///
    /// `verify_subset_of_proof` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the balance and the revision, but here we are only interested
    /// in verifying the balance.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `verify_subset_of_proof`: A boolean indicating whether we are verifying a subset of a larger proof.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option<u64>`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<u64>` represents the balance of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid balance.
    /// - The proved key value is not for the correct path or key in balances.
    /// - More than one balance is found.
    ///
    pub fn verify_identity_balance_for_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
        verify_subset_of_proof: bool,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let path_query = Self::identity_balance_query(&identity_id);
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };

        if proved_key_values.len() == 0 {
            Ok((root_hash, None))
        } else if proved_key_values.len() == 1 {
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

    /// Verifies the balances of multiple identities by their identity IDs.
    ///
    /// `is_proof_subset` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the balances and revisions, but here we are only
    /// interested in verifying the balances.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `identity_ids`: A slice of 32-byte arrays representing the identity IDs of the users.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// a generic collection `T` of tuples. Each tuple in `T` consists of a 32-byte array
    /// representing an identity ID and an `Option<Credits>`. The `Option<Credits>` represents
    /// the balance of the respective identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the identity IDs does not correspond to a valid balance.
    /// - The number of proved key values does not match the number of identity IDs provided.
    /// - The value size of the balance is incorrect.
    ///
    pub fn verify_identity_balances_for_identity_ids<
        T: FromIterator<([u8; 32], Option<Credits>)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        identity_ids: &[[u8; 32]],
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::balances_for_identity_ids_query(identity_ids)?;
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };
        if proved_key_values.len() == identity_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 32] = proved_key_value
                        .1
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let maybe_element = proved_key_value.2;
                    match maybe_element {
                        None => Ok((key, None)),
                        Some(element) => {
                            let balance: Credits = element
                                .as_sum_item_value()
                                .map_err(Error::GroveDB)?
                                .try_into()
                                .map_err(|_| {
                                    Error::Proof(ProofError::IncorrectValueSize(
                                        "balance was negative",
                                    ))
                                })?;
                            Ok((key, Some(balance)))
                        }
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount(
                "expected same count as elements requested",
            )))
        }
    }

    /// Verifies the identity IDs of multiple identities by their public key hashes.
    ///
    /// `is_proof_subset` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the identity IDs and revisions, but here we are only
    /// interested in verifying the identity IDs.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `public_key_hashes`: A slice of 20-byte arrays representing the public key hashes of the users.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// a generic collection `T` of tuples. Each tuple in `T` consists of a 20-byte array
    /// representing a public key hash and an `Option<[u8; 32]>`. The `Option<[u8; 32]>` represents
    /// the identity ID of the respective identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the public key hashes does not correspond to a valid identity ID.
    /// - The number of proved key values does not match the number of public key hashes provided.
    /// - The value size of the identity ID is incorrect.
    ///
    pub fn verify_identity_ids_by_public_key_hashes<
        T: FromIterator<([u8; 20], Option<[u8; 32]>)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hashes: &[[u8; 20]],
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        let (root_hash, proved_key_values) = if is_proof_subset {
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
