//! FFI types for forwarding
//! [`TokenBalanceChangeSet`](platform_wallet::changeset::TokenBalanceChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! Mirrors the shape of the (`identity_id`, `token_id`) keyed
//! `BTreeMap<(Identifier, Identifier), TokenAmount>` on
//! `PlatformWalletInfo.token_balances`. Swift maps each upsert onto a
//! `PersistentTokenBalance` row keyed by `(tokenId, identityId)` and
//! drops rows for every removal.
//!
//! The `watched` / `unwatched` portions of the changeset are
//! intentionally not surfaced over FFI today: there is no Swift-side
//! schema for the watch registry — the sync driver re-watches on every
//! tick from the union of (local identities x known tokens), so the
//! registry is reconstructed in-memory from that source of truth
//! rather than persisted independently.

/// Flat C mirror of one `(identity_id, token_id) -> balance` row from
/// `TokenBalanceChangeSet.balances`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TokenBalanceUpsertFFI {
    pub identity_id: [u8; 32],
    pub token_id: [u8; 32],
    pub balance: u64,
}

/// Flat C mirror of one `(identity_id, token_id)` tombstone from
/// `TokenBalanceChangeSet.removed_balances`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TokenBalanceRemovalFFI {
    pub identity_id: [u8; 32],
    pub token_id: [u8; 32],
}
