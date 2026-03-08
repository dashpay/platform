use crate::drive::shielded::nullifiers::queries::{
    shielded_recent_nullifiers_path, SHIELDED_RECENT_NULLIFIERS_KEY_U8,
};
use crate::drive::shielded::paths::shielded_credit_pool_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::ProtocolError;
use grovedb::Element;
use grovedb::TransactionArg;

use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of storing nullifiers for a block.
    ///
    /// Serializes the nullifier list using bincode and stores it in the
    /// shielded credit pool per-block nullifiers count sum tree keyed by block height.
    /// Each entry is an ItemWithSumItem where:
    /// - The item contains the serialized nullifiers
    /// - The sum value is the number of nullifiers
    ///
    /// Before storing, checks if compaction thresholds are exceeded and triggers
    /// compaction if necessary. If compaction occurs, the current block's nullifiers
    /// are included in the compaction rather than stored separately.
    pub(in crate::drive) fn store_nullifiers_for_block_v0(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Early return if there are no nullifiers to store
        if nullifiers.is_empty() {
            return Ok(());
        }

        // Check if compaction is needed - if so, include current nullifiers in compaction
        let compacted = self.check_and_compact_nullifiers_if_needed(
            nullifiers,
            block_height,
            block_time_ms,
            transaction,
            platform_version,
        )?;

        // If we compacted, the current nullifiers are already included - don't store separately
        if compacted {
            return Ok(());
        }

        // Serialize the nullifiers using bincode
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let serialized = bincode::encode_to_vec(nullifiers, config).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode nullifiers: {}",
                e
            ))))
        })?;

        // The sum value is the number of nullifiers
        let entry_count = i64::try_from(nullifiers.len()).map_err(|_| {
            Error::Drive(crate::error::drive::DriveError::CorruptedDriveState(
                "nullifier count exceeds i64::MAX".to_string(),
            ))
        })?;

        // Store in the shielded pool per-block nullifiers count sum tree with block height as key
        let path = shielded_recent_nullifiers_path();

        // Use block height as the key (big-endian for proper ordering)
        let key = block_height.to_be_bytes();

        // Insert as ItemWithSumItem where:
        // - item data = serialized nullifiers
        // - sum value = number of nullifiers
        let mut drive_operations = vec![];
        self.grove_insert(
            path.as_ref().into(),
            &key,
            Element::new_item_with_sum_item(serialized, entry_count),
            transaction,
            None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Apply any operations that were generated
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }

    /// Checks if compaction thresholds are exceeded and triggers compaction if needed.
    /// If compaction occurs, the provided nullifiers are included in the compaction.
    ///
    /// Returns true if compaction was performed (meaning the nullifiers were included).
    fn check_and_compact_nullifiers_if_needed(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let pool_path = shielded_credit_pool_path();

        // Get the count sum tree element to check current count and sum
        let mut drive_operations = vec![];
        let tree_element = self.grove_get_raw(
            (&pool_path).into(),
            &[SHIELDED_RECENT_NULLIFIERS_KEY_U8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if let Some(Element::CountSumTree(_, count, sum, _)) = tree_element {
            let max_blocks = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_blocks_before_nullifier_compaction as u64;
            let max_nullifiers = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_nullifiers_before_compaction as i64;

            // Check if either threshold would be exceeded after adding the current block
            // count + 1 for the new block, sum + current nullifiers count
            let new_count = count + 1;
            let new_sum = sum + nullifiers.len() as i64;

            if new_count >= max_blocks || new_sum >= max_nullifiers {
                // Trigger compaction, including the current block's nullifiers
                self.compact_nullifiers_with_current_block(
                    nullifiers,
                    block_height,
                    block_time_ms,
                    transaction,
                    platform_version,
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}
