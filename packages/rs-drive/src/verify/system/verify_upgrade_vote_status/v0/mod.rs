use crate::drive::protocol_upgrade::desired_version_for_validators_path_vec;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use crate::verify::RootHash;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::{GroveDb, PathQuery, SizedQuery};
use integer_encoding::VarInt;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Verifies a proof containing the current upgrade state.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `first_pro_tx_hash`: the first pro tx hash that we are querying for.
    /// - `count`: the amount of Evonodes that we want to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `BTreeMap<[u8;32], ProtocolVersion>`. The `BTreeMap<[u8;32], ProtocolVersion>`
    /// represents a map of the version that each Evonode has voted for.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_upgrade_vote_status_v0(
        proof: &[u8],
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<[u8; 32], ProtocolVersion>), Error> {
        let path = desired_version_for_validators_path_vec();

        let query_item = if let Some(start_protx_hash) = start_protx_hash {
            QueryItem::RangeFrom(start_protx_hash.to_vec()..)
        } else {
            QueryItem::RangeFull(RangeFull)
        };

        let path_query = PathQuery::new(
            path,
            SizedQuery::new(Query::new_single_query_item(query_item), Some(count), None),
        );

        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let protocol_version_map = elements
            .into_iter()
            .map(|(_, key, element)| {
                let pro_tx_hash: [u8; 32] = key.try_into().map_err(|_| {
                    ProofError::CorruptedProof("protocol version not decodable".to_string())
                })?;
                let element = element.ok_or(ProofError::CorruptedProof(
                    "expected a count for each version, got none".to_string(),
                ))?;
                let version_bytes = element.as_item_bytes().map_err(|_| {
                    ProofError::CorruptedProof(
                        "expected an item for the element of a version".to_string(),
                    )
                })?;
                let version = u32::decode_var(version_bytes)
                    .ok_or(ProofError::CorruptedProof(
                        "version count not decodable".to_string(),
                    ))?
                    .0;
                Ok((pro_tx_hash, version))
            })
            .collect::<Result<BTreeMap<[u8; 32], ProtocolVersion>, Error>>()?;

        Ok((root_hash, protocol_version_map))
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn should_prove_and_verify_upgrade_vote_status_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .fetch_proved_validator_version_votes(None, 100, None, &platform_version.drive)
            .expect("should fetch proved validator version votes");

        let (_root_hash, vote_map) =
            Drive::verify_upgrade_vote_status(&proof, None, 100, platform_version)
                .expect("should verify upgrade vote status");

        assert!(vote_map.is_empty(), "no votes should exist yet");
    }

    #[test]
    fn should_prove_and_verify_upgrade_vote_status_with_votes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let validator1: [u8; 32] = [1u8; 32];
        let validator2: [u8; 32] = [2u8; 32];
        let validator3: [u8; 32] = [3u8; 32];

        let version_a: ProtocolVersion = platform_version.protocol_version + 1;
        let version_b: ProtocolVersion = platform_version.protocol_version + 2;

        drive
            .update_validator_proposed_app_version(
                validator1,
                version_a,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .update_validator_proposed_app_version(
                validator2,
                version_b,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .update_validator_proposed_app_version(
                validator3,
                version_a,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        let proof = drive
            .fetch_proved_validator_version_votes(None, 100, None, &platform_version.drive)
            .expect("should fetch proved validator version votes");

        let (_root_hash, vote_map) =
            Drive::verify_upgrade_vote_status(&proof, None, 100, platform_version)
                .expect("should verify upgrade vote status");

        assert_eq!(vote_map.len(), 3, "should have 3 validator entries");
        assert_eq!(vote_map[&validator1], version_a);
        assert_eq!(vote_map[&validator2], version_b);
        assert_eq!(vote_map[&validator3], version_a);
    }

    #[test]
    fn should_prove_and_verify_upgrade_vote_status_with_start_protx_hash() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let validator1: [u8; 32] = [1u8; 32];
        let validator2: [u8; 32] = [2u8; 32];

        let version_a: ProtocolVersion = platform_version.protocol_version + 1;

        drive
            .update_validator_proposed_app_version(
                validator1,
                version_a,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .update_validator_proposed_app_version(
                validator2,
                version_a,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");

        // Query starting from validator2 (should include validator2 since RangeFrom is inclusive)
        let proof = drive
            .fetch_proved_validator_version_votes(
                Some(validator2),
                100,
                None,
                &platform_version.drive,
            )
            .expect("should fetch proved validator version votes");

        let (_root_hash, vote_map) =
            Drive::verify_upgrade_vote_status(&proof, Some(validator2), 100, platform_version)
                .expect("should verify upgrade vote status");

        // validator2 = [2u8; 32] > validator1 = [1u8; 32], so only validator2 should be returned
        assert_eq!(vote_map.len(), 1, "should have 1 validator entry");
        assert_eq!(vote_map[&validator2], version_a);
    }
}
