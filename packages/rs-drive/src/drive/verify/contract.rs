use crate::drive::contract::paths::{
    contract_keeping_history_storage_path, contract_root_path, contract_root_path_vec,
    contract_storage_path_vec,
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
use dpp::dashcore::hashes::hex::ToHex;
use grovedb::{Element, GroveDb, PathQuery};

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
        let mut path_query = if contract_known_keeps_history.unwrap_or_default() {
            Self::fetch_contract_with_history_latest_query(contract_id)
        } else {
            Self::fetch_contract_query(contract_id)
        };

        path_query.query.limit = Some(1);

        let result = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(proof, &path_query)
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &path_query)
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
        if proved_key_values.len() == 0 {
            return Err(Error::Proof(ProofError::WrongElementCount(
                "expected one element (even if it is none)",
            )));
        }
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if contract_known_keeps_history.unwrap_or_default() {
                if path != contract_keeping_history_storage_path(&contract_id) {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct path for the historical contract".to_string(),
                    )));
                }
            } else if path != contract_root_path(&contract_id) {
                if key != vec![0] {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct key for the contract"
                            .to_string(),
                    )));
                }
                return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back an element for the correct path for the historical contract".to_string(),
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
                .transpose();
            match contract {
                Ok(contract) => Ok((root_hash, contract)),
                Err(e) => {
                    if contract_known_keeps_history == Some(true) {
                        // just return error
                        Err(e)
                    } else {
                        Self::verify_contract(proof, Some(true), is_proof_subset, contract_id)
                    }
                }
            }
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
    /// - `contract_ids_with_keeps_history` a BTreemap with keys being the contract ids we are looking
    ///     to search for, values being if they keep history. For this call we must know if they keep
    ///     history.
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
    pub fn verify_contracts(
        proof: &[u8],
        _is_proof_subset: bool, //this will be used later
        contract_ids: &[[u8; 32]],
    ) -> Result<(RootHash, BTreeMap<[u8; 32], Option<DataContract>>), Error> {
        let request_len = contract_ids.len();

        if request_len == 0 {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "we didn't query anything",
            )));
        }

        let mut contracts = BTreeMap::new();

        let mut returned_root_hash = None;

        for contract_id in contract_ids {
            let (root_hash, contract) = Self::verify_contract(proof, None, true, *contract_id)?;
            returned_root_hash = Some(root_hash);
            contracts.insert(*contract_id, contract);
        }

        // let mut contracts_query = Self::fetch_contracts_query(
        //     non_historical_contracts.as_slice(),
        //     historical_contracts.as_slice(),
        // )?;
        //
        // contracts_query.query.limit = Some(request_len as u16);
        //
        // //todo: we are currently not proving succintness, a new method is required in grovedb
        // let (root_hash, mut proved_key_values) = GroveDb::verify_subset_query_with_absence_proof(proof, &contracts_query)?;
        //
        // let contracts = proved_key_values.into_iter().map(|(path, key, maybe_element) | {
        //     let last_part = path.last().ok_or(Error::Proof(ProofError::CorruptedProof(
        //         "path of a proved item was empty".to_string(),
        //     )))?;
        //     let (contract_id, contract_keeps_history) = if last_part.len() == 32 { // non history
        //         let contract_id : [u8;32] = last_part.clone().try_into().expect("expected 32 bytes");
        //         (contract_id, false)
        //     } else {
        //         if path.len() == 0 {
        //             return Err(Error::Proof(ProofError::CorruptedProof(
        //                 "path of a proved item wasn't big enough".to_string(),
        //             )));
        //         }
        //         let before_last_part = path.get(path.len() - 2).ok_or(Error::Proof(ProofError::CorruptedProof(
        //             "we got back an invalid proof, the path was empty".to_string(),
        //         )))?;
        //         if before_last_part.len() != 32 {
        //             return Err(Error::Proof(ProofError::CorruptedProof(
        //                 "the contract id wasn't 32 bytes".to_string(),
        //             )));
        //         }
        //         // otherwise the key is the time and the previous to last member of the path is the contract id
        //         let before_last_part : [u8;32] = before_last_part.clone().try_into().expect("expected 32 bytes");
        //         (before_last_part, true)
        //     };
        //     if contract_keeps_history {
        //         if path != contract_keeping_history_storage_path(&contract_id) {
        //             return Err(Error::Proof(ProofError::CorruptedProof(
        //                 format!("we did not get back an element for the correct path for the historical contract, received: ({})", path.iter().map(|a| a.to_hex()).collect::<Vec<_>>().join("|")),
        //             )));
        //         }
        //     } else if path != contract_root_path(&contract_id) {
        //         return Err(Error::Proof(ProofError::CorruptedProof(
        //             format!("we did not get back an element for the correct path for the non historical contract, received: ({})", path.iter().map(|a| a.to_hex()).collect::<Vec<_>>().join("|")),
        //         )));
        //     };
        //
        //     let contract = maybe_element
        //         .map(|element| {
        //             element
        //                 .into_item_bytes()
        //                 .map_err(Error::GroveDB)
        //                 .and_then(|bytes| {
        //                     DataContract::deserialize_no_limit(&bytes).map_err(Error::Protocol)
        //                 })
        //         })
        //         .transpose()?;
        //     Ok((root_hash, contract))
        // }).collect::<Result<BTreeMap<[u8; 32], Option<DataContract>>, Error>>()?;

        Ok((returned_root_hash.unwrap(), contracts))
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
                    "we did not get back an element for the correct path for the historical contract".to_string(),
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
