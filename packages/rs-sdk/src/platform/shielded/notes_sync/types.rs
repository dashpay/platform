//! Types for shielded note sync.

use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::{Note, PaymentAddress};
use rs_dapi_client::RequestSettings;
use std::sync::Arc;

/// Default maximum number of chunk queries in flight at once.
pub const DEFAULT_MAX_CONCURRENT: usize = 4;

/// Callback fired after each chunk completes during
/// [`sync_shielded_notes`](super::sync_shielded_notes).
///
/// Arguments: `(cumulative_notes_scanned, latest_block_height)`.
///
/// Used to surface live progress during a long initial sync (a cold
/// sync of a 1M-note pool can run >20 min in a single call; without
/// this callback there's no signal between start and end).
///
/// The callback is fired from inside the sliding-window chunk loop on
/// the same task that drives the sync — it should be cheap. If it
/// needs to do anything expensive (DB write, async work), forward via
/// a channel and process out-of-band.
pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// Configuration for [`sync_shielded_notes`](super::sync_shielded_notes).
#[derive(Clone)]
pub struct ShieldedSyncConfig {
    /// Maximum number of chunk queries in flight at once (default: 4).
    pub max_concurrent: usize,
    /// Request settings forwarded to each individual fetch call.
    pub request_settings: RequestSettings,
    /// Optional progress callback — see [`ProgressCallback`].
    /// `None` (default) disables progress reporting.
    pub on_chunk_completed: Option<ProgressCallback>,
}

impl Default for ShieldedSyncConfig {
    fn default() -> Self {
        Self {
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            request_settings: RequestSettings::default(),
            on_chunk_completed: None,
        }
    }
}

/// One chunk of encrypted notes emitted by the streaming sync entry
/// point [`sync_shielded_notes_stream`](super::sync_shielded_notes::sync_shielded_notes_stream).
///
/// Batches are yielded **in strictly ascending tree-position order**
/// (`start_index`) even when the underlying network fetches complete
/// out of order — the stream's reorder buffer holds an early-finishing
/// later chunk until every predecessor has been emitted. This lets the
/// consumer append commitments to a position-ordered Merkle tree the
/// instant a contiguous chunk is ready, overlapping tree-append with
/// later network fetches.
pub struct ShieldedChunkBatch {
    /// Chunk start position in the commitment tree (tree order). Always
    /// a multiple of the fetch size for non-final chunks.
    pub start_index: u64,
    /// Raw encrypted notes for this chunk, in tree order. The note at
    /// vec index `i` has global tree position `start_index + i`.
    pub notes: Vec<ShieldedEncryptedNote>,
    /// Driver-IVK trial-decryption hits within this chunk (the notes
    /// that decrypted under the `ivk` passed to the stream). Positions
    /// are absolute (already offset by `start_index`).
    pub decrypted: Vec<DecryptedNote>,
    /// Platform block height reported by the response that produced
    /// this chunk.
    pub block_height: u64,
    /// True for the final (buffer) chunk — a short read
    /// (`notes.len() < fetch_size`) that signals end-of-stream. The
    /// BulkAppendTree buffer chunk is mutable and may receive more
    /// notes before the next sync, so the consumer resumes from this
    /// chunk's `start_index` next pass.
    pub is_partial: bool,
    /// On-chain total number of notes in the shielded `CommitmentTree` at
    /// the block this chunk's proof was taken against — the sync
    /// progress-bar denominator. Extracted from the SAME note-fetch proof
    /// (no separate RPC), so it arrives with the very first batch and is
    /// stable across the sync; carrying it per-batch is intentional and
    /// cheap.
    pub total_count: u64,
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
    /// On-chain total number of notes in the shielded `CommitmentTree` — the
    /// sync progress-bar denominator. Extracted from the note-fetch proofs
    /// (no separate RPC); taken as the max/last-seen across the sync's
    /// batches. `0` if no chunk was fetched (e.g. an aligned start past the
    /// end of the tree returned nothing).
    pub total_count: u64,
}
