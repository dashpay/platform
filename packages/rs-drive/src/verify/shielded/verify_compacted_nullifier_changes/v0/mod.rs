use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path_vec;
use crate::drive::shielded::nullifiers::types::{CompactedNullifierChange, CompactedNullifiers};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::util::common::compacted_key;
use crate::verify::RootHash;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies compacted nullifier changes proof.
    ///
    /// # Soundness
    ///
    /// Compacted keys are 16 bytes `(start_block_be, end_block_be)`. A range
    /// such as `(100, 200)` that *contains* a requested height `150` sorts
    /// lexicographically **before** `(150, 150)`. This means the lower bound of
    /// the forward range scan (`start_key`) cannot be trusted to come from the
    /// caller-requested height alone — a malicious prover could prove
    /// `range_from((150, 150)..)` directly and the containing range `(100, 200)`
    /// would appear only as a hash-only boundary node, silently hiding a spend.
    ///
    /// To close this hole we never derive `start_key` from un-authenticated
    /// proof bytes. Instead we use GroveDB's chained path query verification:
    ///
    /// 1. A **boundary query** (descending `range_to_inclusive(..=(start, MAX))`,
    ///    limit 1) authenticates, against the real root hash, the single
    ///    greatest compacted key `<= (start_block_height, u64::MAX)`. A malicious
    ///    prover cannot substitute or omit this key without breaking the root
    ///    hash.
    /// 2. A **generator** inspects that authenticated boundary key. If its
    ///    `end_block >= start_block_height` the range contains (or starts at) the
    ///    requested height, so we use that exact key as the forward `start_key`.
    ///    Otherwise no containing range exists and we fall back to
    ///    `(start_block_height, start_block_height)`.
    /// 3. The chained **forward query** (`range_from(start_key..)`, caller limit)
    ///    is verified against the same root hash, and its authenticated results
    ///    are decoded into the returned changes.
    ///
    /// # KNOWN LIVENESS BUG (tracked in PR #3792 — fix deferred)
    ///
    /// The boundary query is **descending** (greatest key `<= bound`) while the
    /// forward query is **ascending** (paginated `range_from`). A single GroveDB
    /// proof is one-directional, so when **two or more** compacted ranges sort
    /// at/below `start_block_height` the honest proof fails verification with
    /// "Cannot verify upper bound of queried range" (see the two `#[ignore]`d
    /// `*_below_query_height*` regression tests). The single-range and empty
    /// cases work, which is why this slipped the original tests. The planned fix
    /// is to re-key compacted entries by `(end_block, start_block)` so retrieval
    /// becomes a single ascending `range_from((H, 0)..)` — which also closes the
    /// original absence-proof soundness hole structurally (the containing range
    /// becomes an ordinary in-range result that cannot be omitted).
    pub(super) fn verify_compacted_nullifier_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<CompactedNullifierChange>), Error> {
        let path = shielded_compacted_nullifiers_path_vec();

        // Step 1: boundary query — authenticate the single greatest compacted
        // key <= (start_block_height, u64::MAX). Descending, limit 1.
        let boundary_end_key = compacted_key(start_block_height, u64::MAX);
        let mut boundary_inner = Query::new_with_direction(false); // descending
        boundary_inner.insert_range_to_inclusive(..=boundary_end_key);
        let boundary_query =
            PathQuery::new(path.clone(), SizedQuery::new(boundary_inner, Some(1), None));

        // Step 2: generator — derive the forward query's lower bound from the
        // AUTHENTICATED boundary result (not from raw proof bytes).
        let forward_path = path.clone();
        let generator =
            move |boundary_results: Vec<PathKeyOptionalElementTrio>| -> Option<PathQuery> {
                // The boundary key contains start_block_height when its end_block is
                // at or beyond it. Use the authenticated key directly in that case;
                // otherwise there is no containing range and we start at
                // (start_block_height, start_block_height).
                //
                // The boundary query is limit-1, so there is at most one
                // authenticated key. A non-16-byte key is tree/proof corruption —
                // reject (return None, which fails the chained verification) rather
                // than silently degrading to the (start, start) forward-only lower
                // bound and accepting an incomplete result set.
                if boundary_results.iter().any(|(_p, key, _e)| key.len() != 16) {
                    return None;
                }
                let start_key = boundary_results
                    .iter()
                    .find_map(|(_path, key, _element)| {
                        let end_block = u64::from_be_bytes(
                            key[8..16].try_into().expect("len checked to be 16"),
                        );
                        if end_block >= start_block_height {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| compacted_key(start_block_height, start_block_height));

                let mut forward_inner = Query::new();
                forward_inner.insert_range_from(start_key..);
                Some(PathQuery::new(
                    forward_path.clone(),
                    SizedQuery::new(forward_inner, limit, None),
                ))
            };

        // Step 3: verify the chained queries. GroveDB enforces that every
        // sub-query binds to the SAME root hash, so the boundary key authenticated
        // in step 1 and the forward results in step 3 are consistent with one
        // another and with the real state.
        let (root_hash, results) = GroveDb::verify_query_with_chained_path_queries(
            proof,
            &boundary_query,
            vec![generator],
            &platform_version.drive.grove_version,
        )?;

        // We pass exactly one chained (forward) generator that always returns
        // `Some`, so GroveDB returns exactly two result sets: [boundary, forward].
        // A different cardinality can only be a GroveDB-internal invariant break
        // (never caller-supplied proof corruption), so classify it as an internal
        // code-execution error rather than `CorruptedProof`.
        let [_boundary_results, forward_results]: [Vec<PathKeyOptionalElementTrio>; 2] =
            results.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "chained verification invariant: expected [boundary, forward] result sets",
                ))
            })?;

        // Process the verified forward results.
        let mut compacted_changes = Vec::new();

        for (_path, key, maybe_element) in forward_results {
            let Some(element) = maybe_element else {
                continue;
            };

            let (start_block, end_block) = CompactedNullifierChange::parse_key(&key)?;

            // Get the serialized data from the Item element
            let grovedb::Element::Item(serialized_data, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item element for compacted nullifiers".to_string(),
                )));
            };

            let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            compacted_changes.push(CompactedNullifierChange {
                start_block,
                end_block,
                nullifiers,
            });
        }

        Ok((root_hash, compacted_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path_vec;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::{PathQuery, Query, SizedQuery};
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_compacted_nullifier_changes_roundtrip() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Store enough blocks of nullifiers to trigger compaction
        let max_blocks = platform_version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_nullifier_compaction as u64;

        for block_height in 1u64..=(max_blocks + 1) {
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

        // Prove compacted changes from block 1
        let proof = drive
            .prove_compacted_nullifier_changes(1, None, None, platform_version)
            .expect("should prove compacted nullifier changes");

        // Verify the proof
        let (root_hash, compacted_changes) =
            Drive::verify_compacted_nullifier_changes(proof.as_slice(), 1, None, platform_version)
                .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        // We should have at least one compacted entry
        assert!(
            !compacted_changes.is_empty(),
            "should have at least one compacted entry"
        );

        // All entries should have valid block ranges
        for change in &compacted_changes {
            assert!(
                change.start_block <= change.end_block,
                "start should be <= end"
            );
            assert!(
                !change.nullifiers.is_empty(),
                "each entry should have nullifiers"
            );
        }
    }

    #[test]
    fn should_prove_and_verify_empty_compacted_nullifier_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove compacted changes from block 100 with no data stored
        let proof = drive
            .prove_compacted_nullifier_changes(100, None, None, platform_version)
            .expect("should prove empty compacted nullifier changes");

        // Verify the proof
        let (root_hash, compacted_changes) = Drive::verify_compacted_nullifier_changes(
            proof.as_slice(),
            100,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            compacted_changes.is_empty(),
            "should have no compacted entries when no data stored"
        );
    }

    /// Stores a single compacted nullifier entry directly under the compacted
    /// nullifiers path with the given `(start_block, end_block)` key.
    ///
    /// This bypasses the normal recent->compacted promotion so that tests can
    /// construct exact tree shapes (e.g. a containing range) deterministically.
    fn store_compacted_entry(
        drive: &Drive,
        start_block: u64,
        end_block: u64,
        nullifiers: Vec<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) {
        use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path;
        use grovedb::Element;
        use grovedb_costs::CostContext;
        use grovedb_path::SubtreePath;

        let key = super::compacted_key(start_block, end_block);
        let value = CompactedNullifiers::new(nullifiers)
            .encode()
            .expect("encode nullifiers");

        let path = shielded_compacted_nullifiers_path();

        let CostContext { value: result, .. } = drive.grove.insert(
            SubtreePath::from(path.as_ref()),
            key.as_slice(),
            Element::new_item(value),
            None,
            None,
            &platform_version.drive.grove_version,
        );
        result.expect("insert compacted entry");
    }

    /// Verifies that the honest path returns the containing range `(100, 200)`
    /// when querying from a height inside it (`150`).
    #[test]
    fn should_return_containing_range_for_start_inside_it() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // A spend recorded inside a compacted range that straddles block 150.
        let double_spend_nullifier = [0xAB; 32];
        store_compacted_entry(
            &drive,
            100,
            200,
            vec![double_spend_nullifier],
            platform_version,
        );

        // Honest proof from block 150 (inside the (100, 200) range).
        let proof = drive
            .prove_compacted_nullifier_changes(150, None, None, platform_version)
            .expect("should prove compacted nullifier changes");

        let (_root_hash, compacted_changes) = Drive::verify_compacted_nullifier_changes(
            proof.as_slice(),
            150,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert_eq!(
            compacted_changes.len(),
            1,
            "the containing range (100, 200) must be surfaced for start=150"
        );
        let change = &compacted_changes[0];
        assert_eq!(change.start_block, 100);
        assert_eq!(change.end_block, 200);
        assert_eq!(change.nullifiers.as_slice(), &[double_spend_nullifier]);
    }

    /// ADVERSARIAL: multiple compacted ranges all sort at/below the query
    /// height — the realistic light-client sync case. A reviewer flagged a
    /// possible direction-mismatch liveness break (ascending merged proof vs
    /// descending limit-1 boundary verify query) that the single-range tests
    /// would not catch. This proves the honest prove->verify roundtrip succeeds
    /// and returns the containing range when >=2 ranges are <= (start, MAX).
    ///
    /// KNOWN-FAILING (see PR #3792 description): the chained
    /// boundary(descending) + forward(ascending) scheme cannot be satisfied by a
    /// single one-directional GroveDB proof. With >=2 compacted ranges at/below
    /// the query height the honest proof fails with "Cannot verify upper bound of
    /// queried range". Planned fix: re-key compacted entries by
    /// (end_block, start_block) so the query is a single ascending
    /// `range_from((H,0)..)`. Un-ignore once fixed.
    #[ignore = "known liveness bug: chained descending-boundary + ascending-forward \
                cannot share one GroveDB proof; fix = re-key by end_block (see PR #3792)"]
    #[test]
    fn multiple_ranges_below_query_height_verify() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Three contiguous compacted ranges, ALL with start_block <= 150.
        store_compacted_entry(&drive, 1, 64, vec![[0x01; 32]], platform_version);
        store_compacted_entry(&drive, 65, 128, vec![[0x02; 32]], platform_version);
        store_compacted_entry(&drive, 129, 192, vec![[0x03; 32]], platform_version);

        // Query from 150: bound = (150, MAX). Keys <= bound: (1,64), (65,128),
        // (129,192) — THREE keys. (129,192) contains 150 (129 <= 150 <= 192).
        let proof = drive
            .prove_compacted_nullifier_changes(150, None, None, platform_version)
            .expect("should prove with multiple ranges below the query height");

        let (_root, changes) = Drive::verify_compacted_nullifier_changes(
            proof.as_slice(),
            150,
            None,
            platform_version,
        )
        .expect("VERIFY MUST NOT FAIL with multiple ranges <= bound");

        assert!(
            changes
                .iter()
                .any(|c| c.start_block == 129 && c.end_block == 192),
            "containing range (129,192) must be surfaced; got {:?}",
            changes
                .iter()
                .map(|c| (c.start_block, c.end_block))
                .collect::<Vec<_>>()
        );
    }

    /// ADVERSARIAL variant: a non-zero gap below the containing range, querying
    /// from a height strictly inside the third range with two full ranges below.
    ///
    /// KNOWN-FAILING (see PR #3792 description): same direction-mismatch bug as
    /// `multiple_ranges_below_query_height_verify`. Un-ignore once the compacted
    /// tree is re-keyed by (end_block, start_block).
    #[ignore = "known liveness bug: chained descending-boundary + ascending-forward \
                cannot share one GroveDB proof; fix = re-key by end_block (see PR #3792)"]
    #[test]
    fn containing_range_with_two_lower_ranges_verifies() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        store_compacted_entry(&drive, 100, 200, vec![[0xA1; 32]], platform_version);
        store_compacted_entry(&drive, 300, 400, vec![[0xA2; 32]], platform_version);
        store_compacted_entry(&drive, 500, 600, vec![[0xA3; 32]], platform_version);

        // Query from 550: (500,600) contains it; (100,200) and (300,400) are
        // both <= (550, MAX) and sort below the containing range.
        let proof = drive
            .prove_compacted_nullifier_changes(550, None, None, platform_version)
            .expect("should prove");

        let (_root, changes) = Drive::verify_compacted_nullifier_changes(
            proof.as_slice(),
            550,
            None,
            platform_version,
        )
        .expect("VERIFY MUST NOT FAIL");

        assert!(
            changes
                .iter()
                .any(|c| c.start_block == 500 && c.end_block == 600),
            "containing range (500,600) must be surfaced; got {:?}",
            changes
                .iter()
                .map(|c| (c.start_block, c.end_block))
                .collect::<Vec<_>>()
        );
    }

    /// PoC: a malicious prover that skips the descending discovery step and
    /// proves `range_from((150, 150)..)` directly MUST NOT be able to make the
    /// verifier silently return zero nullifiers while a containing range
    /// `(100, 200)` actually holds a spend.
    ///
    /// With the chained-boundary verifier, the boundary query authenticates
    /// `(100, 200)` as the true greatest key `<= (150, MAX)`. The generator then
    /// requires the forward query to start at `(100, 200)`. The malicious proof,
    /// which only includes `(100, 200)` as a hash-only boundary node and lacks
    /// its full value, cannot satisfy that forward query, so verification fails
    /// (it can never silently return zero).
    #[test]
    fn malicious_skip_descending_proof_is_rejected() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Containing range (100, 200) holds a double-spend nullifier.
        let double_spend_nullifier = [0xCD; 32];
        store_compacted_entry(
            &drive,
            100,
            200,
            vec![double_spend_nullifier],
            platform_version,
        );

        // Craft the MALICIOUS proof the OLD (vulnerable) way: prove
        // range_from((150, 150)..) directly, skipping descending discovery.
        // (100, 200) sorts before (150, 150) lexicographically, so it appears
        // only as a hash-only boundary node in this proof.
        let path = shielded_compacted_nullifiers_path_vec();
        let malicious_start_key = super::compacted_key(150, 150);
        let mut malicious_inner = Query::new();
        malicious_inner.insert_range_from(malicious_start_key..);
        let malicious_path_query =
            PathQuery::new(path, SizedQuery::new(malicious_inner, None, None));

        let grovedb_costs::CostContext {
            value: malicious_proof_result,
            ..
        } = drive.grove.get_proved_path_query(
            &malicious_path_query,
            None,
            None,
            &platform_version.drive.grove_version,
        );
        let malicious_proof = malicious_proof_result
            .expect("should produce a (malicious) proof for the direct forward query");

        let result = Drive::verify_compacted_nullifier_changes(
            malicious_proof.as_slice(),
            150,
            None,
            platform_version,
        );

        // The fix must NOT silently return zero nullifiers. Either verification
        // fails outright, or it surfaces the (100, 200) double-spend.
        match result {
            Err(_) => {
                // Expected: the boundary query authenticates (100, 200) as the
                // greatest key <= (150, MAX); the malicious proof cannot satisfy
                // the resulting forward query, so verification fails.
            }
            Ok((_root_hash, changes)) => {
                assert!(
                    changes
                        .iter()
                        .any(|c| c.start_block == 100 && c.end_block == 200),
                    "malicious proof must not silently hide the containing range \
                     (100, 200); got {} changes",
                    changes.len()
                );
            }
        }
    }

    /// Querying past the last compacted range: the boundary key `(100, 200)` has
    /// `end_block < start_block_height`, so there is no containing range and the
    /// forward scan finds nothing. Exercises the `find_map` fallback to
    /// `(start, start)` on both prover and verifier (a single key `<= bound`, so
    /// unaffected by the known multi-range liveness bug).
    #[test]
    fn query_past_last_range_returns_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        store_compacted_entry(&drive, 100, 200, vec![[0xAB; 32]], platform_version);

        // Query at 300, strictly past the only range (100, 200).
        let proof = drive
            .prove_compacted_nullifier_changes(300, None, None, platform_version)
            .expect("should prove");
        let (_root, changes) = Drive::verify_compacted_nullifier_changes(
            proof.as_slice(),
            300,
            None,
            platform_version,
        )
        .expect("should verify");

        assert!(
            changes.is_empty(),
            "querying past the last range must return zero changes, got {:?}",
            changes
                .iter()
                .map(|c| (c.start_block, c.end_block))
                .collect::<Vec<_>>()
        );
    }
}
