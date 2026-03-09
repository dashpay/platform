use crate::drive::shielded::nullifiers::queries::{
    shielded_compacted_nullifiers_path_vec, shielded_nullifiers_expiration_time_path_vec,
};
use crate::drive::shielded::nullifiers::types::NullifierExpirationRanges;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of cleaning up expired compacted nullifier entries.
    ///
    /// Queries for all expiration entries with time <= current_block_time_ms,
    /// then deletes the corresponding compacted entries and the expiration entries.
    ///
    /// The query is unbounded (no limit) because the data model naturally caps
    /// the number of expiration entries. Compaction triggers every 64 blocks or
    /// 2048 nullifiers. With 3s blocks (201,600 blocks/week) and 1-week expiry:
    /// - Normal load (~1 shielded TPS): ~3,150 entries/week
    /// - Extreme load (683 shielded TPS, compaction every block): ~201,600 entries/week
    /// Both are well within GroveDB query capacity.
    pub(in crate::drive) fn cleanup_expired_nullifier_compactions_v0(
        &self,
        current_block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        let expiration_path = shielded_nullifiers_expiration_time_path_vec();

        // Query all entries with expiration time <= current_block_time_ms
        let mut query = Query::new();
        // Range from 0 to current_block_time_ms (inclusive)
        query.insert_item(QueryItem::RangeToInclusive(
            ..=current_block_time_ms.to_be_bytes().to_vec(),
        ));

        let path_query =
            PathQuery::new(expiration_path.clone(), SizedQuery::new(query, None, None));

        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let key_elements = results.to_key_elements();

        if key_elements.is_empty() {
            return Ok(0);
        }

        let mut batch = GroveDbOpBatch::new();
        let mut total_cleaned = 0usize;

        let compacted_path = shielded_compacted_nullifiers_path_vec();

        for (expiration_key, element) in key_elements {
            // Get the vec of block ranges from the element
            let grovedb::Element::Item(serialized_ranges, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item element for expiration block ranges".to_string(),
                    ),
                )));
            };

            // Deserialize the vec of block ranges
            let ranges = NullifierExpirationRanges::decode(&serialized_ranges)?;

            // Delete each compacted nullifier entry
            for (start_block, end_block) in ranges.iter() {
                let mut compacted_key = Vec::with_capacity(16);
                compacted_key.extend_from_slice(&start_block.to_be_bytes());
                compacted_key.extend_from_slice(&end_block.to_be_bytes());

                batch.add_delete(compacted_path.clone(), compacted_key);
                total_cleaned += 1;
            }

            // Delete the expiration entry itself
            batch.add_delete(expiration_path.clone(), expiration_key);
        }

        if !batch.is_empty() {
            self.grove_apply_batch(batch, false, transaction, &platform_version.drive)?;
        }

        Ok(total_cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::nullifiers::queries::{
        shielded_compacted_nullifiers_path, shielded_nullifiers_expiration_time_path,
    };
    use crate::drive::shielded::nullifiers::types::CompactedNullifiers;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::Element;

    /// Calculation: can the unbounded query in cleanup be a problem?
    ///
    /// Compaction triggers every 64 blocks or 2048 nullifiers.
    /// Platform blocks can happen every ~3s. In one week (604,800s):
    ///   - 201,600 blocks
    ///   - At most 201,600 / 64 = 3,150 compaction events (block-triggered)
    ///   - Each compaction = 1 expiration entry (unique timestamp key)
    ///
    /// To get more compactions, nullifier-triggered compaction must dominate:
    ///   - 2048 nullifiers/block → compaction every block → 201,600 entries/week
    ///   - That requires 2048 shielded tx/block ÷ 3s = ~683 shielded TPS sustained
    ///
    /// Even at 683 TPS for a full week, only ~201,600 expiration entries accumulate.
    /// At normal load (<1 shielded TPS), it's ~3,150 entries.
    ///
    /// This test creates 5,000 expiration entries (exceeding the normal worst-case week
    /// at <1 TPS) and verifies cleanup handles them fine.
    #[test]
    fn test_cleanup_handles_5000_expired_entries() {
        let drive = setup_drive_with_initial_state_structure(None);
        let transaction = drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();

        let expiration_path = shielded_nullifiers_expiration_time_path();
        let compacted_path = shielded_compacted_nullifiers_path();

        let num_entries: u64 = 5000;

        // Insert 5000 expiration entries and their corresponding compacted entries.
        // Each expiration entry has a unique timestamp key and references one block range.
        for i in 0..num_entries {
            let expiration_time_ms = (i + 1) * 1000; // 1s, 2s, ... 5000s
            let expiration_key = expiration_time_ms.to_be_bytes().to_vec();

            let start_block = i * 64;
            let end_block = start_block + 63;

            // Serialize the block ranges for the expiration entry
            let ranges = NullifierExpirationRanges::new(vec![(start_block, end_block)]);
            let serialized_ranges = ranges.encode().expect("encode ranges");

            // Insert expiration entry
            drive
                .grove
                .insert(
                    expiration_path.as_ref(),
                    &expiration_key,
                    Element::new_item(serialized_ranges),
                    None,
                    Some(&transaction),
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("insert expiration entry");

            // Insert corresponding compacted nullifier entry
            let mut compacted_key = Vec::with_capacity(16);
            compacted_key.extend_from_slice(&start_block.to_be_bytes());
            compacted_key.extend_from_slice(&end_block.to_be_bytes());

            // Just a small dummy payload
            let dummy_nullifiers = CompactedNullifiers::new(vec![[0u8; 32]]);
            let serialized_nullifiers = dummy_nullifiers.encode().expect("encode nullifiers");

            drive
                .grove
                .insert(
                    compacted_path.as_ref(),
                    &compacted_key,
                    Element::new_item(serialized_nullifiers),
                    None,
                    Some(&transaction),
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("insert compacted entry");
        }

        // Now cleanup all entries (current_block_time >= all expiration times)
        let current_time_ms = (num_entries + 1) * 1000;

        let cleaned = drive
            .cleanup_expired_nullifier_compactions(
                current_time_ms,
                Some(&transaction),
                platform_version,
            )
            .expect("cleanup should succeed with 5000 entries");

        assert_eq!(cleaned, num_entries as usize);
    }
}
