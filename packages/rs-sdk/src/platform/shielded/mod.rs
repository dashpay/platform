//! Shielded note sync and trial decryption utilities.
//!
//! This module provides:
//! - [`try_decrypt_note`]: compact trial decryption on a single encrypted note
//! - [`sync_shielded_notes`]: end-to-end sync that fetches encrypted notes from
//!   the network in parallel and performs trial decryption

mod decrypt;
pub mod nullifier_sync;
mod sync;
mod types;

pub use decrypt::try_decrypt_note;
pub use sync::sync_shielded_notes;
pub use types::{DecryptedNote, ShieldedSyncConfig, ShieldedSyncResult};
