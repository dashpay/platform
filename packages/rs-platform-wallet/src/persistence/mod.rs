//! Delta-based persistence for the platform wallet.
//!
//! This module provides:
//!
//! - [`Merge`] — a trait for composing changeset deltas.
//! - [`WalletChangeSet`] — the top-level delta type encompassing all wallet state.
//! - [`WalletPersistence`] / [`AsyncWalletPersistence`] — storage backend traits.

pub mod changeset;
pub mod merge;
pub mod traits;

pub use changeset::WalletChangeSet;
pub use merge::Merge;
pub use traits::AsyncWalletPersistence;
pub use traits::WalletPersistence;
