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
use grovedb::GroveDb;

impl Drive {
    /// Verifies that the contract is in the Proof
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

    /// Verifies that the contract history is in the Proof
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
