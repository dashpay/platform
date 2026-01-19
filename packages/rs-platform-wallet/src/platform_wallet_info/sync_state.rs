//! Platform sync state tracking
//!
//! This module provides the `PlatformSyncState` struct for tracking the state
//! of platform address synchronization, enabling efficient incremental sync.

/// Time interval (in seconds) after which a full sync should be performed
/// 6 days and 20 hours = 590400 seconds
const FULL_SYNC_INTERVAL_SECS: u64 = 6 * 24 * 60 * 60 + 20 * 60 * 60;

/// Tracks the state of platform address synchronization
///
/// This struct enables efficient incremental synchronization by tracking:
/// - When the last full sync was performed
/// - The checkpoint height from that sync
/// - The last terminal block processed
///
/// The sync algorithm uses this state to decide whether to perform a full
/// privacy-preserving sync or just a terminal sync for recent changes.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlatformSyncState {
    /// Timestamp of last full sync (seconds since epoch)
    pub last_full_sync_timestamp: u64,

    /// Checkpoint height from last full sync
    ///
    /// This is the Platform block height at which the address tree was queried.
    /// Terminal sync should start from this height to catch any changes.
    pub checkpoint_height: u64,

    /// Last processed terminal block height
    ///
    /// This tracks how far terminal sync has progressed, allowing us to avoid
    /// re-processing the same blocks.
    pub last_terminal_block: u64,

    /// Highest found address index (for gap limit tracking)
    pub highest_found_index: Option<u32>,
}

impl PlatformSyncState {
    /// Create a new empty sync state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a full sync is needed based on time since last sync
    ///
    /// Returns true if:
    /// - No full sync has ever been performed
    /// - No checkpoint exists
    /// - More than 6d20h has passed since the last full sync
    pub fn needs_full_sync(&self, current_timestamp: u64) -> bool {
        self.last_full_sync_timestamp == 0
            || self.checkpoint_height == 0
            || current_timestamp.saturating_sub(self.last_full_sync_timestamp)
                >= FULL_SYNC_INTERVAL_SECS
    }

    /// Update the state after a full sync
    pub fn update_after_full_sync(&mut self, timestamp: u64, checkpoint_height: u64) {
        self.last_full_sync_timestamp = timestamp;
        self.checkpoint_height = checkpoint_height;
    }

    /// Update the last terminal block processed
    pub fn update_last_terminal_block(&mut self, block_height: u64) {
        if block_height > self.last_terminal_block {
            self.last_terminal_block = block_height;
        }
    }

    /// Get the starting height for terminal sync
    ///
    /// Returns the higher of checkpoint_height and last_terminal_block,
    /// since we don't need to re-process blocks we've already handled.
    pub fn terminal_sync_start_height(&self) -> u64 {
        self.checkpoint_height.max(self.last_terminal_block)
    }

    /// Reset the sync state
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_needs_full_sync() {
        let state = PlatformSyncState::new();
        assert!(state.needs_full_sync(1000));
    }

    #[test]
    fn test_recent_sync_no_full_sync_needed() {
        let mut state = PlatformSyncState::new();
        state.update_after_full_sync(1000, 100);

        // 1 hour later - no full sync needed
        assert!(!state.needs_full_sync(1000 + 3600));
    }

    #[test]
    fn test_old_sync_needs_full_sync() {
        let mut state = PlatformSyncState::new();
        state.update_after_full_sync(1000, 100);

        // 7 days later - full sync needed
        let seven_days = 7 * 24 * 60 * 60;
        assert!(state.needs_full_sync(1000 + seven_days));
    }

    #[test]
    fn test_terminal_sync_start_height() {
        let mut state = PlatformSyncState::new();
        state.checkpoint_height = 100;
        state.last_terminal_block = 150;

        assert_eq!(state.terminal_sync_start_height(), 150);

        state.last_terminal_block = 50;
        assert_eq!(state.terminal_sync_start_height(), 100);
    }
}
