//! FFI-compatible types for nullifier synchronization.

/// Configuration for nullifier BLAST sync.
///
/// Pass null to `dash_sdk_sync_nullifiers` to use default values.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DashSDKNullifierSyncConfig {
    /// Minimum privacy count — subtrees smaller than this are expanded.
    /// Default: 32
    pub min_privacy_count: u64,

    /// Maximum concurrent branch queries.
    /// Default: 10
    pub max_concurrent_requests: u32,

    /// Maximum number of iterations (safety limit).
    /// Default: 50
    pub max_iterations: u32,

    /// Shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// Default: 0
    pub pool_type: u32,

    /// Optional 32-byte pool identifier for individual token pools.
    /// Only used when `has_pool_identifier` is true.
    pub pool_identifier: [u8; 32],

    /// Whether `pool_identifier` is valid.
    pub has_pool_identifier: bool,

    /// Maximum age in seconds before a full tree rescan is forced.
    /// Default: 604800 (7 days)
    pub full_rescan_after_time_s: u64,
}

impl Default for DashSDKNullifierSyncConfig {
    fn default() -> Self {
        Self {
            min_privacy_count: 32,
            max_concurrent_requests: 10,
            max_iterations: 50,
            pool_type: 0,
            pool_identifier: [0u8; 32],
            has_pool_identifier: false,
            full_rescan_after_time_s: 7 * 24 * 60 * 60,
        }
    }
}

/// Metrics about the nullifier sync process.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct DashSDKNullifierSyncMetrics {
    pub trunk_queries: u32,
    pub branch_queries: u32,
    pub total_elements_seen: u32,
    pub total_proof_bytes: u32,
    pub branch_query_failures: u32,
    pub iterations: u32,
    pub compacted_queries: u32,
    pub recent_queries: u32,
}

/// Result of nullifier BLAST sync.
///
/// `found` and `absent` are contiguous arrays of 32-byte nullifiers.
/// Free with `dash_sdk_nullifier_sync_result_free`.
#[repr(C)]
pub struct DashSDKNullifierSyncResult {
    /// Contiguous array of found (spent) nullifiers, each 32 bytes.
    pub found: *mut u8,
    /// Number of found nullifiers.
    pub found_count: u32,
    /// Contiguous array of absent (unspent) nullifiers, each 32 bytes.
    pub absent: *mut u8,
    /// Number of absent nullifiers.
    pub absent_count: u32,
    /// Block height of the tree snapshot (from trunk/branch scan).
    pub checkpoint_height: u64,
    /// Highest block height seen — persist for next sync call.
    pub new_sync_height: u64,
    /// Block time at the latest response — persist for next sync call.
    pub new_sync_timestamp: u64,
    /// Sync metrics.
    pub metrics: DashSDKNullifierSyncMetrics,
}
