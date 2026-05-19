use crate::drive::shielded::paths::shielded_latest_recorded_anchor_path_query;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verify a `getMostRecentShieldedAnchor` proof.
    ///
    /// Replays the canonical "latest entry in `[..., "s", [96]]`,
    /// `limit 1` reverse" path query that the drive-abci handler
    /// runs (see `shielded_latest_recorded_anchor_path_query`) and
    /// extracts the anchor bytes from the highest-block-height entry.
    ///
    /// Returns `Ok(None)` if the anchors-by-height index is empty —
    /// i.e. no shielded ops have produced a recorded anchor yet on
    /// this chain.
    pub(super) fn verify_most_recent_shielded_anchor_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let path_query = shielded_latest_recorded_anchor_path_query();

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        if proved_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected at most 1 element for most recent shielded anchor",
            )));
        }

        let anchor = match proved_key_values.pop() {
            Some(proved) => match proved.2 {
                Some(Element::Item(value, _)) => {
                    let anchor: [u8; 32] = value.try_into().map_err(|_v: Vec<u8>| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "anchors-by-height value is not 32 bytes",
                        ))
                    })?;
                    Some(anchor)
                }
                Some(_) => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected an item for most recent shielded anchor".to_string(),
                    )));
                }
                None => None,
            },
            None => None,
        };

        Ok((root_hash, anchor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::{
        shielded_credit_pool_anchors_by_height_path_vec, MAIN_SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_ANCHORS_BY_HEIGHT_KEY,
    };
    use crate::drive::RootTree;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use platform_version::version::PlatformVersion;

    fn seed_anchor_at_height(
        drive: &Drive,
        height: u64,
        anchor: [u8; 32],
        platform_version: &PlatformVersion,
    ) {
        let by_height_path = vec![
            vec![RootTree::ShieldedBalances as u8],
            MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
            vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
        ];
        let op = QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
            by_height_path,
            height.to_be_bytes().to_vec(),
            Element::new_item(anchor.to_vec()),
        );
        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("seed anchor at height");
    }

    #[test]
    fn should_prove_and_verify_most_recent_shielded_anchor_present() {
        // Seed the highest-block-height entry; verifier reads it back.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let anchor: [u8; 32] = [42u8; 32];
        seed_anchor_at_height(&drive, 100, anchor, platform_version);

        let path_query = shielded_latest_recorded_anchor_path_query();
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        let (root_hash, verified_anchor) =
            Drive::verify_most_recent_shielded_anchor(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            verified_anchor,
            Some(anchor),
            "verified anchor should match seeded anchor"
        );
    }

    #[test]
    fn should_prove_and_verify_most_recent_shielded_anchor_absent() {
        // No anchors recorded — the index is empty, so the verifier
        // returns None.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let path_query = shielded_latest_recorded_anchor_path_query();
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        let (root_hash, verified_anchor) =
            Drive::verify_most_recent_shielded_anchor(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            verified_anchor.is_none(),
            "anchor should be absent when not set"
        );
    }

    #[test]
    fn highest_block_height_wins() {
        // Multiple recorded anchors → the verifier returns the one at
        // the highest block_height (`limit 1` + `left_to_right=false`).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        seed_anchor_at_height(&drive, 50, [0x11u8; 32], platform_version);
        seed_anchor_at_height(&drive, 200, [0x22u8; 32], platform_version);
        seed_anchor_at_height(&drive, 100, [0x33u8; 32], platform_version);

        let path_query = shielded_latest_recorded_anchor_path_query();
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        let (_, verified_anchor) =
            Drive::verify_most_recent_shielded_anchor(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert_eq!(
            verified_anchor,
            Some([0x22u8; 32]),
            "highest height (200) should win"
        );
        // Suppress unused-import warnings under cfg(test).
        let _ = shielded_credit_pool_anchors_by_height_path_vec;
    }
}
