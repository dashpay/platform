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
