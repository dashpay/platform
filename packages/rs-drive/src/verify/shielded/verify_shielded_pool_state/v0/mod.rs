use crate::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_TOTAL_BALANCE_KEY};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_pool_state_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_TOTAL_BALANCE_KEY]),
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
                "expected at most 1 element for shielded pool state",
            )));
        }

        let balance = if let Some(proved) = proved_key_values.pop() {
            match proved.2 {
                Some(Element::SumItem(value, _)) => {
                    if value < 0 {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "shielded pool balance cannot be negative".to_string(),
                        )));
                    }
                    Some(value as u64)
                }
                Some(_) => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected a sum item for shielded pool balance".to_string(),
                    )));
                }
                None => None,
            }
        } else {
            None
        };

        Ok((root_hash, balance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::{
        shielded_credit_pool_path_vec, SHIELDED_TOTAL_BALANCE_KEY,
    };
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_shielded_pool_state_with_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let pool_path = shielded_credit_pool_path_vec();
        let balance: u64 = 1_000_000;

        // Insert a total balance sum item
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            pool_path.clone(),
            vec![SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(balance as i64),
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
        let path_query = PathQuery {
            path: pool_path,
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_TOTAL_BALANCE_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, verified_balance) =
            Drive::verify_shielded_pool_state(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            verified_balance,
            Some(balance),
            "verified balance should match"
        );
    }

    #[test]
    fn should_prove_and_verify_shielded_pool_state_zero_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let pool_path = shielded_credit_pool_path_vec();

        // The initialization creates SHIELDED_TOTAL_BALANCE_KEY as SumItem(0)
        // So we should get Some(0) from the proof
        let path_query = PathQuery {
            path: pool_path,
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_TOTAL_BALANCE_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Verify
        let (root_hash, verified_balance) =
            Drive::verify_shielded_pool_state(proof.as_slice(), false, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(verified_balance, Some(0), "initial balance should be zero");
    }
}
