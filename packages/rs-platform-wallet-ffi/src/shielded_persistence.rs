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

/// One per-subwallet sync-watermark advance.
#[repr(C)]
pub struct ShieldedSyncedIndexFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    /// Highest global commitment-tree index the subwallet has scanned.
    pub last_synced_index: u64,
}

/// One per-subwallet nullifier-sync checkpoint.
#[repr(C)]
pub struct ShieldedNullifierCheckpointFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    /// Block height of the most recent nullifier sync pass.
    pub height: u64,
    /// Block timestamp (Unix seconds) of the most recent pass.
    pub timestamp: u64,
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

/// One per-subwallet sync-watermark + nullifier-checkpoint snapshot.
/// Restored alongside notes so the rehydrated `SubwalletState`
/// resumes incremental sync from the right place.
#[repr(C)]
pub struct ShieldedSubwalletSyncStateFFI {
    pub wallet_id: [u8; 32],
    pub account_index: u32,
    pub last_synced_index: u64,
    /// `1` iff the optional `nullifier_checkpoint` is populated.
    pub has_nullifier_checkpoint: u8,
    pub nullifier_checkpoint_height: u64,
    pub nullifier_checkpoint_timestamp: u64,
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
