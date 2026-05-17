use crate::drive::protocol_upgrade::versions_counter_path_vec;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use crate::verify::RootHash;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::{GroveDb, PathQuery};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use platform_version::version::PlatformVersion;
use std::ops::RangeFull;

impl Drive {
    /// Verifies a proof containing the current upgrade state.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `IntMap<ProtocolVersion, u64>`. The `IntMap<ProtocolVersion, u64>`
    /// represents votes count of each version in the current epoch.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_upgrade_state_v0(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IntMap<ProtocolVersion, u64>), Error> {
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );

        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let protocol_version_map = elements
            .into_iter()
            .map(|(_, key, element)| {
                let version = ProtocolVersion::decode_var(key.as_slice())
                    .ok_or(ProofError::CorruptedProof(
                        "protocol version not decodable".to_string(),
                    ))?
                    .0;
                let element = element.ok_or(ProofError::CorruptedProof(
                    "expected a count for each version, got none".to_string(),
                ))?;
                let count_bytes = element.as_item_bytes().map_err(|_| {
                    ProofError::CorruptedProof(
                        "expected an item for the element of a version".to_string(),
                    )
                })?;
                let count = u64::decode_var(count_bytes)
                    .ok_or(ProofError::CorruptedProof(
                        "version count not decodable".to_string(),
                    ))?
                    .0;
                Ok((version, count))
            })
            .collect::<Result<IntMap<ProtocolVersion, u64>, Error>>()?;

        Ok((root_hash, protocol_version_map))
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn should_prove_and_verify_upgrade_state_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // With no votes cast, the upgrade state should be empty
        let proof = drive
            .fetch_proved_versions_with_counter(None, &platform_version.drive)
            .expect("should fetch proved versions with counter");

        let (_root_hash, version_map) = Drive::verify_upgrade_state(&proof, platform_version)
            .expect("should verify upgrade state");

        assert!(version_map.is_empty(), "no versions should be recorded yet");
    }

    #[test]
    fn should_prove_and_verify_upgrade_state_with_votes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Cast votes from multiple validators for different versions
        let validator1: [u8; 32] = [1u8; 32];
        let validator2: [u8; 32] = [2u8; 32];
        let validator3: [u8; 32] = [3u8; 32];

        let target_version: ProtocolVersion = platform_version.protocol_version + 1;

        drive
            .update_validator_proposed_app_version(
                validator1,
                target_version,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .update_validator_proposed_app_version(
                validator2,
                target_version,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should update validator proposed app version");

        drive
            .update_validator_proposed_app_version(
                validator3,
                target_version + 1,
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
            .fetch_proved_versions_with_counter(None, &platform_version.drive)
            .expect("should fetch proved versions with counter");

        let (_root_hash, version_map) = Drive::verify_upgrade_state(&proof, platform_version)
            .expect("should verify upgrade state");

        assert_eq!(
            version_map.len(),
            2,
            "should have two distinct version entries"
        );
        assert_eq!(
            version_map.get(&target_version).copied(),
            Some(2),
            "target_version should have 2 votes"
        );
        assert_eq!(
            version_map.get(&(target_version + 1)).copied(),
            Some(1),
            "target_version+1 should have 1 vote"
        );
    }
}
