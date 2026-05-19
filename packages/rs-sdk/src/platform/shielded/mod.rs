//! Shielded note sync and trial decryption utilities.
//!
//! This module provides:
//! - [`try_decrypt_note`]: compact trial decryption on a single encrypted note
//! - [`sync_shielded_notes`]: end-to-end sync that fetches encrypted notes from
//!   the network in parallel and performs trial decryption

pub mod notes_sync;
pub mod nullifier_sync;

pub use notes_sync::decrypt::try_decrypt_note;
pub use notes_sync::sync_shielded_notes::sync_shielded_notes;
pub use notes_sync::types::{DecryptedNote, ShieldedSyncConfig, ShieldedSyncResult};
