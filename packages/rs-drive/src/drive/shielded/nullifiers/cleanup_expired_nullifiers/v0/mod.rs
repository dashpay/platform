use crate::drive::shielded::nullifiers::queries::{
    shielded_compacted_nullifiers_path_vec, shielded_nullifiers_expiration_time_path_vec,
};
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
    pub(in crate::drive) fn cleanup_expired_nullifiers_v0(
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

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

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
            let (ranges, _): (Vec<(u64, u64)>, usize) =
                bincode::decode_from_slice(&serialized_ranges, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode expiration block ranges: {}",
                        e
                    ))))
                })?;

            // Delete each compacted nullifier entry
            for (start_block, end_block) in &ranges {
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
