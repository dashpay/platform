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
        let keeps_history = contract_known_keeps_history.unwrap_or(false);

        let result = Self::verify_contract_return_serialization_v0_given_history(
            proof,
            keeps_history,
            is_proof_subset,
            in_multiple_contract_proof_form,
            contract_id,
            platform_version,
        );

        if contract_known_keeps_history.is_none() {
            match &result {
                Ok((_, Some(_))) => result,
                _ => {
                    tracing::debug!(
                        ?contract_id,
                        "retrying contract verification with history enabled"
                    );
                    Self::verify_contract_return_serialization_v0_given_history(
                        proof,
                        true,
                        is_proof_subset,
                        in_multiple_contract_proof_form,
                        contract_id,
                        platform_version,
                    )
                }
            }
        } else {
            result
        }
    }

    fn verify_contract_return_serialization_v0_given_history(
        proof: &[u8],
        keeps_history: bool,
        is_proof_subset: bool,
        in_multiple_contract_proof_form: bool,
        contract_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<VerifyContractReturn, Error> {
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
                .transpose()?;

            Ok((root_hash, contract))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one contract id",
            )))
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_contract_return_serialization() {
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

        let (_root_hash, verified_result) = Drive::verify_contract_return_serialization(
            &proof,
            Some(false),
            false,
            false,
            contract_id,
            platform_version,
        )
        .expect("should verify contract with serialization");

        assert!(verified_result.is_some(), "verified result should be Some");
        let (verified_contract, serialized_bytes) = verified_result.unwrap();
        assert_eq!(verified_contract.id(), contract.id());
        assert_eq!(verified_contract.version(), contract.version());

        // Verify the serialized bytes can be deserialized back to the same contract
        let deserialized = dpp::prelude::DataContract::versioned_deserialize(
            &serialized_bytes,
            false,
            platform_version,
        )
        .expect("should deserialize contract bytes");
        assert_eq!(deserialized.id(), contract.id());
    }

    #[test]
    fn should_prove_and_verify_non_existent_contract_return_serialization() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let non_existent_id = [0xffu8; 32];

        let proof = drive
            .prove_contract(non_existent_id, None, platform_version)
            .expect("should prove non-existent contract");

        let (_root_hash, verified_result) = Drive::verify_contract_return_serialization(
            &proof,
            Some(false),
            false,
            false,
            non_existent_id,
            platform_version,
        )
        .expect("should verify non-existent contract proof");

        assert!(
            verified_result.is_none(),
            "verified result should be None for non-existent contract"
        );
    }
}
