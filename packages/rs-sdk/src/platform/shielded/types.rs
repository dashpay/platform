//! Types for shielded note sync.

use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::{Note, PaymentAddress};
use rs_dapi_client::RequestSettings;

/// Default maximum number of chunk queries in flight at once.
pub(super) const DEFAULT_MAX_CONCURRENT: usize = 4;

/// Configuration for [`sync_shielded_notes`](super::sync_shielded_notes).
pub struct ShieldedSyncConfig {
    /// Maximum number of chunk queries in flight at once (default: 4).
    pub max_concurrent: usize,
    /// Request settings forwarded to each individual fetch call.
    pub request_settings: RequestSettings,
}

impl Default for ShieldedSyncConfig {
    fn default() -> Self {
        Self {
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            request_settings: RequestSettings::default(),
        }
    }
}

/// A note that was successfully decrypted (belongs to the viewer).
pub struct DecryptedNote {
    /// Global position of this note in the commitment tree.
    pub position: u64,
    /// The decrypted Orchard note (contains value, rseed, rho).
    pub note: Note,
    /// The recipient payment address.
    pub address: PaymentAddress,
    /// The nullifier (32 bytes).
    pub nullifier: [u8; 32],
    /// The note commitment (32 bytes).
    pub cmx: [u8; 32],
}

/// Result of [`sync_shielded_notes`](super::sync_shielded_notes).
pub struct ShieldedSyncResult {
    /// Notes that successfully decrypted (belong to the viewer).
    pub decrypted_notes: Vec<DecryptedNote>,
    /// All raw encrypted notes fetched, in tree order.
    /// Useful for updating a local commitment tree.
    pub all_notes: Vec<ShieldedEncryptedNote>,
    /// Next chunk-aligned index to resume syncing from.
    pub next_start_index: u64,
    /// Total number of notes scanned in this sync.
    pub total_notes_scanned: u64,
    /// Platform block height at the time of the most recent chunk response.
    pub block_height: u64,
}
