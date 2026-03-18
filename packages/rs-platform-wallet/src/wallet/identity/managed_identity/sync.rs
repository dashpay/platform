//! Synchronization and block time management for ManagedIdentity

use super::ManagedIdentity;
use crate::BlockTime;
use dpp::prelude::TimestampMillis;

impl ManagedIdentity {
    /// Update the last balance update block time
    pub fn update_balance_block_time(&mut self, block_time: BlockTime) {
        self.last_updated_balance_block_time = Some(block_time);
    }

    /// Update the last keys sync block time
    pub fn update_keys_sync_block_time(&mut self, block_time: BlockTime) {
        self.last_synced_keys_block_time = Some(block_time);
    }

    /// Check if balance needs updating based on time elapsed
    pub fn needs_balance_update(
        &self,
        current_timestamp: TimestampMillis,
        max_age_millis: TimestampMillis,
    ) -> bool {
        match self.last_updated_balance_block_time {
            Some(block_time) => block_time.is_older_than(current_timestamp, max_age_millis),
            None => true, // Never updated
        }
    }

    /// Check if keys need syncing based on time elapsed
    pub fn needs_keys_sync(
        &self,
        current_timestamp: TimestampMillis,
        max_age_millis: TimestampMillis,
    ) -> bool {
        match self.last_synced_keys_block_time {
            Some(block_time) => block_time.is_older_than(current_timestamp, max_age_millis),
            None => true, // Never synced
        }
    }
}
