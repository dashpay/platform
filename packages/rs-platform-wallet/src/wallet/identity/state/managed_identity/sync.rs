//! Synchronization and block time management for ManagedIdentity

use super::ManagedIdentity;
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::BlockTime;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::prelude::TimestampMillis;

impl ManagedIdentity {
    /// Replay a persisted snapshot selected by the manager's revision policy.
    /// Its balance and watermark replace the old pair together, including any
    /// uncommitted snapshot superseded by the accepted persisted entry.
    pub(crate) fn restore_persisted_balance(
        &mut self,
        balance: u64,
        block_time: Option<BlockTime>,
    ) {
        self.identity.set_balance(balance);
        self.last_updated_balance_block_time = block_time;
        self.pending_balance_snapshot = None;
    }

    /// The pending balance also rides unrelated scalar snapshots so a later
    /// profile/key-sync write cannot queue the old balance behind a failed flush.
    pub(crate) fn balance_snapshot_for_persistence(&self) -> (u64, Option<BlockTime>) {
        if let Some((balance, block_time)) = self.pending_balance_snapshot {
            if self
                .last_updated_balance_block_time
                .is_none_or(|current| block_time.height >= current.height)
            {
                return (balance, Some(block_time));
            }
        }
        (
            self.identity.balance(),
            self.last_updated_balance_block_time,
        )
    }

    fn balance_snapshot_is_newer(&self, block_time: BlockTime) -> bool {
        // Both Fetch<IdentityBalance> and the transaction affected-state waits
        // verify a GroveDB root AND its quorum-signed StateId. Their heights
        // identify committed snapshots of the same chain, not independent node
        // clocks. Equal heights cannot order two transaction completions.
        let (_, previous) = self.balance_snapshot_for_persistence();
        if let Some(previous) = previous {
            if block_time.height < previous.height {
                tracing::warn!(
                    identity = %self.id(),
                    response_height = block_time.height,
                    retained_height = previous.height,
                    "Ignoring an older verified balance snapshot"
                );
                return false;
            }
            if block_time.height == previous.height {
                tracing::debug!(identity = %self.id(), height = block_time.height,
                    "Ignoring a duplicate-height verified balance snapshot");
                return false;
            }
        }
        true
    }

    /// Retry an idempotent scalar snapshot before the watermark can suppress it.
    /// The caller holds the wallet-manager write lock through commit/publication.
    pub(crate) fn retry_pending_balance(
        &mut self,
        persister: &WalletPersister,
    ) -> Result<(), PlatformWalletError> {
        if self.pending_balance_snapshot.is_none() {
            return Ok(());
        }
        let (balance, block_time) = self.balance_snapshot_for_persistence();
        persister
            .store(self.snapshot_changeset().into())
            .map_err(|e| persister.classify_store_failure(e))?;
        if !persister.store_commits_inline() {
            persister
                .flush()
                .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        }
        self.identity.set_balance(balance);
        self.last_updated_balance_block_time = block_time;
        self.pending_balance_snapshot = None;
        Ok(())
    }

    pub(crate) fn persist_refreshed_balance(
        &mut self,
        balance: u64,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) -> Result<(), PlatformWalletError> {
        self.retry_pending_balance(persister)?;
        if self.balance_snapshot_is_newer(block_time) {
            self.pending_balance_snapshot = Some((balance, block_time));
            self.retry_pending_balance(persister)?;
        }
        Ok(())
    }

    /// Keep a verified post-broadcast snapshot even if its cache write fails.
    /// A cache failure must not invite a second payment. Retain the write for a
    /// later refresh/transaction, and return the balance actually kept locally.
    pub(crate) fn persist_confirmed_balance(
        &mut self,
        balance: u64,
        block_time: BlockTime,
        persister: &WalletPersister,
    ) -> u64 {
        if self.balance_snapshot_is_newer(block_time) {
            self.identity.set_balance(balance);
            self.last_updated_balance_block_time = Some(block_time);
            self.pending_balance_snapshot = Some((balance, block_time));
        }
        // A transaction response can confirm the exact pending query snapshot
        // whose cache write failed. Publish that verified value now, while
        // retaining the same write obligation if storage is still unavailable.
        if self.pending_balance_snapshot == Some((balance, block_time)) {
            self.identity.set_balance(balance);
            self.last_updated_balance_block_time = Some(block_time);
        }
        if let Err(error) = self.retry_pending_balance(persister) {
            tracing::error!(identity = %self.id(), %error,
                "Failed to persist confirmed identity balance; snapshot retained for retry");
        }
        self.identity.balance()
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
