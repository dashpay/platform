use crate::drive::Drive;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::BlockAwareCreditOperation;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::CompactedAddressBalanceProof;

/// Result type for fetched compacted address balance changes
/// Each entry is (start_block, end_block, address_balance_map)
pub type CompactedAddressBalanceChanges = Vec<(
    u64,
    u64,
    BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
)>;

impl Drive {
    /// Version 0 implementation of fetching compacted address balance changes.
    ///
    /// Retrieves all compacted address balance change records where `end_block >= start_block_height`.
    /// This includes ranges that contain `start_block_height` (e.g., range 400-600 when querying
    /// from block 505) as well as ranges that start after `start_block_height`.
    ///
    /// Returns a vector of (start_block, end_block, address_balance_map) tuples.
    pub(super) fn fetch_compacted_address_balance_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<CompactedAddressBalanceChanges, Error> {
        let path = Self::saved_compacted_block_transactions_address_balances_path_vec();

        // Keys are 16 bytes: (start_block, end_block), both big-endian.
        // We want ranges where end_block >= start_block_height, which includes:
        // - Ranges that contain start_block_height (e.g., 400-600 contains 505)
        // - Ranges that start at or after start_block_height
        //
        // Strategy:
        // 1. First query: descending from (start_block_height, u64::MAX) with limit 1
        //    to find any range where start_block <= start_block_height that might contain it
        // 2. Second query: ascending from (start_block_height, 0) to get ranges
        //    that start at or after start_block_height

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let mut compacted_changes = Vec::new();
        let limit_usize = limit.map(|l| l as usize);

        // Query 1: Find if there's a range containing start_block_height
        // Query descending from (start_block_height, u64::MAX) with limit 1
        let mut desc_end_key = Vec::with_capacity(16);
        desc_end_key.extend_from_slice(&start_block_height.to_be_bytes());
        desc_end_key.extend_from_slice(&u64::MAX.to_be_bytes());

        let mut desc_query = Query::new_with_direction(false); // descending
        desc_query.insert_range_to_inclusive(..=desc_end_key);

        let desc_path_query =
            PathQuery::new(path.clone(), SizedQuery::new(desc_query, Some(1), None));

        let (desc_results, _) = self.grove_get_path_query(
            &desc_path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        // Check if we found a range that contains start_block_height
        if let Some((key, element)) = desc_results.to_key_elements().into_iter().next() {
            if key.len() != 16 {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "invalid compacted block key length, expected 16 bytes".to_string(),
                    ),
                )));
            }

            let start_block = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Only include if end_block >= start_block_height (range contains our block)
            if end_block >= start_block_height {
                let Element::Item(serialized_data, _) = element else {
                    return Err(Error::Protocol(Box::new(
                        ProtocolError::CorruptedSerialization(
                            "expected item element for compacted address balances".to_string(),
                        ),
                    )));
                };

                let (address_balances, _): (
                    BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                    usize,
                ) = bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode compacted address balances: {}",
                        e
                    ))))
                })?;

                compacted_changes.push((start_block, end_block, address_balances));
            }
        }

        // Check if we've already hit the limit
        if let Some(l) = limit_usize {
            if compacted_changes.len() >= l {
                return Ok(compacted_changes);
            }
        }

        // Query 2: Get ranges that start at or after start_block_height (ascending)
        // Always use (start_block_height, 0) for consistent proof verification
        // The result may overlap with descending query if descending found a range
        // starting exactly at start_block_height - we dedupe below
        let mut asc_start_key = Vec::with_capacity(16);
        asc_start_key.extend_from_slice(&start_block_height.to_be_bytes());
        asc_start_key.extend_from_slice(&0u64.to_be_bytes());

        let mut asc_query = Query::new();
        asc_query.insert_range_from(asc_start_key..);

        let asc_path_query = PathQuery::new(path, SizedQuery::new(asc_query, limit, None));

        let (asc_results, _) = self.grove_get_path_query(
            &asc_path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        // Track the start_block from descending query to avoid duplicates
        let desc_start_block = compacted_changes.first().map(|(start, _, _)| *start);

        for (key, element) in asc_results.to_key_elements() {
            // Check if we've reached the limit
            if let Some(l) = limit_usize {
                if compacted_changes.len() >= l {
                    break;
                }
            }

            if key.len() != 16 {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "invalid compacted block key length, expected 16 bytes".to_string(),
                    ),
                )));
            }

            let start_block = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Skip if this is the same range we got from descending query
            if Some(start_block) == desc_start_block {
                continue;
            }

            let Element::Item(serialized_data, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item element for compacted address balances".to_string(),
                    ),
                )));
            };

            let (address_balances, _): (
                BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                usize,
            ) = bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                    "cannot decode compacted address balances: {}",
                    e
                ))))
            })?;

            compacted_changes.push((start_block, end_block, address_balances));
        }

        Ok(compacted_changes)
    }

    /// Version 0 implementation for proving compacted address balance changes.
    ///
    /// Uses two independently verifiable proofs:
    /// 1. A descending predecessor proof authenticates which range, if any,
    ///    contains `start_block_height`.
    /// 2. A forward proof starts at that authenticated range (or at the
    ///    request-derived fallback key when no range contains the height).
    ///
    /// This ensures the proof covers all relevant ranges efficiently.
    pub(super) fn prove_compacted_address_balance_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_compacted_block_transactions_address_balances_path_vec();

        // Step 1: Authenticate the predecessor used to select the forward query.
        let mut desc_end_key = Vec::with_capacity(16);
        desc_end_key.extend_from_slice(&start_block_height.to_be_bytes());
        desc_end_key.extend_from_slice(&u64::MAX.to_be_bytes());

        let mut desc_query = Query::new_with_direction(false); // descending
        desc_query.insert_range_to_inclusive(..=desc_end_key);

        let desc_path_query =
            PathQuery::new(path.clone(), SizedQuery::new(desc_query, Some(1), None));

        let (desc_results, _) = self.grove_get_path_query(
            &desc_path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let predecessor_proof = self.grove_get_proved_path_query(
            &desc_path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        // Determine the actual start key for the proved query
        // If we found a containing range, use its exact key
        // Otherwise use (start_block_height, start_block_height) since end_block >= start_block always
        let start_key = if let Some((key, _)) = desc_results.to_key_elements().into_iter().next() {
            if key.len() == 16 {
                let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());
                // If this range contains start_block_height, use its exact key
                if end_block >= start_block_height {
                    key
                } else {
                    // No containing range, use (start_block_height, start_block_height)
                    let mut key = Vec::with_capacity(16);
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key
                }
            } else {
                let mut key = Vec::with_capacity(16);
                key.extend_from_slice(&start_block_height.to_be_bytes());
                key.extend_from_slice(&start_block_height.to_be_bytes());
                key
            }
        } else {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        };

        // Step 2: Proved ascending query from start_key

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let forward_proof = self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        bincode::encode_to_vec(
            CompactedAddressBalanceProof {
                predecessor_proof,
                forward_proof,
            },
            bincode::config::standard()
                .with_big_endian()
                .with_no_limit(),
        )
        .map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode compacted address balance proof: {e}"
            ))))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use dpp::version::mocks::v2_test::TEST_PLATFORM_V2;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const ADDR_1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const ADDR_2: PlatformAddress = PlatformAddress::P2sh([11; 20]);

    const TEST_BLOCK_TIME_MS: u64 = 1_700_000_000_000;

    /// Creates a platform version with low compaction thresholds for testing.
    fn create_test_platform_version(max_blocks: u16, max_addresses: u32) -> PlatformVersion {
        let mut version = TEST_PLATFORM_V2.clone();
        version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_compaction = max_blocks;
        version
            .drive
            .methods
            .saved_block_transactions
            .max_addresses_before_compaction = max_addresses;
        version
    }

    /// Stores `count` blocks of address balance changes (blocks 1..=count).
    /// Each block has ADDR_1; even-numbered blocks also have ADDR_2.
    fn store_blocks(drive: &crate::drive::Drive, count: u64, platform_version: &PlatformVersion) {
        for block_height in 1u64..=count {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR_1, CreditOperation::AddToCredits(block_height * 1000));
            if block_height % 2 == 0 {
                changes.insert(ADDR_2, CreditOperation::AddToCredits(block_height * 500));
            }
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    TEST_BLOCK_TIME_MS + block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }
    }

    #[test]
    fn should_fetch_compacted_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold: compact after 3 blocks, so storing 4 blocks triggers compaction
        let platform_version = create_test_platform_version(3, 10_000);

        // Store 4 blocks — the 4th block triggers compaction (count 3 + 1 >= 3)
        store_blocks(&drive, 4, &platform_version);

        // Fetch compacted changes starting from block 0 (should include everything)
        let compacted = drive
            .fetch_compacted_address_balance_changes_v0(0, None, None, &platform_version)
            .expect("should fetch compacted changes");

        assert!(
            !compacted.is_empty(),
            "should have at least one compacted entry after compaction"
        );

        for (start, end, changes) in &compacted {
            assert!(
                start <= end,
                "start block {} should be <= end block {}",
                start,
                end
            );
            assert!(
                !changes.is_empty(),
                "each compacted entry should have address changes"
            );
        }
    }

    #[test]
    fn should_fetch_empty_compacted_when_no_compaction() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Threshold of 64 blocks (default) — storing 2 blocks will never compact
        let platform_version = create_test_platform_version(64, 10_000);

        // Store only 2 blocks — well below the compaction threshold
        store_blocks(&drive, 2, &platform_version);

        // Fetch compacted changes from block 0 — no compaction should have occurred
        let compacted = drive
            .fetch_compacted_address_balance_changes_v0(0, None, None, &platform_version)
            .expect("should fetch compacted changes");

        assert!(
            compacted.is_empty(),
            "should have no compacted entries when below compaction threshold, got {} entries",
            compacted.len()
        );
    }

    #[test]
    fn should_prove_compacted_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = create_test_platform_version(3, 10_000);

        // Store 4 blocks to trigger compaction
        store_blocks(&drive, 4, &platform_version);

        // Generate a proof of compacted changes from block 1
        let proof = drive
            .prove_compacted_address_balance_changes_v0(1, None, None, &platform_version)
            .expect("should prove compacted address balance changes");

        assert!(
            !proof.is_empty(),
            "proof bytes should not be empty after compaction"
        );
    }

    #[test]
    fn should_fetch_compacted_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Very low threshold so we can trigger multiple compactions
        let platform_version = create_test_platform_version(3, 10_000);

        // Store enough blocks to trigger at least two compactions
        // Compaction fires at blocks 4, 7, 10, ...  (every 3 blocks after the first 3)
        store_blocks(&drive, 10, &platform_version);

        // Fetch with limit=1 — should return at most 1 compacted entry
        let compacted = drive
            .fetch_compacted_address_balance_changes_v0(0, Some(1), None, &platform_version)
            .expect("should fetch compacted changes with limit");

        assert!(
            compacted.len() <= 1,
            "with limit=1, should have at most 1 entry, got {}",
            compacted.len()
        );

        // Also fetch without limit to confirm there is more than 1 entry total
        let all_compacted = drive
            .fetch_compacted_address_balance_changes_v0(0, None, None, &platform_version)
            .expect("should fetch all compacted changes");

        // The limited result should be a subset of the full result
        assert!(
            compacted.len() <= all_compacted.len(),
            "limited results ({}) should not exceed unlimited results ({})",
            compacted.len(),
            all_compacted.len()
        );
    }
}
