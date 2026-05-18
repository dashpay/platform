//! Delta-based changesets for the platform wallet.
//!
//! This module provides:
//!
//! - [`Merge`] — a trait for composing changeset deltas.
//! - [`PlatformWalletChangeSet`] — the top-level delta type encompassing all wallet state.
//! - [`ClientStartState`] — the snapshot returned by
//!   [`PlatformWalletPersistence::load`] so the platform wallet can boot without re-syncing.
//! - [`PlatformWalletPersistence`] — storage backend trait.

#[allow(clippy::module_inception)]
pub mod changeset;
pub mod client_start_state;
pub mod client_wallet_start_state;
pub mod core_bridge;
pub mod identity_manager_start_state;
pub mod merge;
pub mod platform_address_sync_start_state;
#[cfg(feature = "serde")]
pub mod serde_adapters;
pub mod traits;

pub use changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, AssetLockChangeSet, AssetLockEntry,
    ContactChangeSet, ContactRequestEntry, CoreChangeSet, IdentityChangeSet, IdentityEntry,
    IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet,
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    ReceivedContactRequestKey, SentContactRequestKey, TokenBalanceChangeSet, WalletMetadataEntry,
};
pub use client_start_state::ClientStartState;
pub use client_wallet_start_state::ClientWalletStartState;
pub use core_bridge::spawn_wallet_event_adapter;
pub use identity_manager_start_state::IdentityManagerStartState;
pub use merge::Merge;
pub use platform_address_sync_start_state::PlatformAddressSyncStartState;
pub use traits::{PersistenceError, PlatformWalletPersistence};
