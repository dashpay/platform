use crate::drive::shielded::paths::{
    shielded_credit_pool_path_vec, SHIELDED_MOST_RECENT_ANCHOR_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_most_recent_shielded_anchor_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        if proved_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected at most 1 element for most recent shielded anchor",
            )));
        }

        let anchor = if let Some(proved) = proved_key_values.pop() {
            match proved.2 {
                Some(Element::Item(value, _)) => {
                    let anchor: [u8; 32] = value.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "most recent anchor is not 32 bytes",
                        ))
                    })?;
                    // A zero anchor means no anchor has been recorded yet
                    if anchor == [0u8; 32] {
                        None
                    } else {
                        Some(anchor)
                    }
                }
                Some(_) => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected an item for most recent shielded anchor".to_string(),
                    )));
                }
                None => None,
            }
        } else {
            None
        };

        Ok((root_hash, anchor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::{
        shielded_credit_pool_path_vec, SHIELDED_MOST_RECENT_ANCHOR_KEY,
    };
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::{PathQuery, Query, SizedQuery};
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_most_recent_shielded_anchor_present() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let pool_path = shielded_credit_pool_path_vec();
        let anchor: [u8; 32] = [42u8; 32];

        // Insert the most recent anchor
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            pool_path.clone(),
            vec![SHIELDED_MOST_RECENT_ANCHOR_KEY],
            Element::new_item(anchor.to_vec()),
        );

        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should apply batch");

        // Construct the same path query as the verify function
        let path_query = PathQuery {
            path: pool_path,
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, verified_anchor) =
            Drive::verify_most_recent_shielded_anchor(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            verified_anchor,
            Some(anchor),
            "verified anchor should match"
        );
    }

    #[test]
    fn should_prove_and_verify_most_recent_shielded_anchor_absent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let pool_path = shielded_credit_pool_path_vec();

        // Construct the same path query (no data inserted)
        let path_query = PathQuery {
            path: pool_path,
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
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
    fn should_prove_and_verify_zero_anchor_as_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let pool_path = shielded_credit_pool_path_vec();

        // Insert a zero anchor (means no anchor has been recorded yet)
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            pool_path.clone(),
            vec![SHIELDED_MOST_RECENT_ANCHOR_KEY],
            Element::new_item([0u8; 32].to_vec()),
        );

        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should apply batch");

        // Construct the same path query
        let path_query = PathQuery {
            path: pool_path,
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify - zero anchor should be treated as None
        let (root_hash, verified_anchor) =
            Drive::verify_most_recent_shielded_anchor(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            verified_anchor.is_none(),
            "zero anchor should be treated as none"
        );
    }
}
