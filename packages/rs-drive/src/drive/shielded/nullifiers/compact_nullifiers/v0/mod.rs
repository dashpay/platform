use super::ONE_WEEK_IN_MS;
use crate::drive::shielded::nullifiers::queries::{
    shielded_compacted_nullifiers_path_vec, shielded_nullifiers_expiration_time_path,
    shielded_nullifiers_expiration_time_path_vec, shielded_recent_nullifiers_path_vec,
};
use crate::drive::shielded::nullifiers::types::{CompactedNullifiers, NullifierExpirationRanges};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::grove_operations::DirectQueryType;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use grovedb_path::SubtreePath;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of compacting nullifiers including a current block.
    ///
    /// Drains all entries from the nullifiers count sum tree,
    /// concatenates them with the provided current block's nullifiers,
    /// and stores the result in the compacted nullifiers tree
    /// with a (start_block, end_block) key.
    ///
    /// Also stores the expiration time (current block time + 1 week) in the
    /// nullifiers expiration time tree with the same (start_block, end_block) key.
    ///
    /// Returns the range of blocks that were compacted (start_block, end_block).
    pub(in crate::drive) fn compact_nullifiers_with_current_block_v0(
        &self,
        current_nullifiers: &[[u8; 32]],
        current_block_height: u64,
        current_block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, u64), Error> {
        let path = shielded_recent_nullifiers_path_vec();

        // Query all entries from the nullifiers tree
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));

        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let key_elements = results.to_key_elements();

        // Track block range - start with current block
        let mut start_block: u64 = current_block_height;
        let mut end_block: u64 = current_block_height;

        // Concatenate all nullifiers together (no merge semantics needed)
        let mut combined_nullifiers: Vec<[u8; 32]> = Vec::new();
        let mut keys_to_delete: Vec<Vec<u8>> = Vec::new();

        // Process stored blocks in chronological order (ascending by block height)
        // Keys are big-endian u64, so they're already in ascending order
        for (key, element) in key_elements {
            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.clone().try_into().map_err(|_| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                    "invalid block height key length".to_string(),
                )))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Track start and end blocks
            if block_height < start_block {
                start_block = block_height;
            }
            if block_height > end_block {
                end_block = block_height;
            }

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item with sum item element for nullifiers".to_string(),
                    ),
                )));
            };

            // Deserialize the nullifier list
            let block_nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            // Simply concatenate - no merge semantics needed for nullifiers
            combined_nullifiers.extend(block_nullifiers.iter());

            keys_to_delete.push(key);
        }

        // Append the current block's nullifiers
        combined_nullifiers.extend_from_slice(current_nullifiers);

        // Serialize the combined nullifiers
        let serialized = CompactedNullifiers::new(combined_nullifiers).encode()?;

        // Create the compacted key: (start_block, end_block) as 16 bytes
        let mut compacted_key = Vec::with_capacity(16);
        compacted_key.extend_from_slice(&start_block.to_be_bytes());
        compacted_key.extend_from_slice(&end_block.to_be_bytes());

        // Build batch operations
        let mut batch = GroveDbOpBatch::new();

        // Delete all original entries from the count sum tree
        for key in keys_to_delete {
            batch.add_delete(path.clone(), key);
        }

        // Insert the compacted entry as a plain Item (not ItemWithSumItem)
        batch.add_insert(
            shielded_compacted_nullifiers_path_vec(),
            compacted_key.clone(),
            Element::new_item(serialized),
        );

        // Calculate expiration time (current block time + 1 week)
        let expiration_time_ms = current_block_time_ms.saturating_add(ONE_WEEK_IN_MS);
        let expiration_key = expiration_time_ms.to_be_bytes().to_vec();

        // Check if an entry with this expiration time already exists
        // If so, we need to append to the existing vec of block ranges
        let expiration_path = shielded_nullifiers_expiration_time_path();

        let mut drive_operations = vec![];
        let existing_ranges = self.grove_get_raw_optional(
            SubtreePath::from(expiration_path.as_ref()),
            &expiration_key,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let expiration_value = if let Some(Element::Item(existing_data, _)) = existing_ranges {
            // Deserialize existing vec of block ranges and append the new one
            let mut ranges = NullifierExpirationRanges::decode(&existing_data)?.into_inner();
            ranges.push((start_block, end_block));
            NullifierExpirationRanges::new(ranges).encode()?
        } else {
            // No existing entry, create new vec with single range
            NullifierExpirationRanges::new(vec![(start_block, end_block)]).encode()?
        };

        // Store in the expiration tree: key = expiration_time, value = vec of (start_block, end_block)
        batch.add_insert(
            shielded_nullifiers_expiration_time_path_vec(),
            expiration_key,
            Element::new_item(expiration_value),
        );

        // Apply the batch
        self.grove_apply_batch(batch, false, transaction, &platform_version.drive)?;

        Ok((start_block, end_block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    /// Compacting with no existing recent entries (empty pool) produces a single
    /// compacted entry whose range is (current_block, current_block).
    #[test]
    fn compact_with_empty_pool_uses_current_block_as_range() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let nullifiers = vec![[7u8; 32], [8u8; 32]];

        let (start, end) = drive
            .compact_nullifiers_with_current_block_v0(
                &nullifiers,
                42,
                5_000,
                None,
                platform_version,
            )
            .expect("compact should succeed");

        assert_eq!(start, 42);
        assert_eq!(end, 42);

        // Read back via fetch_compacted.
        let compacted = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch compacted");
        assert_eq!(compacted.len(), 1);
        assert_eq!(compacted[0].start_block, 42);
        assert_eq!(compacted[0].end_block, 42);
        assert_eq!(compacted[0].nullifiers.as_slice(), nullifiers.as_slice());
    }

    /// Compacting twice with the same current_block_time_ms must merge the two
    /// ranges under the same expiration key (exercising the
    /// "existing_ranges is Some" branch). We additionally drive a cleanup past
    /// the shared expiration time: if the second compaction had *overwritten*
    /// the expiration index entry instead of appending, cleanup would only
    /// chase one range and leave the other compacted row dangling.
    #[test]
    fn second_compaction_with_same_time_appends_range_to_expiration() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let block_time = 1_000u64;

        drive
            .compact_nullifiers_with_current_block_v0(
                &[[1u8; 32]],
                10,
                block_time,
                None,
                platform_version,
            )
            .expect("first compact");

        drive
            .compact_nullifiers_with_current_block_v0(
                &[[2u8; 32]],
                20,
                block_time, // same block time → same expiration key
                None,
                platform_version,
            )
            .expect("second compact");

        let compacted = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch");
        assert_eq!(compacted.len(), 2, "both compacted ranges must be present");

        // Cleanup past the shared expiration must remove BOTH ranges. This is
        // what proves the second compaction appended its range to the
        // existing expiration-index entry rather than overwriting it — a
        // single range under that key would leave one compacted row behind
        // and the count would be 1, not 2.
        let current =
            block_time + crate::drive::shielded::nullifiers::compact_nullifiers::ONE_WEEK_IN_MS + 1;
        let cleaned = drive
            .cleanup_expired_nullifier_compactions_v0(current, None, platform_version)
            .expect("cleanup");
        assert_eq!(
            cleaned, 2,
            "cleanup must delete both ranges stored under the shared expiration key"
        );

        let compacted_after = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch after cleanup");
        assert!(
            compacted_after.is_empty(),
            "no compacted rows should remain after cleanup"
        );
    }

    /// Compacting drains entries stored via store_nullifiers_for_block_v0 and the
    /// final combined list is the concatenation of stored + current in block order.
    #[test]
    fn compact_drains_recent_and_concats_in_order() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .store_nullifiers_for_block_v0(&[[1u8; 32]], 1, 100, None, platform_version)
            .expect("store 1");
        drive
            .store_nullifiers_for_block_v0(&[[2u8; 32]], 2, 200, None, platform_version)
            .expect("store 2");

        let (start, end) = drive
            .compact_nullifiers_with_current_block_v0(&[[3u8; 32]], 3, 300, None, platform_version)
            .expect("compact");

        assert_eq!(start, 1);
        assert_eq!(end, 3);

        // After compaction, recent pool should be drained.
        let recent = drive
            .fetch_recent_nullifier_changes(0, None, None, platform_version)
            .expect("fetch recent");
        assert_eq!(recent.len(), 0, "recent should be drained after compact");

        let compacted = drive
            .fetch_compacted_nullifier_changes(0, None, None, platform_version)
            .expect("fetch compacted");
        assert_eq!(compacted.len(), 1);
        // Concatenation in ascending block order: [1][2][3]
        assert_eq!(
            compacted[0].nullifiers.as_slice(),
            &[[1u8; 32], [2u8; 32], [3u8; 32]]
        );
    }
}
