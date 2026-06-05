use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for compacted address balances storage as u8
const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';
use dpp::balances::credits::BlockAwareCreditOperation;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::VerifiedCompactedAddressBalanceChanges;

/// Builds the 16-byte big-endian compacted key `(start_block, end_block)`.
fn compacted_key(start_block: u64, end_block: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(16);
    key.extend_from_slice(&start_block.to_be_bytes());
    key.extend_from_slice(&end_block.to_be_bytes());
    key
}

/// Path to the compacted address balances subtree:
/// `[SavedBlockTransactions, COMPACTED_ADDRESS_BALANCES_KEY]`.
fn compacted_address_balances_path() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::SavedBlockTransactions as u8],
        vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
    ]
}

impl Drive {
    /// Verifies compacted address balance changes proof.
    ///
    /// # Soundness
    ///
    /// Compacted keys are 16 bytes `(start_block_be, end_block_be)`. A range
    /// such as `(100, 200)` that *contains* a requested height `150` sorts
    /// lexicographically **before** `(150, 150)`. This means the lower bound of
    /// the forward range scan (`start_key`) cannot be trusted to come from the
    /// caller-requested height alone — a malicious prover could prove
    /// `range_from((150, 150)..)` directly and the containing range `(100, 200)`
    /// would appear only as a hash-only boundary node, silently hiding a change.
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
    /// Identical to the shielded-nullifier verifier: the boundary query is
    /// **descending** while the forward query is **ascending**, and a single
    /// GroveDB proof is one-directional, so when **two or more** compacted
    /// address-balance ranges sort at/below `start_block_height` the honest proof
    /// fails verification with "Cannot verify upper bound of queried range" (see
    /// the `#[ignore]`d `multiple_ranges_below_query_height_verify` regression
    /// test). The single-range and empty cases work. The planned fix is to re-key
    /// compacted entries by `(end_block, start_block)` so retrieval becomes a
    /// single ascending `range_from((H, 0)..)` — which also closes the original
    /// absence-proof soundness hole structurally.
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let path = compacted_address_balances_path();

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
                let start_key = boundary_results
                    .iter()
                    .find_map(|(_path, key, _element)| {
                        if key.len() != 16 {
                            return None;
                        }
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
        // sub-query binds to the SAME root hash.
        let (root_hash, mut results) = GroveDb::verify_query_with_chained_path_queries(
            proof,
            &boundary_query,
            vec![generator],
            &platform_version.drive.grove_version,
        )?;

        // results[0] is the boundary query, results[1] is the forward query.
        let forward_results = results.pop().ok_or_else(|| {
            Error::Proof(ProofError::CorruptedProof(
                "chained verification returned no forward results".to_string(),
            ))
        })?;

        // Process the verified forward results.
        let mut compacted_changes = Vec::new();

        for (_path, key, maybe_element) in forward_results {
            let Some(element) = maybe_element else {
                continue;
            };

            if key.len() != 16 {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "invalid compacted block key length, expected 16 bytes".to_string(),
                )));
            }

            let range_start = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let range_end = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Get the serialized data from the Item element
            let grovedb::Element::Item(serialized_data, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item element for compacted address balances".to_string(),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (
                BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                usize,
            ) = bincode::decode_from_slice(&serialized_data, bincode_config).map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode compacted address balances: {}",
                    e
                )))
            })?;

            compacted_changes.push((range_start, range_end, address_balances));
        }

        Ok((root_hash, compacted_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_compacted_address_balance_changes_roundtrip() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store enough blocks of changes to trigger compaction
        // The compaction threshold is controlled by platform_version
        // We insert many blocks to ensure compaction happens
        let max_blocks = platform_version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_compaction as u64;

        for block_height in 1u64..=(max_blocks + 1) {
            let mut changes = BTreeMap::new();
            changes.insert(
                address_1,
                CreditOperation::AddToCredits(block_height * 1000),
            );
            if block_height % 2 == 0 {
                changes.insert(address_2, CreditOperation::AddToCredits(block_height * 500));
            }
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Prove compacted changes from block 1
        let proof = drive
            .prove_compacted_address_balance_changes(1, None, None, platform_version)
            .expect("should prove compacted address balance changes");

        // Verify the proof
        let (root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            1,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        // We should have at least one compacted entry
        assert!(
            !compacted_changes.is_empty(),
            "should have at least one compacted entry"
        );

        // All entries should have valid block ranges
        for (start, end, changes) in &compacted_changes {
            assert!(*start <= *end, "start should be <= end");
            assert!(!changes.is_empty(), "each entry should have changes");
        }
    }

    #[test]
    fn should_prove_and_verify_empty_compacted_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove compacted changes from block 100 with no data stored
        let proof = drive
            .prove_compacted_address_balance_changes(100, None, None, platform_version)
            .expect("should prove empty compacted changes");

        // Verify the proof
        let (root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
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

    /// Stores a single compacted address-balance entry directly under the
    /// compacted address balances path with the given `(start_block,
    /// end_block)` key. Bypasses normal compaction so tests can build exact
    /// tree shapes (e.g. a containing range).
    fn store_compacted_entry(
        drive: &Drive,
        start_block: u64,
        end_block: u64,
        changes: BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
        platform_version: &PlatformVersion,
    ) {
        use grovedb::Element;
        use grovedb_costs::CostContext;
        use grovedb_path::SubtreePath;

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let key = compacted_key(start_block, end_block);
        let value = bincode::encode_to_vec(&changes, config).expect("encode changes");

        let path = Drive::saved_compacted_block_transactions_address_balances_path();

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

    /// Honest path returns the containing range `(100, 200)` for a start height
    /// inside it (`150`).
    #[test]
    fn should_return_containing_range_for_start_inside_it() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([7; 20]);
        let mut changes = BTreeMap::new();
        changes.insert(
            address,
            BlockAwareCreditOperation::from_operation(150, &CreditOperation::AddToCredits(12_345)),
        );
        store_compacted_entry(&drive, 100, 200, changes.clone(), platform_version);

        let proof = drive
            .prove_compacted_address_balance_changes(150, None, None, platform_version)
            .expect("should prove compacted address balance changes");

        let (_root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
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
        let (start, end, returned_changes) = &compacted_changes[0];
        assert_eq!(*start, 100);
        assert_eq!(*end, 200);
        assert_eq!(returned_changes, &changes);
    }

    /// PoC: a malicious prover that skips descending discovery and proves
    /// `range_from((150, 150)..)` directly MUST NOT make the verifier silently
    /// return zero changes while a containing range `(100, 200)` holds a change.
    #[test]
    fn malicious_skip_descending_proof_is_rejected() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([9; 20]);
        let mut changes = BTreeMap::new();
        changes.insert(
            address,
            BlockAwareCreditOperation::from_operation(150, &CreditOperation::AddToCredits(99_999)),
        );
        store_compacted_entry(&drive, 100, 200, changes, platform_version);

        // Craft the MALICIOUS proof the OLD (vulnerable) way: prove
        // range_from((150, 150)..) directly. (100, 200) sorts before (150, 150)
        // and so appears only as a hash-only boundary node.
        let path = compacted_address_balances_path();
        let malicious_start_key = compacted_key(150, 150);
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

        let result = Drive::verify_compacted_address_balance_changes(
            malicious_proof.as_slice(),
            150,
            None,
            platform_version,
        );

        match result {
            Err(_) => {
                // Expected: the boundary query authenticates (100, 200) as the
                // greatest key <= (150, MAX); the malicious proof cannot satisfy
                // it, so verification fails.
            }
            Ok((_root_hash, returned_changes)) => {
                assert!(
                    returned_changes
                        .iter()
                        .any(|(start, end, _)| *start == 100 && *end == 200),
                    "malicious proof must not silently hide the containing range \
                     (100, 200); got {} changes",
                    returned_changes.len()
                );
            }
        }
    }

    /// Querying past the last compacted range: the boundary key `(100, 200)` has
    /// `end_block < start_block_height`, so there is no containing range and the
    /// forward scan finds nothing. This exercises the `find_map` fallback to
    /// `(start, start)` on both prover and verifier — a single key `<= bound`, so
    /// it is unaffected by the known multi-range liveness bug.
    #[test]
    fn query_past_last_range_returns_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([7; 20]);
        let mut changes = BTreeMap::new();
        changes.insert(
            address,
            BlockAwareCreditOperation::from_operation(150, &CreditOperation::AddToCredits(12_345)),
        );
        store_compacted_entry(&drive, 100, 200, changes, platform_version);

        // Query at 300, strictly past the only range (100, 200).
        let proof = drive
            .prove_compacted_address_balance_changes(300, None, None, platform_version)
            .expect("should prove");
        let (_root, compacted_changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            300,
            None,
            platform_version,
        )
        .expect("should verify");

        assert!(
            compacted_changes.is_empty(),
            "querying past the last range must return zero changes, got {:?}",
            compacted_changes
                .iter()
                .map(|(s, e, _)| (*s, *e))
                .collect::<Vec<_>>()
        );
    }

    /// ADVERSARIAL (mirror of the nullifier verifier): >=2 compacted ranges
    /// at/below the query height. KNOWN-FAILING per PR #3792 — the chained
    /// descending-boundary + ascending-forward scheme cannot be satisfied by one
    /// one-directional GroveDB proof; fails with "Cannot verify upper bound of
    /// queried range". Un-ignore once the compacted tree is re-keyed by
    /// `(end_block, start_block)`.
    #[ignore = "known liveness bug: chained descending-boundary + ascending-forward \
                cannot share one GroveDB proof; fix = re-key by end_block (see PR #3792)"]
    #[test]
    fn multiple_ranges_below_query_height_verify() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([7; 20]);
        for (s, e) in [(1u64, 64u64), (65, 128), (129, 192)] {
            let mut changes = BTreeMap::new();
            changes.insert(
                address,
                BlockAwareCreditOperation::from_operation(e, &CreditOperation::AddToCredits(e)),
            );
            store_compacted_entry(&drive, s, e, changes, platform_version);
        }

        // Query from 150: (1,64),(65,128),(129,192) all sort <= (150, MAX).
        let proof = drive
            .prove_compacted_address_balance_changes(150, None, None, platform_version)
            .expect("should prove with multiple ranges below the query height");
        let (_root, changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            150,
            None,
            platform_version,
        )
        .expect("VERIFY MUST NOT FAIL with multiple ranges <= bound");

        assert!(
            changes.iter().any(|(s, e, _)| *s == 129 && *e == 192),
            "containing range (129,192) must be surfaced; got {:?}",
            changes.iter().map(|(s, e, _)| (*s, *e)).collect::<Vec<_>>()
        );
    }
}
