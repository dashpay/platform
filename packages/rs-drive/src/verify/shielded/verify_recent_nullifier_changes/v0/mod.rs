use crate::drive::shielded::nullifiers::queries::shielded_recent_nullifiers_path_vec;
use crate::drive::shielded::nullifiers::types::{CompactedNullifiers, NullifierChangePerBlock};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies recent nullifier changes proof.
    ///
    /// Uses the same query as the prove function: a simple range query
    /// starting from start_block_height.
    pub(super) fn verify_recent_nullifier_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<NullifierChangePerBlock>), Error> {
        let path = shielded_recent_nullifiers_path_vec();

        // Create the same range query as the prove function
        let mut query = Query::new();
        query.insert_range_from(start_block_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut nullifier_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
            let Some(element) = maybe_element else {
                continue;
            };

            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "invalid block height key length".to_string(),
                ))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item with sum item element for nullifiers".to_string(),
                )));
            };

            // Deserialize the nullifier list
            let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            nullifier_changes.push(NullifierChangePerBlock {
                block_height,
                nullifiers,
            });
        }

        Ok((root_hash, nullifier_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_recent_nullifier_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let nullifier_1 = [1u8; 32];
        let nullifier_2 = [2u8; 32];
        let nullifier_3 = [3u8; 32];

        // Store nullifiers for block 100
        drive
            .store_nullifiers_for_block(
                &[nullifier_1, nullifier_2],
                100,
                100_000,
                None,
                platform_version,
            )
            .expect("should store nullifiers for block 100");

        // Store nullifiers for block 101
        drive
            .store_nullifiers_for_block(&[nullifier_3], 101, 101_000, None, platform_version)
            .expect("should store nullifiers for block 101");

        // Prove from block 100
        let proof = drive
            .prove_recent_nullifier_changes(100, None, None, platform_version)
            .expect("should prove recent nullifier changes");

        // Verify
        let (root_hash, changes) = Drive::verify_recent_nullifier_changes(
            proof.as_slice(),
            100,
            None,
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(changes.len(), 2, "should have 2 blocks of changes");

        assert_eq!(changes[0].block_height, 100);
        assert_eq!(changes[0].nullifiers.len(), 2);

        assert_eq!(changes[1].block_height, 101);
        assert_eq!(changes[1].nullifiers.len(), 1);
    }

    #[test]
    fn should_prove_and_verify_empty_recent_nullifier_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove from block 100 with no data stored
        let proof = drive
            .prove_recent_nullifier_changes(100, None, None, platform_version)
            .expect("should prove empty recent nullifier changes");

        // Verify
        let (root_hash, changes) = Drive::verify_recent_nullifier_changes(
            proof.as_slice(),
            100,
            None,
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(changes.is_empty(), "should have no changes");
    }

    #[test]
    fn should_prove_and_verify_recent_nullifier_changes_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Store 3 blocks of nullifiers
        for block_height in 100u64..103 {
            let nullifier = [block_height as u8; 32];
            drive
                .store_nullifiers_for_block(
                    &[nullifier],
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store nullifiers");
        }

        // Prove from block 100 with limit 2
        let proof = drive
            .prove_recent_nullifier_changes(100, Some(2), None, platform_version)
            .expect("should prove with limit");

        // Verify
        let (root_hash, changes) = Drive::verify_recent_nullifier_changes(
            proof.as_slice(),
            100,
            Some(2),
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            changes.len(),
            2,
            "should have 2 blocks of changes (limited)"
        );
        assert_eq!(changes[0].block_height, 100);
        assert_eq!(changes[1].block_height, 101);
    }
}
