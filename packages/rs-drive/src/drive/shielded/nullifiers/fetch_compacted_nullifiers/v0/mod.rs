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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::ProtocolError;
    use grovedb::Element;

    /// Limit=Some(0) must short-circuit before any GroveDB work and return an empty vec.
    #[test]
    fn limit_zero_short_circuits() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_compacted_nullifier_changes_v0(0, Some(0), None, platform_version)
            .expect("limit 0 should return Ok([])");
        assert_eq!(result.len(), 0);
    }

    /// Empty compacted tree should return empty vec.
    #[test]
    fn fetch_empty_compacted_tree_returns_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_compacted_nullifier_changes_v0(0, None, None, platform_version)
            .expect("empty tree fetch");
        assert!(result.is_empty());
    }

    /// Querying from a block_height past the only compacted range should miss that
    /// range entirely (because end_block < start_block_height).
    #[test]
    fn query_past_the_only_range_returns_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Force-insert one compacted entry spanning blocks 10..=20.
        let compacted_path = shielded_compacted_nullifiers_path();
        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&10u64.to_be_bytes());
        key.extend_from_slice(&20u64.to_be_bytes());
        let serialized = CompactedNullifiers::new(vec![[1u8; 32]]).encode().unwrap();
        drive
            .grove
            .insert(
                compacted_path.as_ref(),
                &key,
                Element::new_item(serialized),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("insert compacted");

        // Query from block 1000 — past the 10..=20 range.
        let result = drive
            .fetch_compacted_nullifier_changes_v0(1000, None, None, platform_version)
            .expect("fetch");
        assert_eq!(result.len(), 0, "range [10,20] does not cover 1000");
    }

    /// Querying from a block_height inside an existing range must return that range
    /// (exercising the descending-query branch that covers `start_block_height`).
    #[test]
    fn query_inside_range_returns_it() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Force-insert a compacted entry spanning blocks 400..=600.
        let compacted_path = shielded_compacted_nullifiers_path();
        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&400u64.to_be_bytes());
        key.extend_from_slice(&600u64.to_be_bytes());
        let nullifiers = vec![[0xAAu8; 32], [0xBBu8; 32]];
        let serialized = CompactedNullifiers::new(nullifiers.clone())
            .encode()
            .unwrap();
        drive
            .grove
            .insert(
                compacted_path.as_ref(),
                &key,
                Element::new_item(serialized),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("insert");

        let result = drive
            .fetch_compacted_nullifier_changes_v0(505, None, None, platform_version)
            .expect("fetch from block inside range");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].start_block, 400);
        assert_eq!(result[0].end_block, 600);
        assert_eq!(result[0].nullifiers.as_slice(), nullifiers.as_slice());
    }

    /// Inserting a garbage item payload under the compacted path triggers the
    /// CorruptedSerialization branch in CompactedNullifiers::decode.
    #[test]
    fn fetch_rejects_undecodable_payload_in_compacted_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let compacted_path = shielded_compacted_nullifiers_path();
        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&0u64.to_be_bytes());
        key.extend_from_slice(&10u64.to_be_bytes());
        // Garbage bytes that bincode cannot decode into Vec<[u8;32]>.
        drive
            .grove
            .insert(
                compacted_path.as_ref(),
                &key,
                Element::new_item(vec![0xFFu8; 3]),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("force-insert garbage item");

        match drive.fetch_compacted_nullifier_changes_v0(0, None, None, platform_version) {
            Err(Error::Protocol(b)) => match b.as_ref() {
                ProtocolError::CorruptedSerialization(_) => {}
                other => panic!("expected CorruptedSerialization, got: {:?}", other),
            },
            Err(other) => panic!("expected Error::Protocol, got: {:?}", other),
            Ok(_) => panic!("should reject undecodable payload"),
        }
    }
}
