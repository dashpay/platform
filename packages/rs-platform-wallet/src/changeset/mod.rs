//! Delta-based changesets for the platform wallet.
//!
//! This module provides:
//!
//! - [`Merge`] — a trait for composing changeset deltas.
//! - [`PlatformWalletChangeSet`] — the top-level delta type encompassing all wallet state.
//! - [`PlatformWalletPersistence`] — storage backend trait.

pub mod changeset;
pub mod core_bridge;
pub mod merge;
pub mod traits;

pub use changeset::{
    AssetLockChangeSet, AssetLockEntry, ContactChangeSet, ContactRequestEntry, IdentityChangeSet,
    IdentityEntry, PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    ReceivedContactRequestKey, SentContactRequestKey, TokenBalanceChangeSet,
};
pub use core_bridge::CorePersistenceBridge;
pub use merge::Merge;
pub use traits::PlatformWalletPersistence;
