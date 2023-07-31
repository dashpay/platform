use crate::drive::balances::balance_path;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{identity_key_tree_path, identity_path};
use crate::drive::{unique_key_hashes_tree_path_vec, Drive};

use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::fee::Credits;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
pub use dpp::prelude::{Identity, Revision};
use dpp::serialization::PlatformDeserializable;
use grovedb::GroveDb;
use std::collections::BTreeMap;

impl Drive {
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
    pub(super) fn verify_identity_balances_for_identity_ids_v0<
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
}
