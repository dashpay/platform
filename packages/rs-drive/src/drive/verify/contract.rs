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
use crate::error::query::QuerySyntaxError;
use grovedb::{GroveDb, PathQuery};

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
    pub fn verify_contract(
        proof: &[u8],
        contract_known_keeps_history: Option<bool>,
        is_proof_subset: bool,
        contract_id: [u8; 32],
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
                    Self::verify_contract(proof, Some(true), is_proof_subset, contract_id)
                } else {
                    Err(e)
                };
            }
        };
        if proved_key_values.is_empty() {
            return Ok((root_hash, None));
        }
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if contract_known_keeps_history.unwrap_or_default() {
                if path != contract_keeping_history_storage_path(&contract_id) {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct path for the historical contract",
                    )));
                }
            } else if path != contract_root_path(&contract_id) {
                if key != vec![0] {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct key for the contract",
                    )));
                }
                return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct path for the historical contract",
                    )));
            };

            let contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            DataContract::deserialize_no_limit(&bytes).map_err(Error::Protocol)
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

    /// Verifies that the contracts is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `is_proof_subset`: A boolean indicating whether to verify a subset of a larger proof.
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
    pub fn verify_contracts<I>(
        proof: &[u8],
        is_proof_subset: bool,
        contract_ids_with_keeps_history: BTreeMap<[u8; 32], bool>,
    ) -> Result<(RootHash, BTreeMap<[u8; 32], Option<DataContract>>), Error> {
        let request_len = contract_ids_with_keeps_history.len();

        if request_len == 0 {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "we did not get back an element for the correct path for the historical contract",
            )));
        }

        let path_queries: Vec<PathQuery> = contract_ids_with_keeps_history
            .into_iter()
            .map(|(contract_id, keeps_history)| {
                if keeps_history {
                    Self::fetch_contract_with_history_latest_query(contract_id)
                } else {
                    Self::fetch_contract_query(contract_id)
                }
            })
            .collect();

        let merged_path_query = PathQuery::merge(path_queries.iter().collect())?;

        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(proof, &merged_path_query)
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &merged_path_query)
        }?;
        if proved_key_values.len() != request_len {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we did not get back the number of elements we are looking for",
            )));
        }

        let contracts = proved_key_values.into_iter().map(|(path, key, maybe_element) | {
            let last_part = path.last().ok_or(Error::Proof(ProofError::CorruptedProof(
                "path of a proved item was empty",
            )))?;
            let (contract_id, contract_keeps_history) = if last_part.len() == 32 { // non history
                let contract_id : [u8;32] = last_part.clone().try_into().expect("expected 32 bytes");
                (contract_id, true)
            } else {
                if path.len() == 0 {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "path of a proved item wasn't big enough",
                    )));
                }
                let before_last_part = path.get(path.len() - 1).ok_or(Error::Proof(ProofError::CorruptedProof(
                    "we got back an invalid proof, the path was empty",
                )))?;
                if before_last_part.len() != 32 {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "the contract id wasn't 32 bytes",
                    )));
                }
                // otherwise the key is the time and the previous to last member of the path is the contract id
                let before_last_part : [u8;32] = before_last_part.clone().try_into().expect("expected 32 bytes");
                (before_last_part, false)
            };
            if contract_keeps_history {
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

            let contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            DataContract::deserialize_no_limit(&bytes).map_err(Error::Protocol)
                        })
                })
                .transpose()?;
            Ok((root_hash, contract))
        }).collect::<Result<BTreeMap<[u8; 32], Option<DataContract>>, Error>>()?;

        Ok((root_hash, contracts))
    }

    /// Verifies that the contract's history is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `contract_id`: The contract's unique identifier.
    /// - `start_at_date`: The start date for the contract's history.
    /// - `limit`: An optional limit for the number of items to be retrieved.
    /// - `offset`: An optional offset for the items to be retrieved.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<BTreeMap<u64, DataContract>>`. The `Option<BTreeMap<u64, DataContract>>`
    /// represents a mapping from dates to contracts if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - The contract serialization fails.
    pub fn verify_contract_history(
        proof: &[u8],
        contract_id: [u8; 32],
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
    ) -> Result<(RootHash, Option<BTreeMap<u64, DataContract>>), Error> {
        let path_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        let (root_hash, mut proved_key_values) = GroveDb::verify_query(proof, &path_query)?;

        let mut contracts: BTreeMap<u64, DataContract> = BTreeMap::new();
        for (path, key, maybe_element) in proved_key_values.drain(..) {
            if path != contract_storage_path_vec(&contract_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the historical contract",
                )));
            }

            let date = decode::decode_u64(&key).map_err(|_| {
                Error::Drive(DriveError::CorruptedContractPath(
                    "contract key is not a valid u64",
                ))
            })?;

            let maybe_contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            DataContract::deserialize_no_limit(&bytes).map_err(Error::Protocol)
                        })
                })
                .transpose()?;

            if let Some(contract) = maybe_contract {
                contracts.insert(date, contract);
            } else {
                return Err(Error::Drive(DriveError::CorruptedContractPath(
                    "expected a contract at this path",
                )));
            }
        }

        Ok((root_hash, Some(contracts)))
    }
}
