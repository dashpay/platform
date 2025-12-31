use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Version 0 implementation of compacting address balance changes including a current block.
    ///
    /// Drains all entries from the address balances count sum tree,
    /// merges them with the provided current block's address balances,
    /// and stores the result in the compacted address balances tree
    /// with a (start_block, end_block) key.
    ///
    /// Returns the range of blocks that were compacted (start_block, end_block).
    pub(super) fn compact_address_balances_with_current_block_v0(
        &self,
        current_address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        current_block_height: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, u64), Error> {
        let path = Self::saved_block_transactions_address_balances_path_vec();

        // Query all entries from the address balances tree
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

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // Track block range - start with current block
        let mut start_block: u64 = current_block_height;
        let mut end_block: u64 = current_block_height;

        // Start with empty map - we'll merge in chronological order
        let mut merged_balances: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        let mut keys_to_delete: Vec<Vec<u8>> = Vec::new();

        // First, process stored blocks in chronological order (ascending by block height)
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
                        "expected item with sum item element for address balances".to_string(),
                    ),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (BTreeMap<PlatformAddress, CreditOperation>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode address balances: {}",
                        e
                    ))))
                })?;

            // Merge into the combined map - stored blocks come first chronologically
            // existing.merge(operation) means: apply existing first, then operation (later)
            for (address, operation) in address_balances {
                merged_balances
                    .entry(address)
                    .and_modify(|existing| {
                        // existing is from earlier blocks, operation is from this (later) block
                        *existing = existing.merge(&operation);
                    })
                    .or_insert(operation);
            }

            keys_to_delete.push(key);
        }

        // Finally, merge the current block's balances (the latest in chronological order)
        for (address, operation) in current_address_balances {
            merged_balances
                .entry(*address)
                .and_modify(|existing| {
                    // existing is from stored blocks (earlier), operation is from current (latest)
                    *existing = existing.merge(operation);
                })
                .or_insert(*operation);
        }

        // Serialize the merged balances
        let serialized = bincode::encode_to_vec(&merged_balances, config).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode compacted address balances: {}",
                e
            ))))
        })?;

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

        // Insert the compacted entry
        batch.add_insert(
            Self::saved_compacted_block_transactions_address_balances_path_vec(),
            compacted_key,
            Element::new_item(serialized),
        );

        // Apply the batch
        self.grove_apply_batch(batch, false, transaction, &platform_version.drive)?;

        Ok((start_block, end_block))
    }
}
