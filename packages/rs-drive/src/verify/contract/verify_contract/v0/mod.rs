use std::collections::BTreeMap;

use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::bounded_decode::decode_proof_data_contract;
use crate::verify::contract::retry_contract_verification_with_history;
use crate::verify::RootHash;
use dpp::prelude::DataContract;
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
        let keeps_history = contract_known_keeps_history.unwrap_or(false);

        let result = Self::verify_contract_v0_given_history(
            proof,
            keeps_history,
            is_proof_subset,
            in_multiple_contract_proof_form,
            contract_id,
            platform_version,
        );

        retry_contract_verification_with_history(
            result,
            contract_known_keeps_history,
            contract_id,
            in_multiple_contract_proof_form,
            || {
                Self::verify_contract_v0_given_history(
                    proof,
                    true,
                    is_proof_subset,
                    in_multiple_contract_proof_form,
                    contract_id,
                    platform_version,
                )
            },
            |(_, contract)| contract.is_some(),
        )
    }

    fn verify_contract_v0_given_history(
        proof: &[u8],
        keeps_history: bool,
        is_proof_subset: bool,
        in_multiple_contract_proof_form: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<DataContract>), Error> {
        let path_query = match (in_multiple_contract_proof_form, keeps_history) {
            (true, true) => Self::fetch_historical_contracts_query(&[contract_id]),
            (true, false) => Self::fetch_non_historical_contracts_query(&[contract_id]),
            (false, true) => Self::fetch_contract_with_history_latest_query(contract_id, true),
            (false, false) => Self::fetch_contract_query(contract_id, true),
        };

        tracing::trace!(?path_query, "verify contract");

        let result = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )
        };
        let (root_hash, mut proved_key_values) = result.map_err(Error::from)?;
        if proved_key_values.is_empty() {
            return Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }));
        }
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if keeps_history {
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
                        .map_err(Error::from)
                        .and_then(|bytes| {
                            // The computed proof root is authenticated by the caller. Keep
                            // proof-derived object construction bounded until that happens.
                            decode_proof_data_contract(&bytes, platform_version)
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
    /// - `contract_ids_with_keeps_history` a BTreemap with keys being the contract ids we are looking
    ///   to search for, values being if they keep history. For this call we must know if they keep
    ///   history.
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
    // TODO: Use type alias or struct
    #[allow(clippy::type_complexity)]
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

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_existing_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let created_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = created_contract.data_contract_owned();
        let contract_id = contract.id().to_buffer();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should insert contract");

        let proof = drive
            .prove_contract(contract_id, None, platform_version)
            .expect("should prove contract");

        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            false,
            contract_id,
            platform_version,
        )
        .expect("should verify contract");

        assert!(
            verified_contract.is_some(),
            "verified contract should exist"
        );
        let verified = verified_contract.unwrap();
        assert_eq!(verified.id(), contract.id());
        assert_eq!(verified.version(), contract.version());
    }

    #[test]
    fn should_prove_and_verify_non_existent_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let non_existent_id = [0xffu8; 32];

        let proof = drive
            .prove_contract(non_existent_id, None, platform_version)
            .expect("should prove non-existent contract");

        let (_root_hash, verified_contract) = Drive::verify_contract(
            &proof,
            Some(false),
            false,
            false,
            non_existent_id,
            platform_version,
        )
        .expect("should verify non-existent contract proof");

        assert!(
            verified_contract.is_none(),
            "verified contract should be None for non-existent contract"
        );
    }
}
