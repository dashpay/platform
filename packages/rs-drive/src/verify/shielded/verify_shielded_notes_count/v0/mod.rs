use crate::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_notes_count_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        // Single-key query that lands on the notes `CommitmentTree`
        // element itself (no subquery into its contents). Because the
        // query has no subquery, GroveDB returns the serialized element
        // — `Element::CommitmentTree(total_count, chunk_power, flags)` —
        // whose bytes are hashed into the Merk value hash, so verifying
        // this proof cryptographically binds `total_count` to the root
        // hash. This mirrors `verify_shielded_pool_state_v0` exactly,
        // except the proved element is a `CommitmentTree` (decode field
        // 0, `total_count`) rather than a `SumItem`.
        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_NOTES_KEY]),
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
                "expected at most 1 element for shielded notes count",
            )));
        }

        let count = if let Some(proved) = proved_key_values.pop() {
            match proved.2 {
                // `total_count` is the first field of the `CommitmentTree`
                // element and is what `grove_commitment_tree_count` reads.
                Some(Element::CommitmentTree(total_count, ..)) => Some(total_count),
                Some(_) => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected a commitment tree element for shielded notes count".to_string(),
                    )));
                }
                None => None,
            }
        } else {
            None
        };

        Ok((root_hash, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    /// Builds the same single-key PathQuery the verifier replays.
    fn notes_count_path_query() -> PathQuery {
        PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_NOTES_KEY]),
                limit: Some(1),
                offset: None,
            },
        }
    }

    #[test]
    fn should_prove_and_verify_zero_notes_on_fresh_state() {
        // The initial state structure creates the notes CommitmentTree
        // with total_count = 0; a proof of that element verifies to Some(0).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .grove_get_proved_path_query(
                &notes_count_path_query(),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("should produce proof");

        let (root_hash, verified_count) =
            Drive::verify_shielded_notes_count(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(verified_count, Some(0), "fresh state should have 0 notes");
    }

    #[test]
    fn should_prove_and_verify_count_after_inserts() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert 3 distinct notes; the CommitmentTree total_count becomes 3.
        for i in 0u8..3 {
            let mut cmx = [0u8; 32];
            cmx[0] = i + 1;
            let mut rho = [0u8; 32];
            rho[0] = i + 100;
            let mut cv_net = [0u8; 32];
            cv_net[0] = i + 200;

            let ops = Drive::insert_note_op(rho, cmx, cv_net, vec![i; 216], platform_version)
                .expect("build note op");
            let grove_ops =
                crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
            drive
                .grove_apply_batch_with_add_costs(
                    grove_ops,
                    false,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("apply note insert");
        }

        let proof = drive
            .grove_get_proved_path_query(
                &notes_count_path_query(),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("should produce proof");

        let (root_hash, verified_count) =
            Drive::verify_shielded_notes_count(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(verified_count, Some(3), "verified count should match");
    }
}
