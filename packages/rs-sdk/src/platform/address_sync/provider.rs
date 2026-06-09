//! Address provider trait for address synchronization.

use super::types::{AddressFunds, AddressIndex, AddressToBytes};
use async_trait::async_trait;
use std::hash::Hash;

/// Trait for providing addresses to be synchronized.
///
/// This trait abstracts the address derivation and tracking logic, allowing different
/// wallet implementations (HD wallets, single addresses, multi-wallet coordinators,
/// …) to be used with the sync mechanism.
///
/// # The `Tag` associated type
///
/// The sync engine is agnostic to how a provider identifies its pending
/// addresses. Each provider picks an associated [`Tag`](Self::Tag) type
/// — typically an integer derivation index for single-account HD
/// wallets, a composite `(wallet, account, index)` tuple for
/// multi-wallet coordinators, or `()` for single-address providers.
/// The engine passes the tag back verbatim in
/// [`on_address_found`](Self::on_address_found) and
/// [`on_address_absent`](Self::on_address_absent) and stores it in the
/// result keys so the provider can route each callback to the right
/// internal slot without a side lookup.
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
    /// Opaque identifier the provider uses to tag pending addresses.
    ///
    /// The engine carries this tag through its internal state and
    /// passes it back verbatim to the provider's callbacks. The engine
    /// never interprets the tag beyond using it as a map key, so any
    /// `Copy + Ord + Eq + Hash + Send + Sync` type works.
    type Tag: Copy + Ord + Eq + Hash + Send + Sync;

    /// Concrete address type the provider uses. Must implement
    /// [`AddressToBytes`] so the engine can compute GroveDB lookup
    /// keys and round-trip raw key bytes back into the provider's
    /// address type.
    ///
    /// Most providers will pick
    /// [`dpp::address_funds::PlatformAddress`] — callers that only
    /// care about a single variant (e.g. P2PKH) can pick a narrower
    /// type and skip enum-wrap/unwrap work at the trait boundary.
    type Address: AddressToBytes;

    /// Get the gap limit for this provider.
    ///
    /// For HD wallets, this is the number of consecutive unused addresses
    /// that must be checked before assuming no more addresses are in use.
    ///
    /// For non-HD wallets, this can return 0 or any value as it won't affect behavior.
    fn gap_limit(&self) -> AddressIndex;

    /// Get currently pending addresses to synchronize.
    ///
    /// Yields tuples of `(tag, address)` where:
    /// - `tag` is the provider-specific identifier (see [`Tag`](Self::Tag))
    /// - `address` is the address to look up in the address funds tree
    ///
    /// This set may grow when [`on_address_found`](Self::on_address_found) triggers
    /// gap extension. The engine calls `pending_addresses` multiple times
    /// per sync (once per trunk/branch round), so the returned iterator
    /// should be cheap to construct — typically a view into an internal
    /// map rather than a computed list.
    ///
    /// Must be a superset of [`current_balances`](Self::current_balances);
    /// see the invariant documented there.
    fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_;

    /// Called when an address is found in the tree with a balance.
    ///
    /// For HD wallets, this should:
    /// 1. Record the found address and its balance
    /// 2. Potentially extend the search range if this extends the highest known index
    ///
    /// # Arguments
    /// - `tag`: The provider-supplied tag for this pending address
    /// - `address`: The address that was found
    /// - `funds`: The nonce and credits balance at this address
    async fn on_address_found(
        &mut self,
        tag: Self::Tag,
        address: &Self::Address,
        funds: AddressFunds,
    );

    /// Called when an address is proven absent from the tree.
    ///
    /// The provider can use this to:
    /// - Remove the address from pending
    /// - Update internal tracking state
    ///
    /// # Arguments
    /// - `tag`: The provider-supplied tag for this pending address
    /// - `address`: The address proven absent
    async fn on_address_absent(&mut self, tag: Self::Tag, address: &Self::Address);

    /// Check if there are still pending addresses to synchronize.
    ///
    /// Default implementation checks whether
    /// [`pending_addresses`](Self::pending_addresses) yields at least
    /// one element. Implementors with a length-tracked internal set
    /// should override this with an O(1) check to avoid constructing
    /// the iterator just to peek at its first element.
    fn has_pending(&self) -> bool {
        self.pending_addresses().next().is_some()
    }

    /// Signal that a sync pass has completed successfully.
    ///
    /// The engine calls this once after
    /// [`sync_address_balances`](super::sync_address_balances) has
    /// finished its tree-scan and incremental-catch-up phases without
    /// error. Implementors can use it as a commit point — e.g. to
    /// flip a staged "in-sync" scratch set into the provider's
    /// canonical state. Default impl is a no-op.
    async fn sync_finished(&mut self) {}

    /// Get current known balances for incremental catch-up.
    ///
    /// Yields tuples of `(tag, address, funds)` for addresses that have
    /// known state from a previous sync. Used during incremental-only
    /// mode to provide base balances for applying `AddToCredits` delta
    /// operations.
    ///
    /// Returning an iterator rather than a slice means the provider
    /// does not have to maintain a flat cache in the slice's shape —
    /// it can yield from whatever internal structure it keeps. The
    /// `impl Iterator` return (stable RPITIT since Rust 1.75) means the
    /// iterator is statically dispatched with no heap allocation.
    ///
    /// # Invariant
    ///
    /// Every address yielded by `current_balances` MUST also appear in
    /// [`pending_addresses`](Self::pending_addresses) (with the same
    /// `(tag, address)` pairing). The engine builds its entry-time lookup
    /// from `pending_addresses`; an incremental change for an address that
    /// is in `current_balances` but absent from `pending_addresses` is
    /// buffered as unknown and dropped at end-of-pass (the same silent-loss
    /// class as a post-snapshot address that never resolves). Production HD
    /// providers satisfy this by construction — both views project the same
    /// derived-address set. Bridge providers that accept two independent
    /// caller-supplied arrays (e.g. the FFI batch provider) must enforce it
    /// themselves. The engine folds `current_balances` keys into its lookup
    /// defensively, but providers should not rely on that.
    fn current_balances(
        &self,
    ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_;

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
