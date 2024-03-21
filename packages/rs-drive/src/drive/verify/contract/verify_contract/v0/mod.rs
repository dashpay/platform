use std::collections::BTreeMap;

use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use dpp::prelude::DataContract;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use platform_version::version::PlatformVersion;

use crate::error::query::QuerySyntaxError;
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
    #[inline(always)]
    pub(super) fn verify_contract_v0(
        proof: &[u8],
        contract_known_keeps_history: Option<bool>,
        is_proof_subset: bool,
        in_multiple_contract_proof_form: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<DataContract>), Error> {
        let path_query = match (
            in_multiple_contract_proof_form,
            contract_known_keeps_history.unwrap_or_default(),
        ) {
            (true, true) => Self::fetch_historical_contracts_query(&[contract_id]),
            (true, false) => Self::fetch_non_historical_contracts_query(&[contract_id]),
            (false, true) => Self::fetch_contract_with_history_latest_query(contract_id, true),
            (false, false) => Self::fetch_contract_query(contract_id, true),
        };

        tracing::trace!(?path_query, "verify contract");

        let result = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(proof, &path_query)
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &path_query)
        };
        let (root_hash, mut proved_key_values) = match result.map_err(GroveDB) {
            Ok(ok_result) => ok_result,
            Err(e) => {
                return if contract_known_keeps_history.is_none() {
                    tracing::debug!(?path_query,error=?e, "retrying contract verification with history enabled");
                    // most likely we are trying to prove a historical contract
                    Self::verify_contract(
                        proof,
                        Some(true),
                        is_proof_subset,
                        in_multiple_contract_proof_form,
                        contract_id,
                        platform_version,
                    )
                } else {
                    Err(e)
                };
            }
        };
        if proved_key_values.is_empty() {
            return Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }));
        }
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if contract_known_keeps_history.unwrap_or_default() {
                if path != contract_keeping_history_root_path(&contract_id) {
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
            tracing::trace!(?maybe_element, "verify contract returns proved element");

            let contract = maybe_element
                .map(|element| {
                    element
                        .into_item_bytes()
                        .map_err(Error::GroveDB)
                        .and_then(|bytes| {
                            // we don't need to validate the contract locally because it was proved to be in platform
                            // and hence it is valid
                            DataContract::versioned_deserialize(&bytes, false, platform_version)
                                .map_err(Error::Protocol)
                        })
                })
                .transpose();
            match contract {
                Ok(contract) => Ok((root_hash, contract)),
                Err(e) => {
                    if contract_known_keeps_history.is_some() {
                        // just return error
                        Err(e)
                    } else {
                        tracing::debug!(?path_query,error=?e, "retry contract verification with history enabled");
                        Self::verify_contract(
                            proof,
                            Some(true),
                            is_proof_subset,
                            in_multiple_contract_proof_form,
                            contract_id,
                            platform_version,
                        )
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
        platform_version: &PlatformVersion,
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
            let (root_hash, contract) =
                Self::verify_contract(proof, None, true, true, *contract_id, platform_version)?;
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
}
