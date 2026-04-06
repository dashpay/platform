//! Delta-based changesets for the platform wallet.
//!
//! This module provides:
//!
//! - [`Merge`] — a trait for composing changeset deltas.
//! - [`PlatformWalletChangeSet`] — the top-level delta type encompassing all wallet state.
//! - [`PlatformWalletPersistence`] — storage backend trait.

pub mod changeset;
pub mod merge;
pub mod traits;

pub use changeset::{
    AccountChangeSet, AssetLockChangeSet, AssetLockEntry, ChainChangeSet, ContactChangeSet,
    ContactRequestEntry, IdentityChangeSet, IdentityEntry, PlatformAddressChangeSet,
    PlatformAddressEntry, PlatformWalletChangeSet, TransactionChangeSet, TransactionEntry,
    UtxoChangeSet,
};
pub use merge::Merge;
pub use traits::PlatformWalletPersistence;
