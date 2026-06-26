//! Shielded note sync and trial decryption utilities.
//!
//! This module provides:
//! - [`try_decrypt_note`]: compact trial decryption on a single encrypted note
//!   (incoming, under the IVK)
//! - [`try_decrypt_note_with_memo`]: full incoming trial decryption that also
//!   recovers the note's memo (under the IVK)
//! - [`try_recover_outgoing_note`]: OVK recovery of a sent note (recipient,
//!   value, memo) under the wallet's Outgoing Viewing Key
//! - [`sync_shielded_notes`]: end-to-end sync that fetches encrypted notes from
//!   the network in parallel and performs trial decryption

pub mod notes_sync;

pub use notes_sync::decrypt::{
    try_decrypt_note, try_decrypt_note_with_memo, try_recover_outgoing_note, DASH_MEMO_SIZE,
};
pub use notes_sync::sync_shielded_notes::{sync_shielded_notes, sync_shielded_notes_stream};
pub use notes_sync::types::{
    DecryptedNote, ShieldedChunkBatch, ShieldedSyncConfig, ShieldedSyncResult,
};
