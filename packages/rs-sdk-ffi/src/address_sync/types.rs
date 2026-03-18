//! FFI-compatible types for address synchronization

/// Configuration for address synchronization (FFI-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DashSDKAddressSyncConfig {
    /// Minimum privacy count - subtrees smaller than this will be expanded
    /// to include ancestor subtrees for better privacy.
    ///
    /// Higher values provide better privacy but may increase the number of
    /// elements returned per query.
    ///
    /// Default: 32
    pub min_privacy_count: u64,

    /// Maximum concurrent branch queries.
    ///
    /// Higher values can speed up synchronization but increase memory usage
    /// and network load.
    ///
    /// Default: 10
    pub max_concurrent_requests: u32,

    /// Maximum number of iterations (safety limit).
    ///
    /// The sync process iterates until all addresses are resolved. This limit
    /// prevents infinite loops in case of unexpected behavior.
    ///
    /// Default: 50
    pub max_iterations: u32,

    /// Maximum age in seconds before a full tree rescan is forced.
    ///
    /// When `last_sync_timestamp` is provided, elapsed time is compared against
    /// this threshold. If exceeded, a full tree rescan is performed instead of
    /// incremental-only catch-up.
    ///
    /// Set to 0 to always do a full tree scan.
    ///
    /// Default: 604800 (7 days)
    pub full_rescan_after_time_s: u64,
}

impl Default for DashSDKAddressSyncConfig {
    fn default() -> Self {
        Self {
            min_privacy_count: 32,
            max_concurrent_requests: 10,
            max_iterations: 50,
            full_rescan_after_time_s: 7 * 24 * 60 * 60,
        }
    }
}

/// Metrics about the synchronization process (FFI-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DashSDKAddressSyncMetrics {
    /// Number of trunk queries (0 for incremental-only, 1 for full scan)
    pub trunk_queries: u32,

    /// Number of branch queries
    pub branch_queries: u32,

    /// Total elements seen across all proofs.
    ///
    /// This gives an indication of the "anonymity set" - how many addresses
    /// were potentially being queried from the server's perspective.
    pub total_elements_seen: u32,

    /// Total proof bytes received
    pub total_proof_bytes: u32,

    /// Number of iterations (0 = trunk only, 1+ = trunk plus branch rounds)
    pub iterations: u32,

    /// Number of compacted incremental queries (historical aggregated changes)
    pub compacted_queries: u32,

    /// Number of recent incremental queries (per-block changes)
    pub recent_queries: u32,
}

/// A found address with its balance (FFI-compatible)
#[repr(C)]
#[derive(Debug)]
pub struct DashSDKFoundAddress {
    /// The derivation index for this address
    pub index: u32,

    /// Pointer to the address key bytes
    pub key: *mut u8,

    /// Length of the key in bytes
    pub key_len: usize,

    /// Nonce associated with this address
    pub nonce: u32,

    /// Balance in credits at this address
    pub balance: u64,
}

/// An address proven absent from the tree (FFI-compatible)
#[repr(C)]
#[derive(Debug)]
pub struct DashSDKAbsentAddress {
    /// The derivation index for this address
    pub index: u32,

    /// Pointer to the address key bytes
    pub key: *mut u8,

    /// Length of the key in bytes
    pub key_len: usize,
}

/// Result of address synchronization (FFI-compatible)
#[repr(C)]
#[derive(Debug)]
pub struct DashSDKAddressSyncResult {
    /// Array of found addresses with balances
    pub found: *mut DashSDKFoundAddress,

    /// Number of found addresses
    pub found_count: usize,

    /// Array of addresses proven absent
    pub absent: *mut DashSDKAbsentAddress,

    /// Number of absent addresses
    pub absent_count: usize,

    /// Highest found index (for HD wallets)
    /// Only valid if has_highest_found_index is true
    pub highest_found_index: u32,

    /// Whether highest_found_index is valid
    pub has_highest_found_index: bool,

    /// The checkpoint height from the trunk/branch tree scan.
    /// Only meaningful when a full tree scan was performed (0 otherwise).
    pub checkpoint_height: u64,

    /// The new sync height to store for the next incremental sync.
    ///
    /// Return this value from the provider's `last_sync_height()` on the
    /// next call.
    pub new_sync_height: u64,

    /// Platform block time (Unix seconds) at the point of the latest response.
    ///
    /// Pass this value as `last_sync_timestamp` on the next call to
    /// `dash_sdk_sync_address_balances`.
    pub new_sync_timestamp: u64,

    /// The highest block height from the most recent per-block balance changes.
    ///
    /// Store this value and pass it as `last_known_recent_block` on the next
    /// call to enable efficient compaction detection via boundary checks.
    /// A value of 0 means no recent block was observed.
    pub last_known_recent_block: u64,

    /// Metrics about the sync process
    pub metrics: DashSDKAddressSyncMetrics,
}

/// A pending address entry for the provider callback
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DashSDKPendingAddress {
    /// The derivation index for this address
    pub index: u32,

    /// Pointer to the address key bytes (32 bytes)
    pub key: *const u8,

    /// Length of the key in bytes
    pub key_len: usize,
}

/// List of pending addresses returned by the provider callback
#[repr(C)]
#[derive(Debug)]
pub struct DashSDKPendingAddressList {
    /// Array of pending addresses
    pub addresses: *mut DashSDKPendingAddress,

    /// Number of addresses
    pub count: usize,
}

impl DashSDKPendingAddressList {
    /// Create an empty list
    pub fn empty() -> Self {
        Self {
            addresses: std::ptr::null_mut(),
            count: 0,
        }
    }
}
