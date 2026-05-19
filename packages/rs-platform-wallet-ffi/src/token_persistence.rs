//! FFI types for forwarding
//! [`TokenBalanceChangeSet`](platform_wallet::changeset::TokenBalanceChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! Mirrors the shape of the `(identity_id, token_id) -> balance`
//! changeset emitted by
//! [`IdentitySyncManager::sync_now`](platform_wallet::IdentitySyncManager).
//! Swift maps each upsert onto a `PersistentTokenBalance` row keyed
//! by `(tokenId, identityId)` and drops rows for every removal.
//!
//! The watch list itself is not part of this projection — the
//! per-identity registry lives in the manager's in-memory cache and
//! is rehydrated on app start from whatever the Swift side passes to
//! `platform_wallet_manager_identity_sync_register_identity` /
//! `_update_watched_tokens`. Persisted balance rows are the only
//! durable record carried across launches.

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
