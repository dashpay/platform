//! Types for nullifier synchronization.

use rs_dapi_client::RequestSettings;
use std::collections::BTreeSet;

/// A 32-byte nullifier key.
pub type NullifierKey = [u8; 32];

/// Configuration for nullifier synchronization.
#[derive(Debug, Clone)]
pub struct NullifierSyncConfig {
    /// Minimum privacy count - subtrees smaller than this will be expanded
    /// to include ancestor subtrees for better privacy.
    ///
    /// Default: 32
    pub min_privacy_count: u64,

    /// Maximum concurrent branch queries.
    ///
    /// Default: 10
    pub max_concurrent_requests: usize,

    /// Maximum number of iterations (safety limit).
    ///
    /// Default: 50
    pub max_iterations: usize,

    /// The shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    ///
    /// Default: 0 (credit pool)
    pub pool_type: u32,

    /// Optional 32-byte identifier for individual token pools.
    ///
    /// Default: None
    pub pool_identifier: Option<Vec<u8>>,

    /// Maximum age in seconds before a full tree rescan is forced.
    ///
    /// When `last_sync_timestamp` is passed to [`sync_nullifiers`](super::sync_nullifiers),
    /// the function compares `now - last_sync_timestamp` against this threshold.
    /// If the elapsed time exceeds this value, a full tree rescan is performed
    /// instead of incremental-only catch-up.
    ///
    /// Set to `0` to always do a full tree scan regardless of the timestamp.
    ///
    /// Default: 604800 (7 days)
    pub full_rescan_after_time_s: u64,

    /// Request settings for nullifier sync queries.
    pub request_settings: RequestSettings,
}

impl Default for NullifierSyncConfig {
    fn default() -> Self {
        Self {
            min_privacy_count: 32,
            max_concurrent_requests: 10,
            max_iterations: 50,
            pool_type: 0,
            pool_identifier: None,
            full_rescan_after_time_s: 7 * 24 * 60 * 60,
            request_settings: RequestSettings::default(),
        }
    }
}

/// Result of nullifier synchronization.
#[derive(Debug)]
pub struct NullifierSyncResult {
    /// Nullifiers found in the tree (spent).
    pub found: BTreeSet<NullifierKey>,

    /// Nullifiers proven absent from the tree (unspent).
    pub absent: BTreeSet<NullifierKey>,

    /// Metrics about the sync process.
    pub metrics: NullifierSyncMetrics,

    /// The checkpoint height from the trunk/branch scan.
    ///
    /// This is the block height at which the tree snapshot was taken.
    /// Only meaningful when a full tree scan was performed.
    pub checkpoint_height: u64,

    /// The highest block height seen from the incremental phase
    /// (or the checkpoint height if no incremental phase ran).
    ///
    /// After each sync the caller should persist two values:
    /// 1. This `new_sync_height` — pass it back as `last_sync_height` on the
    ///    next call to [`sync_nullifiers`](super::sync_nullifiers).
    /// 2. [`new_sync_timestamp`](Self::new_sync_timestamp) — pass it as the
    ///    `last_sync_timestamp` parameter of [`sync_nullifiers`](super::sync_nullifiers).
    pub new_sync_height: u64,

    /// Platform block time (Unix seconds) at the point of the latest response.
    ///
    /// Store this value and pass it back as `last_sync_timestamp` on the next
    /// call to [`sync_nullifiers`](super::sync_nullifiers). The function compares
    /// it against the current wall-clock time to decide whether a full tree
    /// rescan is needed.
    pub new_sync_timestamp: u64,
}

impl NullifierSyncResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            found: BTreeSet::new(),
            absent: BTreeSet::new(),
            metrics: NullifierSyncMetrics::default(),
            checkpoint_height: 0,
            new_sync_height: 0,
            new_sync_timestamp: 0,
        }
    }
}

impl Default for NullifierSyncResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics about the nullifier synchronization process.
#[derive(Debug, Default, Clone)]
pub struct NullifierSyncMetrics {
    /// Number of trunk queries (0 for incremental-only, 1 for full scan).
    pub trunk_queries: usize,

    /// Number of branch queries.
    pub branch_queries: usize,

    /// Total elements seen across all proofs.
    pub total_elements_seen: usize,

    /// Total proof bytes received.
    pub total_proof_bytes: usize,

    /// Number of branch iterations (0 = trunk only, 1+ = trunk plus branch rounds).
    pub iterations: usize,

    /// Number of compacted incremental queries.
    pub compacted_queries: usize,

    /// Number of recent incremental queries.
    pub recent_queries: usize,
}

impl NullifierSyncMetrics {
    /// Get total number of queries (trunk + branch + incremental).
    pub fn total_queries(&self) -> usize {
        self.trunk_queries + self.branch_queries + self.compacted_queries + self.recent_queries
    }
}
