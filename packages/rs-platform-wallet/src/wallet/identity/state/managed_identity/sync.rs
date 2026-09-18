//! Synchronization and block time management for ManagedIdentity

use super::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
use crate::BlockTime;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::prelude::TimestampMillis;

impl ManagedIdentity {
    /// Apply a transaction result together with its proof height so lagging
    /// balance queries (or older transaction completions) cannot replace it.
    pub(crate) fn set_confirmed_balance(&mut self, balance: u64, height: u64) {
        if self
            .last_updated_balance_block_time
            .is_none_or(|previous| height >= previous.height)
        {
            self.identity.set_balance(balance);
            // These transaction APIs expose only the proof height; zero marks
            // unavailable Core height and time, rather than retaining stale values.
            self.last_updated_balance_block_time = Some(BlockTime::new(height, 0, 0));
        }
    }

    /// Update the last balance update block time.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn update_balance_block_time(
        &mut self,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) {
        self.last_updated_balance_block_time = Some(block_time);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Update the last keys sync block time.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn update_keys_sync_block_time(
        &mut self,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) {
        self.last_synced_keys_block_time = Some(block_time);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
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
