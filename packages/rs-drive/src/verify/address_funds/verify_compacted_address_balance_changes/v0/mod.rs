use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for compacted address balances storage as u8
const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';
/// Standalone decode budget for the two-proof envelope. Not derived from any
/// transport limit (tonic clients default to a 4 MiB response cap and the
/// DAPI server encodes at most 32 MiB): it only needs to sit far above any
/// realistic proof while bounding hostile allocations before GroveDB
/// verification runs.
const MAX_COMPACTED_PROOF_DECODE_BYTES: usize = 16 * 1024 * 1024;
/// A compacted row contains at most one configured address chunk. Keep a
/// separate semantic-object budget after the GroveDB envelope is decoded.
const MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES: usize = 1024 * 1024;
use dpp::balances::credits::BlockAwareCreditOperation;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::{CompactedAddressBalanceProof, VerifiedCompactedAddressBalanceChanges};

impl Drive {
    /// Verifies compacted address balance changes proof.
    ///
    /// The request-derived predecessor query is verified first. Its
    /// authenticated result selects the start key for a separately verified
    /// forward query, and both proofs must commit to the same root.
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        if proof.len() > MAX_COMPACTED_PROOF_DECODE_BYTES {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "compacted address balance proof exceeds the decoding limit".to_string(),
            )));
        }

        let proof_decode_config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<MAX_COMPACTED_PROOF_DECODE_BYTES>();

        let (proof_envelope, consumed): (CompactedAddressBalanceProof, usize) =
            bincode::decode_from_slice(proof, proof_decode_config).map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode compacted address balance proof: {}",
                    e
                )))
            })?;
        if consumed != proof.len() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "compacted address balance proof contains trailing bytes".to_string(),
            )));
        }

        let path = vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
        ];

        let mut predecessor_end_key = Vec::with_capacity(16);
        predecessor_end_key.extend_from_slice(&start_block_height.to_be_bytes());
        predecessor_end_key.extend_from_slice(&u64::MAX.to_be_bytes());
        let mut predecessor_query = Query::new_with_direction(false);
        predecessor_query.insert_range_to_inclusive(..=predecessor_end_key);
        let predecessor_path_query = PathQuery::new(
            path.clone(),
            SizedQuery::new(predecessor_query, Some(1), None),
        );
        let (predecessor_root_hash, predecessor_results) = GroveDb::verify_query(
            &proof_envelope.predecessor_proof,
            &predecessor_path_query,
            &platform_version.drive.grove_version,
        )?;

        let mut authenticated_predecessors = predecessor_results
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)));
        let authenticated_predecessor = authenticated_predecessors.next();
        if authenticated_predecessors.next().is_some() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "predecessor proof returned more than one compacted range".to_string(),
            )));
        }

        let start_key = if let Some((key, _element)) = authenticated_predecessor {
            let key_bytes: [u8; 16] = key.as_slice().try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "invalid compacted predecessor key length".to_string(),
                ))
            })?;
            let range_start = u64::from_be_bytes(key_bytes[..8].try_into().expect("key length"));
            let range_end = u64::from_be_bytes(key_bytes[8..].try_into().expect("key length"));
            if range_start > range_end {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "compacted predecessor has an invalid block range".to_string(),
                )));
            }
            if range_end >= start_block_height {
                key
            } else {
                let mut fallback = Vec::with_capacity(16);
                fallback.extend_from_slice(&start_block_height.to_be_bytes());
                fallback.extend_from_slice(&start_block_height.to_be_bytes());
                fallback
            }
        } else {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        };

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = GroveDb::verify_query(
            &proof_envelope.forward_proof,
            &path_query,
            &platform_version.drive.grove_version,
        )?;
        if root_hash != predecessor_root_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "compacted address balance proofs commit to different roots".to_string(),
            )));
        }

        // Process the verified results
        let mut compacted_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
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
            if serialized_data.len() > MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "compacted address balance row exceeds the decoding limit".to_string(),
                )));
            }
            let row_decode_config = bincode::config::standard()
                .with_big_endian()
                .with_limit::<MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES>();
            let (address_balances, consumed): (
                BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                usize,
            ) = bincode::decode_from_slice(&serialized_data, row_decode_config).map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode compacted address balances: {}",
                    e
                )))
            })?;
            if consumed != serialized_data.len() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "compacted address balance row contains trailing bytes".to_string(),
                )));
            }

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
    use dpp::balances::credits::CreditOperation;
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

        // A query beginning inside a compacted range must include that range.
        // This exercises the independently authenticated predecessor witness.
        let interior_height = max_blocks / 2;
        let interior_proof = drive
            .prove_compacted_address_balance_changes(interior_height, None, None, platform_version)
            .expect("should prove changes from inside a compacted range");
        let (_, interior_changes) = Drive::verify_compacted_address_balance_changes(
            &interior_proof,
            interior_height,
            None,
            platform_version,
        )
        .expect("should verify the authenticated predecessor proof");
        assert!(
            interior_changes
                .iter()
                .any(|(start, end, _)| *start <= interior_height && interior_height <= *end),
            "the compacted range containing the requested height must be returned"
        );
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

    /// Stores enough per-block changes to trigger compaction, leaving at
    /// least one compacted range and one uncompacted recent block.
    fn setup_drive_with_compacted_ranges() -> (Drive, u64) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let address = PlatformAddress::P2pkh([10; 20]);
        let max_blocks = platform_version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_compaction as u64;

        for block_height in 1u64..=(max_blocks + 1) {
            let mut changes = BTreeMap::new();
            changes.insert(address, CreditOperation::AddToCredits(block_height * 1000));
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

        (drive, max_blocks)
    }

    #[test]
    fn beyond_last_compacted_range_uses_fallback_and_returns_empty() {
        // Exercises the predecessor-exists-but-does-not-contain branch: a
        // compacted range sits below the requested height, so the verifier
        // must fall back to the request-derived start key instead of the
        // authenticated predecessor key.
        let (drive, max_blocks) = setup_drive_with_compacted_ranges();
        let platform_version = PlatformVersion::latest();

        // Precondition: compaction produced at least one range below.
        let proof = drive
            .prove_compacted_address_balance_changes(1, None, None, platform_version)
            .expect("should prove from genesis");
        let (_, changes) =
            Drive::verify_compacted_address_balance_changes(&proof, 1, None, platform_version)
                .expect("should verify from genesis");
        assert!(!changes.is_empty(), "compaction must have produced ranges");

        let beyond_height = max_blocks * 10;
        let beyond_proof = drive
            .prove_compacted_address_balance_changes(beyond_height, None, None, platform_version)
            .expect("should prove beyond the last compacted range");
        let (_, beyond_changes) = Drive::verify_compacted_address_balance_changes(
            &beyond_proof,
            beyond_height,
            None,
            platform_version,
        )
        .expect("a non-containing predecessor must verify via the fallback key");
        assert!(
            beyond_changes.is_empty(),
            "no compacted range lies at or beyond the requested height"
        );
    }

    /// Stores a single compacted address-balance entry directly under the
    /// compacted address balances path with the given `(start_block,
    /// end_block)` key. Bypasses normal compaction so tests can build exact
    /// tree shapes (e.g. several ranges at/below the queried height).
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

        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&start_block.to_be_bytes());
        key.extend_from_slice(&end_block.to_be_bytes());
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

    fn single_change_for(
        end_block: u64,
        credits: u64,
    ) -> BTreeMap<PlatformAddress, BlockAwareCreditOperation> {
        let mut changes = BTreeMap::new();
        changes.insert(
            PlatformAddress::P2pkh([7; 20]),
            BlockAwareCreditOperation::from_operation(
                end_block,
                &CreditOperation::AddToCredits(credits),
            ),
        );
        changes
    }

    /// Regression for the chained-proof liveness bug that blocked the
    /// original soundness fix: with two or more compacted ranges sorting
    /// at/below the queried height — the normal paginated-sync shape — a
    /// single one-directional proof could not satisfy both the descending
    /// predecessor query and the ascending forward query. The two-proof
    /// envelope must verify the honest roundtrip.
    #[test]
    fn multiple_ranges_below_query_height_verify() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let containing_changes = single_change_for(192, 192_000);
        for (start, end) in [(1u64, 64u64), (65, 128), (129, 192)] {
            let changes = if end == 192 {
                containing_changes.clone()
            } else {
                single_change_for(end, end * 1000)
            };
            store_compacted_entry(&drive, start, end, changes, platform_version);
        }

        // All three ranges sort at/below (150, u64::MAX).
        let proof = drive
            .prove_compacted_address_balance_changes(150, None, None, platform_version)
            .expect("should prove with multiple ranges below the query height");
        let (_, changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            150,
            None,
            platform_version,
        )
        .expect("verification must not fail with multiple ranges at/below the bound");

        assert_eq!(
            changes
                .iter()
                .map(|(start, end, _)| (*start, *end))
                .collect::<Vec<_>>(),
            vec![(129, 192)],
            "only the range containing the requested height must be returned"
        );
        assert_eq!(
            changes[0].2, containing_changes,
            "the containing range must carry its stored changes"
        );
    }

    /// The forward scan starts at the authenticated containing range and
    /// continues through later ranges; ranges wholly below the queried
    /// height stay excluded even though they sort below the predecessor
    /// bound.
    #[test]
    fn containing_range_with_lower_and_higher_ranges() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for (start, end) in [(1u64, 64u64), (65, 128), (129, 192), (193, 256)] {
            store_compacted_entry(
                &drive,
                start,
                end,
                single_change_for(end, end * 1000),
                platform_version,
            );
        }

        let proof = drive
            .prove_compacted_address_balance_changes(150, None, None, platform_version)
            .expect("should prove with ranges on both sides of the query height");
        let (_, changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            150,
            None,
            platform_version,
        )
        .expect("should verify");

        assert_eq!(
            changes
                .iter()
                .map(|(start, end, _)| (*start, *end))
                .collect::<Vec<_>>(),
            vec![(129, 192), (193, 256)],
            "the containing range and every later range must be returned in order"
        );
    }

    /// Mirrors the paginated sync the gRPC handler performs (it passes an
    /// explicit limit): the caller limit truncates the forward scan, and the
    /// next page resumes past the last returned range — including a final
    /// page that exercises the non-containing-predecessor fallback under a
    /// limit.
    #[test]
    fn caller_limit_paginates_across_ranges() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for (start, end) in [(1u64, 64u64), (65, 128), (129, 192), (193, 256)] {
            store_compacted_entry(
                &drive,
                start,
                end,
                single_change_for(end, end * 1000),
                platform_version,
            );
        }

        let page = |start_height: u64| -> Vec<(u64, u64)> {
            let proof = drive
                .prove_compacted_address_balance_changes(
                    start_height,
                    Some(1),
                    None,
                    platform_version,
                )
                .expect("should prove a limited page");
            let (_, changes) = Drive::verify_compacted_address_balance_changes(
                proof.as_slice(),
                start_height,
                Some(1),
                platform_version,
            )
            .expect("a limited page must verify");
            changes
                .iter()
                .map(|(start, end, _)| (*start, *end))
                .collect()
        };

        assert_eq!(
            page(150),
            vec![(129, 192)],
            "first page returns only the containing range"
        );
        assert_eq!(
            page(193),
            vec![(193, 256)],
            "second page resumes at the next range"
        );
        assert_eq!(
            page(257),
            Vec::<(u64, u64)>::new(),
            "past the last range the fallback start key yields an empty page"
        );
    }

    #[test]
    fn rejects_envelope_halves_committing_to_different_roots() {
        // Splice a predecessor proof from one state with a forward proof from
        // a later state. Each half verifies on its own, so only the explicit
        // cross-proof root binding can reject the mixed envelope.
        let (drive, max_blocks) = setup_drive_with_compacted_ranges();
        let platform_version = PlatformVersion::latest();
        let interior_height = max_blocks / 2;

        let proof_before = drive
            .prove_compacted_address_balance_changes(interior_height, None, None, platform_version)
            .expect("should prove at the earlier state");

        // Mutate uncompacted recent state only: the root changes while the
        // compacted ranges (and therefore the derived start key) do not.
        let address = PlatformAddress::P2pkh([10; 20]);
        let mut changes = BTreeMap::new();
        changes.insert(address, CreditOperation::AddToCredits(1));
        drive
            .store_address_balances_for_block(
                &changes,
                max_blocks + 2,
                (max_blocks + 2) * 1000,
                None,
                platform_version,
            )
            .expect("should store an additional recent block");

        let proof_after = drive
            .prove_compacted_address_balance_changes(interior_height, None, None, platform_version)
            .expect("should prove at the later state");

        let envelope_config = bincode::config::standard().with_big_endian();
        let (before_envelope, _): (CompactedAddressBalanceProof, usize) =
            bincode::decode_from_slice(&proof_before, envelope_config)
                .expect("decode earlier envelope");
        let (after_envelope, _): (CompactedAddressBalanceProof, usize) =
            bincode::decode_from_slice(&proof_after, envelope_config)
                .expect("decode later envelope");

        let spliced = bincode::encode_to_vec(
            CompactedAddressBalanceProof {
                predecessor_proof: before_envelope.predecessor_proof,
                forward_proof: after_envelope.forward_proof,
            },
            envelope_config,
        )
        .expect("encode spliced envelope");

        let error = Drive::verify_compacted_address_balance_changes(
            &spliced,
            interior_height,
            None,
            platform_version,
        )
        .expect_err("mixed-root envelope halves must be rejected");
        assert!(
            error.to_string().contains("different roots"),
            "expected the cross-proof root binding to reject the envelope, got: {error}"
        );
    }

    #[test]
    fn rejects_legacy_single_compacted_address_balance_proof() {
        // This proof was generated with start_block_height = 329
        // Path: [[36], [99]] = [['$'], ['c']] = SavedBlockTransactions -> CompactedAddressBalances
        // Query: RangeTo(..[0, 0, 0, 0, 0, 0, 1, 73, 0, 0, 0, 0, 0, 0, 1, 73]) = RangeTo(..(329, 329))
        let proof: Vec<u8> = vec![
            0, 251, 1, 24, 1, 24, 215, 122, 183, 233, 12, 7, 14, 37, 119, 175, 74, 229, 6, 76, 88,
            209, 79, 175, 135, 230, 45, 89, 116, 17, 76, 49, 87, 242, 4, 40, 35, 2, 33, 229, 84,
            218, 198, 132, 136, 197, 212, 253, 180, 247, 125, 228, 176, 179, 248, 100, 19, 202,
            143, 20, 196, 132, 112, 114, 44, 43, 11, 174, 49, 96, 16, 4, 1, 36, 0, 5, 2, 1, 1, 101,
            0, 126, 152, 206, 30, 157, 147, 101, 232, 212, 68, 50, 242, 52, 170, 136, 72, 39, 136,
            235, 41, 105, 28, 199, 93, 222, 168, 227, 241, 80, 234, 2, 185, 2, 120, 2, 233, 247,
            175, 89, 203, 114, 37, 58, 187, 95, 169, 151, 247, 245, 82, 228, 73, 77, 131, 5, 100,
            241, 7, 152, 139, 156, 94, 138, 33, 173, 16, 2, 124, 156, 122, 25, 79, 47, 39, 233, 96,
            90, 9, 165, 232, 130, 103, 245, 113, 145, 31, 183, 102, 94, 40, 86, 140, 146, 128, 172,
            85, 184, 13, 220, 16, 1, 48, 30, 57, 216, 246, 121, 172, 45, 163, 129, 189, 9, 238,
            108, 64, 21, 231, 140, 164, 160, 37, 184, 182, 34, 151, 148, 194, 74, 83, 124, 199, 87,
            17, 17, 2, 199, 32, 12, 6, 52, 58, 219, 92, 42, 60, 69, 132, 209, 186, 193, 248, 18,
            33, 26, 78, 216, 110, 114, 103, 202, 56, 45, 175, 163, 68, 139, 6, 16, 1, 62, 159, 56,
            33, 169, 193, 143, 7, 131, 179, 169, 31, 163, 163, 50, 117, 235, 211, 94, 247, 244, 60,
            246, 149, 186, 142, 105, 187, 230, 247, 212, 141, 17, 1, 1, 36, 125, 4, 1, 99, 0, 20,
            2, 1, 16, 0, 0, 0, 0, 0, 0, 1, 4, 0, 0, 0, 0, 0, 0, 1, 8, 0, 97, 206, 71, 247, 121, 93,
            103, 7, 209, 119, 82, 59, 209, 145, 171, 254, 112, 14, 204, 81, 30, 98, 213, 203, 146,
            141, 32, 167, 232, 34, 0, 37, 2, 170, 200, 228, 36, 229, 197, 116, 242, 100, 137, 25,
            37, 45, 57, 56, 2, 38, 8, 75, 144, 250, 71, 108, 90, 106, 133, 2, 231, 236, 42, 149,
            206, 16, 1, 77, 52, 100, 69, 203, 109, 100, 190, 84, 2, 6, 238, 168, 74, 208, 99, 16,
            56, 200, 98, 181, 205, 24, 79, 120, 235, 223, 144, 61, 197, 8, 215, 17, 1, 1, 99, 220,
            1, 28, 113, 173, 87, 207, 150, 171, 166, 221, 201, 207, 122, 14, 62, 119, 8, 5, 100,
            182, 50, 112, 191, 244, 5, 125, 9, 161, 17, 66, 201, 126, 148, 2, 170, 62, 172, 42,
            117, 12, 62, 165, 78, 141, 84, 194, 75, 135, 140, 198, 85, 219, 214, 218, 149, 56, 32,
            251, 16, 134, 21, 44, 31, 22, 80, 69, 16, 1, 191, 139, 156, 120, 188, 0, 187, 111, 169,
            41, 135, 146, 156, 103, 102, 125, 235, 0, 70, 180, 94, 103, 134, 250, 135, 56, 55, 144,
            156, 185, 212, 67, 2, 223, 93, 226, 244, 94, 203, 160, 13, 145, 161, 22, 104, 133, 135,
            132, 239, 27, 61, 12, 167, 134, 237, 120, 58, 229, 208, 50, 55, 70, 139, 211, 234, 16,
            2, 58, 253, 249, 36, 12, 24, 122, 198, 7, 161, 13, 237, 229, 3, 224, 98, 176, 51, 237,
            101, 105, 33, 10, 45, 111, 96, 89, 201, 212, 82, 1, 141, 5, 16, 0, 0, 0, 0, 0, 0, 1,
            32, 0, 0, 0, 0, 0, 0, 1, 36, 77, 228, 149, 252, 55, 96, 22, 145, 235, 199, 188, 83, 13,
            88, 13, 241, 48, 191, 78, 152, 20, 50, 252, 186, 199, 105, 134, 73, 62, 136, 228, 196,
            17, 17, 17, 0, 1,
        ];

        let start_block_height = 329u64;
        let platform_version = PlatformVersion::latest();

        let result = Drive::verify_compacted_address_balance_changes(
            &proof,
            start_block_height,
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "a single adaptive proof must fail closed without an authenticated predecessor"
        );
    }
}
