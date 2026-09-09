//! Block time information for synchronization tracking
//!
//! This module provides the `BlockTime` struct which contains block height,
//! core chain height, and timestamp information for tracking sync state.

use dpp::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};

/// Block time information containing height, core height, and timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockTime {
    /// Platform block height
    pub height: BlockHeight,

    /// Core chain block height
    pub core_height: CoreBlockHeight,

    /// Block timestamp in milliseconds since epoch
    pub timestamp: TimestampMillis,
}
