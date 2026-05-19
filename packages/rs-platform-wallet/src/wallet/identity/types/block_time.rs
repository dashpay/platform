//! Block time information for synchronization tracking
//!
//! This module provides the `BlockTime` struct which contains block height,
//! core chain height, and timestamp information for tracking sync state.

use dpp::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};

/// Block time information containing height, core height, and timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockTime {
    /// Platform block height
    pub height: BlockHeight,

    /// Core chain block height
    pub core_height: CoreBlockHeight,

    /// Block timestamp in milliseconds since epoch
    pub timestamp: TimestampMillis,
}

impl BlockTime {
    /// Create a new BlockTime
    pub fn new(
        height: BlockHeight,
        core_height: CoreBlockHeight,
        timestamp: TimestampMillis,
    ) -> Self {
        Self {
            height,
            core_height,
            timestamp,
        }
    }

    /// Check if this block time is older than a given age in milliseconds
    pub fn is_older_than(&self, current_timestamp: TimestampMillis, max_age_millis: u64) -> bool {
        (current_timestamp - self.timestamp) > max_age_millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_time_creation() {
        let block_time = BlockTime::new(100000, 900000, 1234567890);

        assert_eq!(block_time.height, 100000);
        assert_eq!(block_time.core_height, 900000);
        assert_eq!(block_time.timestamp, 1234567890);
    }

    #[test]
    fn test_is_older_than() {
        let block_time = BlockTime::new(100000, 900000, 1000);

        // Not old enough
        assert_eq!(block_time.is_older_than(1050, 100), false);

        // Old enough
        assert_eq!(block_time.is_older_than(1200, 100), true);

        // Exactly at the threshold
        assert_eq!(block_time.is_older_than(1100, 100), false);

        // Just over the threshold
        assert_eq!(block_time.is_older_than(1101, 100), true);
    }
}
