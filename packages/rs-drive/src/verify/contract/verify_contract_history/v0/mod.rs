use crate::drive::contract::paths::{contract_root_path_vec, contract_storage_path_vec};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;

use dpp::prelude::DataContract;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::util::common::decode;
use crate::verify::bounded_decode::decode_proof_data_contract;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
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
    #[inline(always)]
    pub(super) fn verify_contract_history_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, DataContract>>), Error> {
        let path_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        let (root_hash, mut proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        // A contract that does NOT keep history stores its single current version as an
        // Item at the contract root path (key [0]), one level above the history subtree.
        // Querying history for such a contract proves THAT single Item instead of the
        // (non-existent) history subtree. Treat it as a valid empty history rather than a
        // corrupted proof. Require exactly one proved element that is a present item, so any
        // other shape at this path still fails as corrupted (mirroring the strict path
        // discrimination applied to the history entries below).
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = &proved_key_values[0];
            if path == &contract_root_path_vec(&contract_id) && key == &vec![0u8] {
                let is_item = maybe_element
                    .as_ref()
                    .map(|element| element.is_any_item())
                    .unwrap_or(false);
                if !is_item {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected a contract item at the contract root path".to_string(),
                    )));
                }
                return Ok((root_hash, Some(BTreeMap::new())));
            }
        }

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
                        .map_err(Error::from)
                        .and_then(|bytes| decode_proof_data_contract(&bytes, platform_version))
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

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    fn apply_contract(drive: &Drive, contract: &DataContract, block_info: BlockInfo) {
        let platform_version = PlatformVersion::latest();
        drive
            .apply_contract(contract, block_info, true, None, None, platform_version)
            .expect("should apply contract");
    }

    #[test]
    fn should_prove_and_verify_contract_history() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        let contract_id = contract.id().to_buffer();

        // Apply original contract at time 1000
        apply_contract(
            &drive,
            &contract,
            BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            },
        );

        // Apply an update at time 2000
        contract.increment_version();
        apply_contract(
            &drive,
            &contract,
            BlockInfo {
                time_ms: 2000,
                height: 101,
                core_height: 11,
                epoch: Default::default(),
            },
        );

        // Prove history starting from time 0
        let proof = drive
            .prove_contract_history(contract_id, None, 0, Some(10), None, platform_version)
            .expect("should prove contract history");

        let (_root_hash, verified_history) = Drive::verify_contract_history(
            &proof,
            contract_id,
            0,
            Some(10),
            None,
            platform_version,
        )
        .expect("should verify contract history");

        let history = verified_history.expect("history should be Some");
        assert_eq!(history.len(), 2, "should have 2 history entries");

        // Verify the entries exist at the expected timestamps
        assert!(
            history.contains_key(&1000),
            "should have entry at time 1000"
        );
        assert!(
            history.contains_key(&2000),
            "should have entry at time 2000"
        );

        // The first entry should be version 1, second should be version 2
        let first_contract = &history[&1000];
        let second_contract = &history[&2000];
        assert_eq!(first_contract.version(), 1);
        assert_eq!(second_contract.version(), 2);
    }

    #[test]
    fn should_prove_and_verify_contract_history_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        let contract_id = contract.id().to_buffer();

        // Apply original contract at time 1000
        apply_contract(
            &drive,
            &contract,
            BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            },
        );

        // Apply updates at times 2000, 3000, 4000
        for i in 1..=3u64 {
            contract.increment_version();
            apply_contract(
                &drive,
                &contract,
                BlockInfo {
                    time_ms: 1000 * (i + 1),
                    height: 100 + i,
                    core_height: 10 + i as u32,
                    epoch: Default::default(),
                },
            );
        }

        // Prove with limit = 2 (should return 2 most recent entries from start)
        let proof = drive
            .prove_contract_history(contract_id, None, 0, Some(2), None, platform_version)
            .expect("should prove contract history");

        let (_root_hash, verified_history) =
            Drive::verify_contract_history(&proof, contract_id, 0, Some(2), None, platform_version)
                .expect("should verify contract history");

        let history = verified_history.expect("history should be Some");
        assert_eq!(
            history.len(),
            2,
            "should have 2 history entries with limit=2"
        );
    }

    #[test]
    fn should_return_empty_history_for_non_history_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        // The contract does NOT keep history (the default, but make it explicit).
        contract.config_mut().set_keeps_history(false);
        contract.config_mut().set_readonly(false);

        let contract_id = contract.id().to_buffer();

        // Apply the contract once at time 1000
        apply_contract(
            &drive,
            &contract,
            BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            },
        );

        // Prove history starting from time 0
        let proof = drive
            .prove_contract_history(contract_id, None, 0, Some(10), None, platform_version)
            .expect("should prove contract history for a non-history contract");

        let (_root_hash, verified_history) = Drive::verify_contract_history(
            &proof,
            contract_id,
            0,
            Some(10),
            None,
            platform_version,
        )
        .expect("should verify contract history for a non-history contract");

        let history = verified_history.expect("history should be Some (an empty map)");
        assert!(
            history.is_empty(),
            "a contract that does not keep history should return an empty history map"
        );
    }

    #[test]
    fn should_return_empty_history_for_non_history_contract_with_params() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(false);
        contract.config_mut().set_readonly(false);

        let contract_id = contract.id().to_buffer();

        apply_contract(
            &drive,
            &contract,
            BlockInfo {
                time_ms: 1000,
                height: 100,
                core_height: 10,
                epoch: Default::default(),
            },
        );

        // A non-default limit and start_at_ms must not change the empty result.
        // (offset is intentionally not exercised here: count-offset pagination is a
        // prove-side limitation that errors against a non-history contract's item path,
        // and the FFI never sends a non-zero offset for the no-history case anyway.)
        let proof = drive
            .prove_contract_history(contract_id, None, 500, Some(5), None, platform_version)
            .expect("should prove contract history for a non-history contract");

        let (_root_hash, verified_history) = Drive::verify_contract_history(
            &proof,
            contract_id,
            500,
            Some(5),
            None,
            platform_version,
        )
        .expect("should verify contract history for a non-history contract");

        let history = verified_history.expect("history should be Some (an empty map)");
        assert!(
            history.is_empty(),
            "a non-history contract should return empty regardless of limit/start_at_ms"
        );
    }
}
