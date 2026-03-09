use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path_vec;
use crate::drive::shielded::nullifiers::types::{CompactedNullifierChange, CompactedNullifiers};
use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of fetching compacted nullifier changes.
    ///
    /// Retrieves all compacted nullifier change records where `end_block >= start_block_height`.
    /// This includes ranges that contain `start_block_height` (e.g., range 400-600 when querying
    /// from block 505) as well as ranges that start after `start_block_height`.
    ///
    /// Returns a vector of (start_block, end_block, nullifiers) tuples.
    pub(in crate::drive) fn fetch_compacted_nullifier_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<CompactedNullifierChange>, Error> {
        let path = shielded_compacted_nullifiers_path_vec();

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

        let mut compacted_changes = Vec::new();
        let limit_usize = limit.map(|l| l as usize);

        // Short-circuit: if limit is zero, no results should be returned
        if limit_usize == Some(0) {
            return Ok(compacted_changes);
        }

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
            let (start_block, end_block) = CompactedNullifierChange::parse_key(&key)?;

            // Only include if end_block >= start_block_height (range contains our block)
            if end_block >= start_block_height {
                let Element::Item(serialized_data, _) = element else {
                    return Err(Error::Protocol(Box::new(
                        ProtocolError::CorruptedSerialization(
                            "expected item element for compacted nullifiers".to_string(),
                        ),
                    )));
                };

                let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

                compacted_changes.push(CompactedNullifierChange {
                    start_block,
                    end_block,
                    nullifiers,
                });
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

        // Track the (start_block, end_block) from descending query to avoid duplicates
        let desc_range_key = compacted_changes
            .first()
            .map(|c| (c.start_block, c.end_block));

        for (key, element) in asc_results.to_key_elements() {
            // Check if we've reached the limit
            if let Some(l) = limit_usize {
                if compacted_changes.len() >= l {
                    break;
                }
            }

            let (start_block, end_block) = CompactedNullifierChange::parse_key(&key)?;

            // Skip if this is the same range we got from descending query
            if Some((start_block, end_block)) == desc_range_key {
                continue;
            }

            let Element::Item(serialized_data, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item element for compacted nullifiers".to_string(),
                    ),
                )));
            };

            let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            compacted_changes.push(CompactedNullifierChange {
                start_block,
                end_block,
                nullifiers,
            });
        }

        Ok(compacted_changes)
    }
}
