//! Types for address synchronization.

use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use rs_dapi_client::RequestSettings;
use std::collections::{BTreeMap, BTreeSet};

/// A 32-byte address key that we're searching for in the address funds tree.
/// This is derived from an address (e.g., hash of public key).
pub type AddressKey = Vec<u8>;

/// The derivation index for an address (for HD wallets).
pub type AddressIndex = u32;

/// A key at the truncation boundary of a trunk/branch query result.
/// This represents a subtree root whose contents weren't fully returned.
/// Target keys that fall within this subtree's range need a branch query to resolve.
pub type LeafBoundaryKey = Vec<u8>;

/// Configuration for address synchronization.
#[derive(Debug, Clone)]
pub struct AddressSyncConfig {
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
    pub max_concurrent_requests: usize,

    /// Maximum number of iterations (safety limit).
    ///
    /// The sync process iterates until all addresses are resolved. This limit
    /// prevents infinite loops in case of unexpected behavior.
    ///
    /// Default: 50
    pub max_iterations: usize,

    /// Request settings for undergoing address sync queries.
    pub request_settings: RequestSettings,
}

impl Default for AddressSyncConfig {
    fn default() -> Self {
        Self {
            min_privacy_count: 32,
            max_concurrent_requests: 10,
            max_iterations: 50,
            request_settings: RequestSettings::default(),
        }
    }
}

/// Result of address synchronization.
#[derive(Debug)]
pub struct AddressSyncResult {
    /// Addresses found with their balances and nonces.
    ///
    /// Map of `(index, key)` to address funds.
    pub found: BTreeMap<(AddressIndex, AddressKey), AddressFunds>,

    /// Addresses proven absent from the tree.
    ///
    /// Set of `(index, key)` tuples that were proven to not exist.
    pub absent: BTreeSet<(AddressIndex, AddressKey)>,

    /// Highest found index (for HD wallets).
    ///
    /// This is the highest address index that was found in the tree.
    pub highest_found_index: Option<AddressIndex>,

    /// Metrics about the sync process.
    pub metrics: AddressSyncMetrics,

    /// The checkpoint height at which balances were synced.
    ///
    /// This is the block height from which terminal balance updates should start
    /// to catch any changes that occurred after the checkpoint.
    pub checkpoint_height: u64,
}

impl AddressSyncResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
            highest_found_index: None,
            metrics: AddressSyncMetrics::default(),
            checkpoint_height: 0,
        }
    }

    /// Get total credits across all found addresses.
    pub fn total_balance(&self) -> u64 {
        self.found.values().map(|funds| funds.balance).sum()
    }

    /// Get count of addresses found with non-zero balance.
    pub fn non_zero_count(&self) -> usize {
        self.found
            .values()
            .filter(|funds| funds.balance > 0)
            .count()
    }
}

impl Default for AddressSyncResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics about the synchronization process.
#[derive(Debug, Default, Clone)]
pub struct AddressSyncMetrics {
    /// Number of trunk queries (always 1 for a successful sync).
    pub trunk_queries: usize,

    /// Number of branch queries.
    pub branch_queries: usize,

    /// Total elements seen across all proofs.
    ///
    /// This gives an indication of the "anonymity set" - how many addresses
    /// were potentially being queried from the server's perspective.
    pub total_elements_seen: usize,

    /// Total proof bytes received.
    pub total_proof_bytes: usize,

    /// Number of iterations (0 = trunk only, 1+ = trunk plus branch rounds).
    pub iterations: usize,
}

/// Funds stored for a platform address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressFunds {
    /// Address nonce used for anti-replay.
    pub nonce: AddressNonce,
    /// Credits balance held by the address.
    pub balance: Credits,
}

impl AddressSyncMetrics {
    /// Get total number of queries (trunk + branch).
    pub fn total_queries(&self) -> usize {
        self.trunk_queries + self.branch_queries
    }

    /// Get average proof size in bytes.
    pub fn average_proof_bytes(&self) -> f64 {
        let total = self.total_queries();
        if total == 0 {
            0.0
        } else {
            self.total_proof_bytes as f64 / total as f64
        }
    }
}
