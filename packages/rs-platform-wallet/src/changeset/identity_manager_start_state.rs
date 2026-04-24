//! Lean startup snapshot for [`IdentityManager`](crate::wallet::identity::IdentityManager).
//!
//! Mirrors the persistable fields of `IdentityManager` as a plain data
//! struct — no methods, no invariants, no live handles — so persisters
//! can round-trip it without dragging in the manager's business logic.

use indexmap::IndexMap;

use dpp::prelude::Identifier;

use crate::wallet::identity::{ManagedIdentity, WatchedIdentity};

/// Restored [`IdentityManager`](crate::wallet::identity::IdentityManager)
/// state carried in [`ClientWalletStartState`](crate::changeset::ClientWalletStartState).
#[derive(Debug, Default)]
pub struct IdentityManagerStartState {
    /// Owned identities keyed by identity ID.
    pub identities: IndexMap<Identifier, ManagedIdentity>,
    /// Watched (read-only) identities keyed by identity ID.
    pub watched_identities: IndexMap<Identifier, WatchedIdentity>,
    /// Primary identity selection, if any.
    pub primary_identity_id: Option<Identifier>,
    /// Gap-limit scan watermark (highest identity index scanned).
    pub last_scanned_index: u32,
}
