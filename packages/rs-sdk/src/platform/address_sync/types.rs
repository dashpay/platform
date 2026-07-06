//! Types for address synchronization.

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use rs_dapi_client::RequestSettings;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;

/// Abstraction for address types that encode to GroveDB key bytes in
/// the address-funds tree.
///
/// The server stores address-funds entries keyed by
/// `PlatformAddress::to_bytes()` (1-byte variant tag + 20-byte hash).
/// Any provider type used as
/// [`AddressProvider::Address`](super::provider::AddressProvider::Address)
/// must encode to those same bytes — regardless of what the provider
/// stores internally (the full enum, a P2PKH-only newtype, a custom
/// tag, etc.).
pub trait AddressToBytes: Copy + Ord + Eq + Hash + Send + Sync {
    /// Encode this address as the GroveDB address-funds key bytes.
    fn to_bytes(&self) -> Vec<u8>;
}

impl AddressToBytes for PlatformAddress {
    fn to_bytes(&self) -> Vec<u8> {
        PlatformAddress::to_bytes(self)
    }
}

impl AddressToBytes for dpp::key_wallet::PlatformP2PKHAddress {
    /// Encodes to the same 21-byte form as
    /// `PlatformAddress::P2pkh(self.to_bytes()).to_bytes()` — the
    /// server's address-funds tree keys everything through the
    /// `PlatformAddress` enum encoding, so a P2PKH-only provider has
    /// to produce the same byte sequence.
    fn to_bytes(&self) -> Vec<u8> {
        PlatformAddress::P2pkh(self.to_bytes()).to_bytes()
    }
}

/// The derivation index for an address (for HD wallets).
pub type AddressIndex = u32;

/// A key at the truncation boundary of a trunk/branch query result.
/// This represents a subtree root whose contents weren't fully returned.
/// Target keys that fall within this subtree's range need a branch query to resolve.
pub type LeafBoundaryKey = Vec<u8>;

/// Funds stored for a platform address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AddressFunds {
    /// Address nonce used for anti-replay.
    pub nonce: AddressNonce,
    /// Credits balance held by the address.
    pub balance: Credits,
    /// Platform block height this balance is current **as of** — the
    /// height pin.
    ///
    /// The pin means: `balance` includes the effect of every block up to
    /// and including `as_of_height`. It is the reconciliation rule between
    /// the two sources of truth for an address balance:
    ///
    /// - **Direct truth** — a proof-attested absolute (state-transition
    ///   result, trunk/branch scan element). It arrives pinned at its
    ///   proof's block height.
    /// - **Aggregate truth** — the recent/compacted balance-change delta
    ///   stream. A delta recorded at block `B` may only be applied when
    ///   `B > as_of_height` (otherwise it is already included in the
    ///   absolute); applying it advances the pin to `B`.
    ///
    /// Freshness between two absolutes is decided by comparing pins — a
    /// later pin is authoritative *even when it revises the balance
    /// downward* (nonces only advance on outgoing ops, so they cannot
    /// order receive-only state).
    ///
    /// `0` means "unknown provenance" (legacy rows persisted before the
    /// pin existed): every delta applies, matching pre-pin behavior.
    pub as_of_height: u64,
}
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

    /// Maximum age in seconds before a full tree rescan is forced.
    ///
    /// When `last_sync_timestamp` is passed to [`sync_address_balances`](super::sync_address_balances), the
    /// function compares `now - last_sync_timestamp` against this threshold.
    /// If the elapsed time exceeds this value, a full tree rescan is performed
    /// instead of incremental-only catch-up.
    ///
    /// Set to `0` to always do a full tree scan regardless of the timestamp.
    ///
    /// Default: 604800 (7 days)
    pub full_rescan_after_time_s: u64,

    /// Request settings for undergoing address sync queries.
    pub request_settings: RequestSettings,
}

impl Default for AddressSyncConfig {
    fn default() -> Self {
        Self {
            min_privacy_count: 32,
            max_concurrent_requests: 10,
            max_iterations: 50,
            full_rescan_after_time_s: 6 * 24 * 3600 + 23 * 3600 + 45 * 60, // 6 days 23 hours 45 minutes
            request_settings: RequestSettings::default(),
        }
    }
}

/// Result of address synchronization.
///
/// Generic over the provider's [`Tag`](super::AddressProvider::Tag)
/// and [`Address`](super::AddressProvider::Address) so the keys in
/// `found` / `absent` carry whatever metadata and address type the
/// provider chose. Matches the provider's associated types.
#[derive(Debug)]
pub struct AddressSyncResult<Tag, Address> {
    /// Addresses found with their balances and nonces.
    ///
    /// Map of `(tag, address)` to address funds.
    pub found: BTreeMap<(Tag, Address), AddressFunds>,

    /// Addresses proven absent from the tree.
    ///
    /// Set of `(tag, address)` tuples that were proven to not exist.
    pub absent: BTreeSet<(Tag, Address)>,

    /// Metrics about the sync process.
    pub metrics: AddressSyncMetrics,

    /// The checkpoint height from the trunk/branch tree scan.
    ///
    /// This is the block height at which the tree snapshot was taken.
    /// Only meaningful when a full tree scan was performed.
    pub checkpoint_height: u64,

    /// The highest block height seen from the incremental phase
    /// (or the checkpoint height if no incremental phase ran).
    ///
    /// After each sync the caller should persist two values:
    /// 1. This `new_sync_height` — return it from
    ///    [`AddressProvider::last_sync_height`](super::provider::AddressProvider::last_sync_height) on the next call.
    /// 2. [`new_sync_timestamp`](Self::new_sync_timestamp) — pass it as the
    ///    `last_sync_timestamp` parameter of [`sync_address_balances`](super::sync_address_balances).
    pub new_sync_height: u64,

    /// Platform block time (Unix seconds) at the point of the latest response.
    ///
    /// Store this value and pass it back as `last_sync_timestamp` on the next
    /// call to [`sync_address_balances`](super::sync_address_balances). The function compares it against the
    /// current wall-clock time to decide whether a full tree rescan is needed.
    pub new_sync_timestamp: u64,

    /// The highest block height that was actually present in the most recent
    /// batch of per-block address balance changes.
    ///
    /// On subsequent syncs, this height is used as the exclusive start for the
    /// recent query (`RangeAfter`), causing it to appear as a boundary node in
    /// the GroveDB proof. This enables `key_exists_as_boundary` to detect
    /// whether the height has been compacted away.
    ///
    /// Store this value and return it from
    /// [`AddressProvider::last_known_recent_block_height`](super::provider::AddressProvider::last_known_recent_block_height)
    /// on the next call.
    /// A value of `0` means no recent block has been observed yet.
    pub last_known_recent_block: u64,

    /// Raw GroveDB proof bytes from the most recent query (for debugging).
    /// Empty if no proof was captured.
    pub recent_proof: Vec<u8>,
}

impl<Tag, Address> AddressSyncResult<Tag, Address>
where
    Tag: Ord,
    Address: Ord,
{
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
            metrics: AddressSyncMetrics::default(),
            checkpoint_height: 0,
            new_sync_height: 0,
            new_sync_timestamp: 0,
            last_known_recent_block: 0,
            recent_proof: Vec::new(),
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

impl<Tag, Address> Default for AddressSyncResult<Tag, Address>
where
    Tag: Ord,
    Address: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics about the synchronization process.
#[derive(Debug, Default, Clone)]
pub struct AddressSyncMetrics {
    /// Number of trunk queries (0 for incremental-only, 1 for full scan).
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

    /// Number of compacted incremental queries.
    pub compacted_queries: usize,

    /// Number of recent incremental queries.
    pub recent_queries: usize,

    /// Total block entries returned by recent queries (all addresses, not just ours).
    pub recent_entries_returned: usize,

    /// Total block entries returned by compacted queries.
    pub compacted_entries_returned: usize,
}

impl AddressSyncMetrics {
    /// Get total number of queries (trunk + branch + incremental).
    pub fn total_queries(&self) -> usize {
        self.trunk_queries + self.branch_queries + self.compacted_queries + self.recent_queries
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
