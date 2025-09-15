use crate::drive::contract::paths::{contract_keeping_history_root_path, contract_root_path};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::prelude::DataContract;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use platform_version::version::PlatformVersion;

use grovedb::GroveDb;

// Type aliases to simplify complex return types
type ContractBytes = Vec<u8>;
type VerifiedContractWithBytes = Option<(DataContract, ContractBytes)>;
type VerifyContractReturn = (RootHash, VerifiedContractWithBytes);

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
    pub(super) fn verify_contract_return_serialization_v0(
        proof: &[u8],
        contract_known_keeps_history: Option<bool>,
        is_proof_subset: bool,
        in_multiple_contract_proof_form: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<VerifyContractReturn, Error> {
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
        let (root_hash, mut proved_key_values) = match result.map_err(Error::from) {
            Ok(ok_result) => ok_result,
            Err(e) => {
                return if contract_known_keeps_history.is_none() {
                    tracing::debug!(?path_query,error=?e, "retrying contract verification with history enabled");
                    // most likely we are trying to prove a historical contract
                    Self::verify_contract_return_serialization_v0(
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
                        .map_err(Error::from)
                        .and_then(|bytes| {
                            // we don't need to validate the contract locally because it was proved to be in platform
                            // and hence it is valid
                            Ok((
                                DataContract::versioned_deserialize(
                                    &bytes,
                                    false,
                                    platform_version,
                                )
                                .map_err(Error::from)?,
                                bytes,
                            ))
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
                        Self::verify_contract_return_serialization_v0(
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
}
