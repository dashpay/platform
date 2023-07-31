use crate::drive::contract::paths::{
    contract_keeping_history_storage_path, contract_root_path, contract_storage_path_vec,
};
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use dpp::prelude::DataContract;
use std::collections::BTreeMap;

use crate::common::decode;
use crate::error::drive::DriveError;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    /// Verifies that the contract is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `contract_known_keeps_history`: An optional boolean indicating whether the contract keeps a history.
    /// - `is_proof_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `contract_id`: The contract's unique identifier.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<DataContract>`. The `Option<DataContract>`
    /// represents the verified contract if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub(super) fn verify_contract_v0(
        proof: &[u8],
        contract_known_keeps_history: Option<bool>,
        is_proof_subset: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<DataContract>), Error> {
        let path_query = if contract_known_keeps_history.unwrap_or_default() {
            Self::fetch_contract_with_history_latest_query(contract_id)
        } else {
            Self::fetch_contract_query(contract_id)
        };

        let result = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query)
        } else {
            GroveDb::verify_query(proof, &path_query)
        };
        let (root_hash, mut proved_key_values) = match result.map_err(GroveDB) {
            Ok(ok_result) => ok_result,
            Err(e) => {
                return if contract_known_keeps_history.is_none() {
                    // most likely we are trying to prove a historical contract
                    Self::verify_contract(
                        proof,
                        Some(true),
                        is_proof_subset,
                        contract_id,
                        platform_version,
                    )
                } else {
                    Err(e)
                };
            }
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if contract_known_keeps_history.unwrap_or_default() {
                if path != contract_keeping_history_storage_path(&contract_id) {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct path for the historical contract",
                    )));
                }
            } else if path != contract_root_path(&contract_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the historical contract",
                )));
            };

            if key != vec![0] {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key for the contract",
                )));
            }
            let contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            DataContract::versioned_deserialize(&bytes, platform_version)
                                .map_err(Error::Protocol)
                        })
                })
                .transpose()?;
            Ok((root_hash, contract))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one contract id",
            )))
        }
    }
}
