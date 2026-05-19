use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_shielded_nullifiers_v0(
        proof: &[u8],
        nullifiers: &[Vec<u8>],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, bool)>), Error> {
        let mut query = Query::new();
        query.insert_keys(nullifiers.to_vec());

        let path_query = PathQuery {
            path: shielded_credit_pool_nullifiers_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(u16::try_from(nullifiers.len()).map_err(|_| {
                    Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                        "nullifier count exceeds u16::MAX".to_string(),
                    ))
                })?),
                offset: None,
            },
        };

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
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

        // Map each proved entry: if element is Some, nullifier is spent; if None, not spent
        let statuses = proved_key_values
            .into_iter()
            .map(|(_, key, maybe_element)| {
                let is_spent = maybe_element.is_some();
                (key, is_spent)
            })
            .collect();

        Ok((root_hash, statuses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::Element;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_nullifier_spent_status() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let nullifier_spent = vec![1u8; 32];
        let nullifier_unspent = vec![2u8; 32];

        // Insert only the "spent" nullifier into the nullifiers tree
        let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();
        let op = QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
            nullifiers_path.clone(),
            nullifier_spent.clone(),
            Element::new_item(vec![]),
        );

        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should apply batch");

        // Construct and prove the same path query as the verify function
        let nullifiers = vec![nullifier_spent.clone(), nullifier_unspent.clone()];
        let mut query = Query::new();
        query.insert_keys(nullifiers.clone());

        let path_query = PathQuery {
            path: nullifiers_path,
            query: SizedQuery {
                query,
                limit: Some(2),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, statuses) = Drive::verify_shielded_nullifiers(
            proof.as_slice(),
            &nullifiers,
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(statuses.len(), 2, "should have 2 statuses");

        // Find the spent one and the unspent one
        let spent_status = statuses
            .iter()
            .find(|(key, _)| key == &nullifier_spent)
            .expect("should have spent nullifier");
        assert!(spent_status.1, "spent nullifier should be marked as spent");

        let unspent_status = statuses
            .iter()
            .find(|(key, _)| key == &nullifier_unspent)
            .expect("should have unspent nullifier");
        assert!(
            !unspent_status.1,
            "unspent nullifier should be marked as not spent"
        );
    }
}
