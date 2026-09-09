//! Shielded sub-wallet state restored from storage.
//!
//! Returned as part of [`ClientStartState`] by
//! [`PlatformWalletPersistence::load`] so a freshly-bound
//! [`ShieldedWallet`] can rehydrate per-subwallet decrypted notes
//! and sync watermarks without re-decrypting the chain.
//!
//! Keyed by [`SubwalletId`] so a single `BTreeMap` covers every
//! `(wallet_id, account_index)` combination on the network.
//!
//! [`ClientStartState`]: crate::changeset::ClientStartState
//! [`PlatformWalletPersistence::load`]: crate::changeset::PlatformWalletPersistence::load
//! [`ShieldedWallet`]: crate::wallet::shielded::ShieldedWallet
//! [`SubwalletId`]: crate::wallet::shielded::SubwalletId

use crate::wallet::shielded::{
    ShieldedActivityEntry, ShieldedNote, ShieldedOutgoingNote, SubwalletId,
};
use std::collections::BTreeMap;

/// Per-subwallet snapshot — every note (spent + unspent) the
/// persister has on file plus the sync watermarks.
#[derive(Debug, Default, Clone)]
pub struct ShieldedSubwalletStartState {
    /// All known notes for this subwallet, including spent ones.
    /// `is_spent` is preserved from the persisted row so the
    /// in-memory store reflects what scan-based spend detection has
    /// already established.
    pub notes: Vec<ShieldedNote>,
    /// Outgoing (sent) notes recovered via OVK on prior scans, so the
    /// in-memory store's send history survives a cold start without
    /// re-recovering every note. Idempotent on re-record by `cmx`.
    pub outgoing_notes: Vec<ShieldedOutgoingNote>,
    /// Derived activity-log entries persisted on prior sessions (live
    /// recordings + scan derivations). Rehydrated so the scan deriver's
    /// `existing_ids` set includes them — otherwise a cold-started scan
    /// would re-derive a coarse `Sent` / `ShieldedSpend` for a cluster a
    /// rich live entry already owns and overwrite it (the persister
    /// upserts by `entry.id`). Idempotent on re-save by `id`.
    pub activity: Vec<ShieldedActivityEntry>,
    /// Sync watermark: count of note positions scanned = the next
    /// global index to scan (exclusive). `0` = nothing scanned yet.
    pub last_synced_index: u64,
}

/// Whole-client shielded restore state, keyed by `SubwalletId`.
///
/// Lives on [`ClientStartState`] alongside platform-address state.
/// On wallet bind, `PlatformWallet::bind_shielded` consumes the
/// entries that match `(self.wallet_id, account)` for each
/// requested account and hands them back to the in-memory store
/// before kicking off the first sync pass.
#[derive(Debug, Default)]
pub struct ShieldedSyncStartState {
    pub per_subwallet: BTreeMap<SubwalletId, ShieldedSubwalletStartState>,
    /// Persisted per-subwallet Orchard viewing keys, as the raw
    /// 96-byte FVK encoding (`Vec<u8>` for symmetry with the
    /// changeset field). Written once at the first seed-backed
    /// `bind_shielded`; consumed by
    /// `PlatformWallet::bind_shielded_from_persisted` so a launch
    /// can rebind viewing-grade state without resolving the
    /// mnemonic. Kept separate from `per_subwallet` — a subwallet
    /// can have a viewing key but no notes yet, and vice versa
    /// (legacy rows persisted before this field existed).
    pub viewing_keys: BTreeMap<SubwalletId, Vec<u8>>,
}

impl ShieldedSyncStartState {
    /// `true` iff no subwallet snapshot is restored.
    pub fn is_empty(&self) -> bool {
        self.per_subwallet.is_empty() && self.viewing_keys.is_empty()
    }
}
