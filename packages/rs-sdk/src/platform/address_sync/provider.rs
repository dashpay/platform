//! Address provider trait for address synchronization.

use super::types::{AddressFunds, AddressIndex};
use dpp::address_funds::PlatformAddress;
use async_trait::async_trait;

/// Trait for providing addresses to be synchronized.
///
/// This trait abstracts the address derivation and tracking logic, allowing different
/// wallet implementations (HD wallets, single addresses, etc.) to be used with the
/// sync mechanism.
///
/// # Gap Limit Behavior
///
/// For HD wallets, the gap limit determines how many unused addresses to check
/// beyond the last known used address. When an address is found:
///
/// 1. The provider is notified via [`on_address_found`](AddressProvider::on_address_found)
/// 2. The provider can extend [`pending_addresses`](AddressProvider::pending_addresses)
///    to include more addresses
/// 3. Sync continues until all pending addresses are resolved
///
/// # Async mutation contract
///
/// [`on_address_found`](Self::on_address_found) and
/// [`on_address_absent`](Self::on_address_absent) are `async` so implementations
/// can `.await` on internal state writes (for example, acquiring an async lock
/// or persisting to an async store) without having to `block_on` a runtime
/// from a sync trait body.  The sync engine awaits each callback in-order on
/// the caller's task; do not spawn detached tasks that outlive the returned
/// future, as the engine relies on the mutation having been applied before
/// the next iteration observes the updated pending set.
#[async_trait]
pub trait AddressProvider: Send {
    /// Get the gap limit for this provider.
    ///
    /// For HD wallets, this is the number of consecutive unused addresses
    /// that must be checked before assuming no more addresses are in use.
    ///
    /// For non-HD wallets, this can return 0 or any value as it won't affect behavior.
    fn gap_limit(&self) -> AddressIndex;

    /// Get currently pending addresses to synchronize.
    ///
    /// Returns tuples of `(index, address)` where:
    /// - `index` is the derivation index (for HD wallets) or a unique identifier
    /// - `address` is the platform address to look up in the address funds tree
    ///
    /// This set may grow when [`on_address_found`](Self::on_address_found) triggers
    /// gap extension.
    fn pending_addresses(&self) -> Vec<(AddressIndex, PlatformAddress)>;

    /// Called when an address is found in the tree with a balance.
    ///
    /// For HD wallets, this should:
    /// 1. Record the found address and its balance
    /// 2. Potentially extend the search range if this extends the highest known index
    ///
    /// # Arguments
    /// - `index`: The address index that was found
    /// - `address`: The platform address that was found
    /// - `funds`: The nonce and credits balance at this address
    async fn on_address_found(
        &mut self,
        index: AddressIndex,
        address: &PlatformAddress,
        funds: AddressFunds,
    );

    /// Called when an address is proven absent from the tree.
    ///
    /// The provider can use this to:
    /// - Remove the address from pending
    /// - Update internal tracking state
    ///
    /// # Arguments
    /// - `index`: The address index proven absent
    /// - `address`: The platform address proven absent
    async fn on_address_absent(&mut self, index: AddressIndex, address: &PlatformAddress);

    /// Check if there are still pending addresses to synchronize.
    ///
    /// Default implementation checks if [`pending_addresses`](Self::pending_addresses)
    /// is non-empty.
    fn has_pending(&self) -> bool {
        !self.pending_addresses().is_empty()
    }

    /// Get the current highest found index (if any).
    ///
    /// Used for reporting and gap extension logic.
    fn highest_found_index(&self) -> Option<AddressIndex>;

    /// Get current known balances for incremental catch-up.
    ///
    /// Returns tuples of `(index, address, funds)` for addresses that have
    /// known state from a previous sync. This is used during incremental-only
    /// mode to provide base balances for applying `AddToCredits` delta operations.
    fn current_balances(&self) -> &[(AddressIndex, PlatformAddress, AddressFunds)];

    /// Get the last sync height from a previous sync.
    ///
    /// Returns the [`new_sync_height`](super::AddressSyncResult::new_sync_height) value from the
    /// previous call. Used as the starting block height for incremental-only
    /// catch-up. The caller should store this value after each sync and
    /// return it here on subsequent calls.
    ///
    /// Default returns `0`, which means incremental catch-up starts from
    /// the genesis block (effectively a full catch-up).
    fn last_sync_height(&self) -> u64 {
        0
    }

    /// Get the last known recent block height from a previous sync.
    ///
    /// Returns the [`last_known_recent_block`](super::AddressSyncResult::last_known_recent_block)
    /// value from the previous call. This is the highest block height that
    /// was present in the most recent per-block address balance changes
    /// batch.
    ///
    /// When non-zero, the SDK uses `RangeAfter` (exclusive start) for the
    /// recent query, causing this height to appear as a boundary node in the
    /// GroveDB proof. This enables `key_exists_as_boundary` to detect
    /// whether the height has been compacted away, eliminating unnecessary
    /// compacted queries on the hot path.
    ///
    /// Default returns `0` (no prior recent block; use inclusive `RangeFrom`).
    fn last_known_recent_block_height(&self) -> u64 {
        0
    }
}
