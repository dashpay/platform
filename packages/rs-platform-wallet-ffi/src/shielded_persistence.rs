//! C ABI types + callback signatures for shielded note persistence.
//!
//! Mirror of [`platform_wallet::changeset::ShieldedChangeSet`] for the
//! FFI boundary: per-subwallet decrypted notes, spent marks, sync
//! watermarks, nullifier checkpoints. Hosts implement the four
//! callbacks below in [`crate::persistence::PersistenceCallbacks`]
//! so changesets emitted by the Rust-side `ShieldedWallet` reach
//! durable storage (typically SwiftData on iOS).
//!
//! All pointers in these structs are valid for the duration of the
//! callback only — the host must copy any bytes it needs to retain
//! before the call returns.

use std::ffi::c_void;

/// One decrypted shielded note for the host to persist.
///
/// The host writes one row keyed by
/// `(wallet_id, account_index, position)`. Re-saves with the same
/// `(wallet_id, account_index, nullifier)` overwrite the existing
/// row in place — Orchard nullifiers are globally unique, so a
/// rescan after a restart shouldn't produce duplicates.
#[repr(C)]
pub struct ShieldedNoteFFI {
    /// 32-byte wallet identifier.
    pub wallet_id: [u8; 32],
    /// ZIP-32 account index.
    pub account_index: u32,
    /// Global commitment-tree position.
    pub position: u64,
    /// Note commitment (32 bytes).
    pub cmx: [u8; 32],
    /// Nullifier (32 bytes).
    pub nullifier: [u8; 32],
    /// Block height the note was first observed at.
    pub block_height: u64,
    /// `1` if this note has been observed as spent on-chain, `0`
    /// otherwise. (`bool` would still take 1 byte but `u8` is
    /// less surprising across the C ABI.)
    pub is_spent: u8,
    /// Note value in credits.
    pub value: u64,
    /// Pointer to the serialized `orchard::Note` payload.
    /// `recipient(43) || value(8 LE) || rho(32) || rseed(32)` =
    /// 115 bytes. Valid only for the callback window — the host
    /// must copy.
    pub note_data_ptr: *const u8,
    /// Length of `note_data_ptr` in bytes (always 115 for valid notes).
    pub note_data_len: usize,
}

/// One nullifier observed as spent for `(wallet_id, account_index)`.
/// The host flips the matching `is_spent` flag on the existing
/// `ShieldedNoteFFI` row.
#[repr(C)]
pub struct ShieldedNullifierSpentFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub nullifier: [u8; 32],
}

/// One outgoing (sent) note recovered via OVK for the host to persist.
///
/// Distinct from [`ShieldedNoteFFI`] (a received, spendable note):
/// this is append-only send history with no nullifier / position /
/// spend state. The host writes one row keyed by
/// `(wallet_id, account_index, cmx)`; re-persisting the same `cmx`
/// (a re-scan) is an idempotent upsert. The `recipient` is the 43-byte
/// raw Orchard address; `memo_ptr` points at the raw Dash memo bytes
/// (36 bytes), valid only for the callback window — the host must copy.
#[repr(C)]
pub struct ShieldedOutgoingNoteFFI {
    /// 32-byte wallet identifier.
    pub wallet_id: [u8; 32],
    /// ZIP-32 account index.
    pub account_index: u32,
    /// Note commitment (cmx) of the sent note (32 bytes). Primary key.
    pub cmx: [u8; 32],
    /// Recipient's raw Orchard address (43 bytes).
    pub recipient: [u8; 43],
    /// Value sent, in credits.
    pub value: u64,
    /// Block height the sent note appeared at.
    pub block_height: u64,
    /// Pointer to the raw Dash memo bytes (36 bytes). Valid only for
    /// the callback window — the host must copy.
    pub memo_ptr: *const u8,
    /// Length of `memo_ptr` in bytes (always 36 for a recovered note).
    pub memo_len: usize,
}

/// One per-subwallet sync-watermark advance.
#[repr(C)]
pub struct ShieldedSyncedIndexFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    /// Sync watermark: count of note positions scanned = the next
    /// global commitment-tree index to scan (exclusive). `0` = nothing
    /// scanned yet.
    pub last_synced_index: u64,
}

/// One derived shielded-activity entry for the host to persist.
///
/// Mirror of `platform_wallet::wallet::shielded::ShieldedActivityEntry`.
/// The host writes one row keyed by `(wallet_id, account_index,
/// entry_id)` — NOT `entry_id` alone: the same entry id (sha256 of the
/// visible output cmxs) legitimately appears under two accounts of one
/// wallet, e.g. an intra-wallet transfer producing a Sent row on the
/// sending account and a Received row on the receiving account.
/// Re-persisting the same tuple is an upsert that refines the row
/// (Pending→Confirmed/Failed, or a scan-derived `ShieldedSpend`
/// upgraded to a richer kind). All pointers are valid only for the
/// callback window — the host must copy.
///
/// `Option<T>` fields are flattened to a value + a `has_*` flag (`u8`,
/// 1 = present) rather than a sentinel, so `0`/empty is unambiguous.
#[repr(C)]
pub struct ShieldedActivityFFI {
    /// 32-byte wallet identifier.
    pub wallet_id: [u8; 32],
    /// ZIP-32 account index.
    pub account_index: u32,
    /// Entry id (sha256 of sorted visible output cmxs). Unique only
    /// within `(wallet_id, account_index)` — see the struct doc.
    pub entry_id: [u8; 32],
    /// Kind discriminant (see `ShieldedActivityKind::tag`):
    /// 0 Shield, 1 ShieldFromAssetLock, 2 Received, 3 Sent, 4 Unshield,
    /// 5 Withdrawal, 6 IdentityCreate, 7 ShieldedSpend.
    pub kind_tag: u8,
    /// Direction: 0 In, 1 Out, 2 Self.
    pub direction: u8,
    /// Status: 0 Pending, 1 Confirmed, 2 Failed.
    pub status: u8,
    /// Display amount in credits (principal; excludes self-change and
    /// zero-value fillers).
    pub amount: u64,
    /// Exact fee in credits when `has_fee == 1`.
    pub fee: u64,
    /// `1` if `fee` is meaningful, `0` if the fee is unknown.
    pub has_fee: u8,
    /// Block height when `has_block_height == 1`.
    pub block_height: u64,
    /// `1` if `block_height` is meaningful (confirmed), `0` while pending.
    pub has_block_height: u8,
    /// Created-at time in ms since the Unix epoch (display-only;
    /// `block_height` is the canonical sort key). `0` = unknown —
    /// scan-derived (restored) entries carry no wall-clock provenance.
    pub created_at_ms: u64,
    /// Chain-order key when `has_min_note_position == 1`: the smallest
    /// commitment-tree position among the entry's own received notes.
    /// Tree positions are exact append-only chain order — hosts use
    /// this to order otherwise-undatable restored entries. Set by the
    /// scan deriver; live entries (which carry a real `created_at_ms`)
    /// and outgoing-only clusters report `0`/`0`.
    pub min_note_position: u64,
    /// `1` if `min_note_position` is meaningful.
    pub has_min_note_position: u8,
    /// Created identity id (only meaningful when `kind_tag == 6` /
    /// IdentityCreate); all-zero and ignored otherwise.
    pub identity_id: [u8; 32],
    /// `1` when `identity_id` is meaningful (IdentityCreate), else `0`.
    pub has_identity_id: u8,
    /// Counterparty bytes pointer (43B Orchard / 21B PlatformAddress /
    /// Core script) or null. Valid for the callback window only.
    pub counterparty_ptr: *const u8,
    /// Length of `counterparty_ptr` in bytes (0 when null).
    pub counterparty_len: usize,
    /// 36-byte memo pointer or null. Valid for the callback window only.
    pub memo_ptr: *const u8,
    /// Length of `memo_ptr` in bytes (0 when null).
    pub memo_len: usize,
    /// Pointer to the concatenated visible-output cmxs (`note_cmxs_count`
    /// × 32 bytes). Valid for the callback window only.
    pub note_cmxs_ptr: *const u8,
    /// Number of 32-byte cmxs at `note_cmxs_ptr`.
    pub note_cmxs_count: usize,
    /// Pointer to the concatenated spent nullifiers (`spent_nullifiers_count`
    /// × 32 bytes). Valid for the callback window only.
    pub spent_nullifiers_ptr: *const u8,
    /// Number of 32-byte nullifiers at `spent_nullifiers_ptr`.
    pub spent_nullifiers_count: usize,
}

/// One per-subwallet Orchard viewing key for the host to persist.
///
/// The 96 bytes are the raw `FullViewingKey` encoding (`ak ‖ nk ‖
/// rivk`); IVK / OVK / default address are all pure functions of it,
/// so this row alone lets a later launch rebind the shielded
/// sub-wallet without resolving the mnemonic. Viewing-grade only —
/// it can decrypt and recognize notes but cannot authorize a spend.
/// The host upserts one row keyed by `(wallet_id, account_index)`;
/// the FVK for a subwallet never legitimately changes on a network,
/// so a re-emit is byte-identical.
#[repr(C)]
pub struct ShieldedViewingKeyFFI {
    /// 32-byte wallet identifier.
    pub wallet_id: [u8; 32],
    /// ZIP-32 account index.
    pub account_index: u32,
    /// Raw 96-byte Orchard `FullViewingKey` encoding.
    pub fvk_bytes: [u8; 96],
}

// ── Restore (load) ──────────────────────────────────────────────────────

/// One persisted note as the host hands it back at boot. Mirrors
/// [`ShieldedNoteFFI`] but lives in a Swift-allocated array, so
/// the buffer ownership / free contract differs (see
/// [`OnLoadShieldedNotesFreeFn`]).
#[repr(C)]
pub struct ShieldedNoteRestoreFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub position: u64,
    pub cmx: [u8; 32],
    pub nullifier: [u8; 32],
    pub block_height: u64,
    pub is_spent: u8,
    pub value: u64,
    pub note_data_ptr: *const u8,
    pub note_data_len: usize,
}

/// One persisted outgoing (sent) note as the host hands it back at
/// boot. Mirrors [`ShieldedOutgoingNoteFFI`] but lives in a
/// Swift-allocated array, so the buffer ownership / free contract
/// differs (see the matching `on_load_shielded_outgoing_notes_free_fn`).
#[repr(C)]
pub struct ShieldedOutgoingNoteRestoreFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub cmx: [u8; 32],
    pub recipient: [u8; 43],
    pub value: u64,
    pub block_height: u64,
    pub memo_ptr: *const u8,
    pub memo_len: usize,
}

/// One per-subwallet sync-watermark snapshot. Restored alongside
/// notes so the rehydrated `SubwalletState` resumes incremental
/// sync from the right place.
#[repr(C)]
pub struct ShieldedSubwalletSyncStateFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub last_synced_index: u64,
}

/// One persisted Orchard viewing key as the host hands it back at
/// boot. Mirrors [`ShieldedViewingKeyFFI`] but lives in a
/// Swift-allocated array, so the buffer ownership / free contract
/// differs (see the matching `on_load_shielded_viewing_keys_free_fn`).
#[repr(C)]
pub struct ShieldedViewingKeyRestoreFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub fvk_bytes: [u8; 96],
}

/// One persisted activity entry as the host hands it back at boot.
/// Mirrors [`ShieldedActivityFFI`] but lives in a Swift-allocated array,
/// so the buffer ownership / free contract differs (see the matching
/// `on_load_shielded_activity_free_fn`). Field semantics are identical
/// to [`ShieldedActivityFFI`].
#[repr(C)]
pub struct ShieldedActivityRestoreFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub entry_id: [u8; 32],
    pub kind_tag: u8,
    pub direction: u8,
    pub status: u8,
    pub amount: u64,
    pub fee: u64,
    pub has_fee: u8,
    pub block_height: u64,
    pub has_block_height: u8,
    pub created_at_ms: u64,
    pub min_note_position: u64,
    pub has_min_note_position: u8,
    pub identity_id: [u8; 32],
    pub has_identity_id: u8,
    pub counterparty_ptr: *const u8,
    pub counterparty_len: usize,
    pub memo_ptr: *const u8,
    pub memo_len: usize,
    pub note_cmxs_ptr: *const u8,
    pub note_cmxs_count: usize,
    pub spent_nullifiers_ptr: *const u8,
    pub spent_nullifiers_count: usize,
}

// The `on_load_shielded_*_fn` callback types are inlined inside
// [`PersistenceCallbacks`] (rather than declared as `pub type`
// aliases here) so cbindgen sees the full signature, walks into
// the referenced structs, and emits their full field layout in
// the generated header. Bare `pub type X = unsafe extern "C" fn`
// aliases are mangled into opaque structs by cbindgen and don't
// drag in their function-pointer arguments.

#[allow(dead_code)]
fn _keep_c_void_in_scope(_x: *const c_void) {}
