use crate::drive::shielded::paths::shielded_credit_pool_anchors_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_anchors_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<[u8; 32]>), Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        // Anchors are stored as anchor_bytes (key) → block_height_be (value)
        let mut anchors = Vec::with_capacity(proved_key_values.len());
        for (_, key, maybe_element) in proved_key_values {
            if maybe_element.is_some() {
                let anchor: [u8; 32] = key.try_into().map_err(|_v: Vec<u8>| {
                    Error::Drive(DriveError::CorruptedElementType(
                        "anchor key is not 32 bytes",
                    ))
                })?;
                anchors.push(anchor);
            }
        }

        Ok((root_hash, anchors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::shielded_credit_pool_anchors_path_vec;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::{Element, PathQuery, Query, SizedQuery};
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_shielded_anchors() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let anchor_1: [u8; 32] = [1u8; 32];
        let anchor_2: [u8; 32] = [2u8; 32];
        let anchors_path = shielded_credit_pool_anchors_path_vec();

        // Insert anchors into the anchors tree
        // anchor_bytes -> block_height_be
        let ops = vec![
            QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                anchors_path.clone(),
                anchor_1.to_vec(),
                Element::new_item(100u64.to_be_bytes().to_vec()),
            ),
            QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                anchors_path.clone(),
                anchor_2.to_vec(),
                Element::new_item(200u64.to_be_bytes().to_vec()),
            ),
        ];

        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(ops),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should apply batch");

        // Construct and prove the same path query as the verify function
        let path_query = PathQuery {
            path: anchors_path,
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, verified_anchors) =
            Drive::verify_shielded_anchors(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(verified_anchors.len(), 2, "should have 2 anchors");
        assert!(
            verified_anchors.contains(&anchor_1),
            "should contain anchor 1"
        );
        assert!(
            verified_anchors.contains(&anchor_2),
            "should contain anchor 2"
        );
    }

    #[test]
    fn should_prove_and_verify_empty_shielded_anchors() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let anchors_path = shielded_credit_pool_anchors_path_vec();

        // Construct and prove the same path query as the verify function
        let path_query = PathQuery {
            path: anchors_path,
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, verified_anchors) =
            Drive::verify_shielded_anchors(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(verified_anchors.is_empty(), "should have no anchors");
    }
}
