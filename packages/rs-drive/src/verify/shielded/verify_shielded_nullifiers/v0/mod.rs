use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
use crate::drive::tokens::paths::token_shielded_pools_root_path_vec;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::prelude::Identifier;
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
        // Every protocol version selects this generation, so splitting the body out behind a
        // path parameter has to leave the credit pool alone, and it does: this entry point passes
        // the same path the body used to build inline, so an existing credit pool proof is checked
        // by the same PathQuery and yields the same root hash and the same result. A token pool
        // path reaches the shared body only from a caller that cannot exist below the version
        // that admits token pools.
        Self::verify_pool_nullifiers_v0(
            proof,
            shielded_credit_pool_nullifiers_path_vec(),
            nullifiers,
            verify_subset_of_proof,
            platform_version,
        )
    }

    /// Verifies nullifier spent statuses against the nullifiers tree at `nullifiers_path`,
    /// whichever shielded pool (credit or token) it belongs to.
    ///
    /// Reports a nullifier the proof did not carry as unspent, which is also what a pool that does
    /// not exist gives, so a caller whose pool may be missing has to establish that separately,
    /// with `verify_token_shielded_pool_exists_v0`. The credit pool is not named by the caller, so
    /// it cannot be asked for one that was never created; a chain serving this read before the
    /// credit pool exists reads it as empty.
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_pool_nullifiers_v0(
        proof: &[u8],
        nullifiers_path: Vec<Vec<u8>>,
        nullifiers: &[Vec<u8>],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, bool)>), Error> {
        let mut query = Query::new();
        query.insert_keys(nullifiers.to_vec());

        let path_query = PathQuery {
            path: nullifiers_path,
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

    /// Reads out of the same nullifier proof whether the token's shielded pool exists at all,
    /// and refuses the read when it does not.
    ///
    /// A nullifier proof reports a key it did not carry as absent, and a pool that was never
    /// created carries none of them, so an unspent nullifier and a pool the chain does not have
    /// project to the same rows. `is_spent: false` is what a wallet leans on to treat a note as
    /// still spendable, and a pool that does not exist holds no spendable note, so the two facts
    /// must not reach the caller as one answer. The unproved read of the same question refuses a
    /// pool the chain does not have for this reason; attaching a proof does not make the collapsed
    /// answer safe to act on, it only makes it verifiable.
    ///
    /// The nullifiers query descends through the pool, so the proof already carries the pools
    /// tree's own Merk proof for the pool's key — the pool element, or that key proven absent.
    /// Reading it is a strict subset of what the nullifier proof proves, so a subset verification
    /// against the same bytes succeeds whichever way the outer verification ran, and the root hash
    /// both derive ties the two readings to one state.
    pub(super) fn verify_token_shielded_pool_exists_v0(
        proof: &[u8],
        token_id: [u8; 32],
        nullifiers_root_hash: RootHash,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let pool_path_query = PathQuery {
            path: token_shielded_pools_root_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(token_id.to_vec()),
                limit: Some(1),
                offset: None,
            },
        };

        let (pool_root_hash, mut proved_key_values) = GroveDb::verify_subset_query(
            proof,
            &pool_path_query,
            &platform_version.drive.grove_version,
        )?;

        // Both readings run against the same bytes, so they derive the same root by construction.
        // A mismatch means they came from different state and neither answer can be trusted.
        if pool_root_hash != nullifiers_root_hash {
            return Err(Error::Proof(ProofError::IncorrectProof(
                "token shielded pool existence sub-proof root mismatch".to_string(),
            )));
        }

        match proved_key_values.pop() {
            // Creating a pool installs its five children, so a pool that exists is a tree the
            // proof carries rather than a key it omits.
            Some((_, _, Some(element))) if element.is_any_tree() => Ok(()),
            Some((_, _, Some(_))) => Err(Error::Proof(ProofError::CorruptedProof(format!(
                "the token shielded pools tree holds something other than a pool for token {}",
                Identifier::new(token_id)
            )))),
            Some((_, _, None)) | None => {
                Err(Error::Proof(ProofError::UnexpectedResultProof(format!(
                    "the chain has no shielded pool for token {}, so none of its nullifiers has a \
                     spend status",
                    Identifier::new(token_id)
                ))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::{
        shielded_credit_pool_nullifiers_path_vec, token_shielded_pool_nullifiers_path_vec,
    };
    use crate::error::proof::ProofError;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::Element;
    use platform_version::version::PlatformVersion;

    /// A token id the chain holds no shielded pool for. Any 32 bytes an unauthenticated caller
    /// makes up land here.
    const POOL_LESS_TOKEN_ID: [u8; 32] = [0xAB; 32];

    /// The token id of the pool these tests create.
    const POOLED_TOKEN_ID: [u8; 32] = [3u8; 32];

    /// A proved nullifier read of a token pool the chain does not have carries the same datum as
    /// an unspent nullifier in a pool it does have.
    ///
    /// The query handler builds a key-selection query at the pool's nullifiers path and hands out
    /// whatever GroveDB proves. When the pool subtree is missing, GroveDB proves its absence
    /// honestly, and the verifier's absence projection reports every queried key as `None` —
    /// exactly what it reports for a key that is simply not in a nullifiers tree that does exist.
    /// `is_spent: false` is then the answer a wallet leans on to treat a note as still spendable,
    /// and it is anchored to the real root hash, so the client has no way to discount it.
    #[test]
    fn proved_nullifier_read_tells_a_missing_pool_apart_from_an_unspent_nullifier() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // One token pool the chain really has, left empty so the nullifier below is unspent in it.
        let operations = drive
            .create_token_shielded_pool_trees_operations(
                POOLED_TOKEN_ID,
                false,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the pool tree operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the token shielded pool to be created");

        let nullifier = vec![0x11u8; 32];
        let nullifiers = vec![nullifier.clone()];

        // The query handler's path query, reproduced: the pool's nullifiers path, the requested
        // keys, no limit.
        let prove = |token_id: [u8; 32]| {
            let path_query = PathQuery {
                path: token_shielded_pool_nullifiers_path_vec(token_id),
                query: SizedQuery {
                    query: {
                        let mut query = Query::new();
                        query.insert_keys(nullifiers.clone());
                        query
                    },
                    limit: None,
                    offset: None,
                },
            };

            drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            )
        };

        let missing_pool_proof = prove(POOL_LESS_TOKEN_ID)
            .expect("a pool the chain does not have is answered with an absence proof");
        let empty_pool_proof =
            prove(POOLED_TOKEN_ID).expect("a pool the chain has is answered with a proof");

        // Both verification modes: the standalone nullifiers query is checked strictly, while the
        // state transition proof paths check it as a subset of a larger proof.
        for verify_subset_of_proof in [false, true] {
            let verify = |proof: &[u8], token_id: [u8; 32]| {
                Drive::verify_token_shielded_pool_nullifiers(
                    proof,
                    token_id,
                    &nullifiers,
                    verify_subset_of_proof,
                    platform_version,
                )
            };

            let (unspent_root_hash, unspent_statuses) =
                verify(empty_pool_proof.as_slice(), POOLED_TOKEN_ID)
                    .expect("an existing pool verifies");
            assert_eq!(
                unspent_statuses,
                vec![(nullifier.clone(), false)],
                "an unspent nullifier in a pool the chain has is reported unspent \
                 (verify_subset_of_proof: {verify_subset_of_proof})"
            );

            // A pool the chain does not have must be refused rather than answered with a spend
            // status: the caller cannot tell the two apart, so the verifier is the last place the
            // facts can still be separated.
            match verify(missing_pool_proof.as_slice(), POOL_LESS_TOKEN_ID) {
                Err(Error::Proof(ProofError::UnexpectedResultProof(message))) => assert!(
                    message.contains("no shielded pool"),
                    "the refusal must say the pool is the thing that is missing, got {message:?}"
                ),
                Err(other) => panic!(
                    "expected the missing pool to be named, got {other} \
                     (verify_subset_of_proof: {verify_subset_of_proof})"
                ),
                Ok((root_hash, statuses)) => panic!(
                    "a pool the chain does not have was answered with the spend status \
                     {statuses:?}, the same datum an unspent nullifier in a pool it has returns \
                     ({unspent_statuses:?}), and against the same root hash (equal: {}), so \
                     nothing the caller receives separates the two (verify_subset_of_proof: \
                     {verify_subset_of_proof})",
                    root_hash == unspent_root_hash
                ),
            }
        }
    }

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
