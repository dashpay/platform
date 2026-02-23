//! Shielded note sync and trial decryption utilities.
//!
//! This module provides:
//! - [`try_decrypt_note`]: compact trial decryption on a single encrypted note
//! - [`sync_shielded_notes`]: end-to-end sync that fetches encrypted notes from
//!   the network in parallel and performs trial decryption

use crate::error::Error;
use crate::platform::Fetch;
use crate::Sdk;
use drive_proof_verifier::types::ShieldedEncryptedNotesQuery;
use drive_proof_verifier::types::{ShieldedEncryptedNote, ShieldedEncryptedNotes};
use futures::stream::{FuturesUnordered, StreamExt};
use grovedb_commitment_tree::{
    try_compact_note_decryption, CompactAction, DashMemo, EphemeralKeyBytes,
    ExtractedNoteCommitment, Note, Nullifier, OrchardDomain, PaymentAddress,
    PreparedIncomingViewingKey, COMPACT_NOTE_SIZE,
};
use rs_dapi_client::RequestSettings;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use tracing::debug;

/// Minimum length of the `encrypted_note` field for compact trial decryption.
///
/// The `encrypted_note` field layout is:
///   `epk(32) || enc_ciphertext(104) || out_ciphertext(80)` = 216 bytes
///
/// For compact decryption we need at least `epk(32) + COMPACT_NOTE_SIZE` bytes
/// of the enc_ciphertext.
const MIN_ENCRYPTED_NOTE_LEN: usize = 32 + COMPACT_NOTE_SIZE;

/// Default maximum number of chunk queries in flight at once.
const DEFAULT_MAX_CONCURRENT: usize = 4;

// ---------------------------------------------------------------------------
// Trial decryption
// ---------------------------------------------------------------------------

/// Attempt compact trial decryption on a [`ShieldedEncryptedNote`].
///
/// The [`ShieldedEncryptedNote`] struct (from proof verification) has three
/// separate fields:
///   - `cmx`: note commitment (32 bytes)
///   - `nullifier`: nullifier (32 bytes) — used for Rho derivation
///   - `encrypted_note`: `epk(32) || enc_ciphertext(104) || out_ciphertext(80)`
///
/// Returns `Some((note, address))` if the note decrypts successfully under the
/// given incoming viewing key, or `None` if it does not belong to the viewer
/// (including dummy/padding notes).
pub fn try_decrypt_note(
    ivk: &PreparedIncomingViewingKey,
    encrypted_note: &ShieldedEncryptedNote,
) -> Option<(Note, PaymentAddress)> {
    let data = &encrypted_note.encrypted_note;
    if data.len() < MIN_ENCRYPTED_NOTE_LEN {
        return None;
    }

    // Parse nullifier from the dedicated field (32 bytes)
    let nf_bytes: [u8; 32] = encrypted_note.nullifier.as_slice().try_into().ok()?;
    let nf = Nullifier::from_bytes(&nf_bytes).into_option()?;

    // Parse cmx from the dedicated field (32 bytes)
    let cmx_bytes: [u8; 32] = encrypted_note.cmx.as_slice().try_into().ok()?;
    let cmx = ExtractedNoteCommitment::from_bytes(&cmx_bytes).into_option()?;

    // Parse ephemeral public key (first 32 bytes of encrypted_note)
    let epk_bytes: [u8; 32] = data[0..32].try_into().ok()?;

    // Parse compact ciphertext (first COMPACT_NOTE_SIZE bytes of enc_ciphertext,
    // starting at byte 32)
    let enc_compact: [u8; COMPACT_NOTE_SIZE] = data[32..32 + COMPACT_NOTE_SIZE].try_into().ok()?;

    // Build CompactAction and OrchardDomain for trial decryption
    let compact = CompactAction::from_parts(nf, cmx, EphemeralKeyBytes(epk_bytes), enc_compact);
    let domain = OrchardDomain::<DashMemo>::for_compact_action(&compact);

    try_compact_note_decryption(&domain, ivk, &compact)
}

// ---------------------------------------------------------------------------
// Shielded note sync
// ---------------------------------------------------------------------------

/// Configuration for [`sync_shielded_notes`].
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

/// Result of [`sync_shielded_notes`].
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
}

/// Fetch all shielded encrypted notes starting from `start_index`, query
/// multiple nodes in parallel, and perform trial decryption.
///
/// This is the main entry point for wallet sync. It handles:
/// 1. Chunk-aligned pagination (each query covers one BulkAppendTree chunk)
/// 2. Parallel dispatch of chunk queries across network nodes
/// 3. Proof verification on every response
/// 4. Trial decryption with the provided incoming viewing key
///
/// # Arguments
///
/// - `sdk` — SDK instance connected to the network
/// - `ivk` — prepared incoming viewing key for trial decryption
/// - `start_index` — first note position to fetch (must be a multiple of
///   the chunk size, typically 2048)
/// - `config` — optional tuning; `None` uses sensible defaults
///
/// # Returns
///
/// [`ShieldedSyncResult`] containing decrypted notes that belong to the
/// viewer, all raw notes for commitment tree updates, and the next index
/// to resume from.
pub async fn sync_shielded_notes(
    sdk: &Sdk,
    ivk: &PreparedIncomingViewingKey,
    start_index: u64,
    config: Option<ShieldedSyncConfig>,
) -> Result<ShieldedSyncResult, Error> {
    let config = config.unwrap_or_default();

    let chunk_size = sdk
        .version()
        .drive_abci
        .query
        .shielded_queries
        .max_encrypted_notes_per_query as u64;

    // Validate alignment
    if chunk_size > 0 && start_index % chunk_size != 0 {
        return Err(Error::Generic(format!(
            "start_index {} is not chunk-aligned; must be a multiple of {}",
            start_index, chunk_size
        )));
    }

    let max_concurrent = config.max_concurrent.max(1);
    let settings = config.request_settings;

    type ChunkFuture =
        Pin<Box<dyn Future<Output = Result<(u64, Vec<ShieldedEncryptedNote>), Error>> + Send>>;

    // Sliding-window parallel fetch using FuturesUnordered.
    // Each future fetches one chunk and returns (chunk_start_index, notes).
    let mut futures: FuturesUnordered<ChunkFuture> = FuturesUnordered::new();
    let mut next_chunk_index = start_index;
    let mut reached_end = false;

    // Seed initial batch of chunk queries
    for _ in 0..max_concurrent {
        let chunk_idx = next_chunk_index;
        next_chunk_index += chunk_size;
        let sdk = sdk.clone();
        futures.push(Box::pin(async move {
            fetch_chunk(&sdk, chunk_idx, chunk_size, settings).await
        }));
    }

    // Collect results keyed by chunk start_index for ordered reassembly
    let mut chunk_results: BTreeMap<u64, Vec<ShieldedEncryptedNote>> = BTreeMap::new();

    while let Some(result) = futures.next().await {
        let (chunk_idx, notes) = result?;
        let is_partial = (notes.len() as u64) < chunk_size;
        chunk_results.insert(chunk_idx, notes);

        if is_partial {
            reached_end = true;
        }

        // Queue the next chunk if we haven't reached the end
        if !reached_end {
            let chunk_idx = next_chunk_index;
            next_chunk_index += chunk_size;
            let sdk = sdk.clone();
            futures.push(Box::pin(async move {
                fetch_chunk(&sdk, chunk_idx, chunk_size, settings).await
            }));
        }
    }

    // Flatten in tree order and perform trial decryption
    let mut all_notes = Vec::new();
    let mut decrypted_notes = Vec::new();

    for (&chunk_start, notes) in &chunk_results {
        for (i, note) in notes.iter().enumerate() {
            let position = chunk_start + i as u64;

            if let Some((decrypted, address)) = try_decrypt_note(ivk, note) {
                let nf: [u8; 32] = note.nullifier.as_slice().try_into().unwrap_or([0u8; 32]);
                let cmx: [u8; 32] = note.cmx.as_slice().try_into().unwrap_or([0u8; 32]);

                decrypted_notes.push(DecryptedNote {
                    position,
                    note: decrypted,
                    address,
                    nullifier: nf,
                    cmx,
                });
            }
        }
    }

    let total_notes_scanned: u64 = chunk_results.values().map(|v| v.len() as u64).sum();

    // Move notes out of the BTreeMap in order
    for (_, notes) in chunk_results {
        all_notes.extend(notes);
    }

    // Next start index: round up to next chunk boundary
    let raw_next = start_index + total_notes_scanned;
    let next_start_index = if chunk_size > 0 {
        ((raw_next + chunk_size - 1) / chunk_size) * chunk_size
    } else {
        raw_next
    };

    debug!(
        total_notes_scanned,
        decrypted_count = decrypted_notes.len(),
        next_start_index,
        "shielded note sync complete"
    );

    Ok(ShieldedSyncResult {
        decrypted_notes,
        all_notes,
        next_start_index,
        total_notes_scanned,
    })
}

/// Fetch a single chunk of encrypted notes from the network.
///
/// Returns `(chunk_start_index, notes)`. An empty vec means no notes exist
/// at this position (past end of tree).
async fn fetch_chunk(
    sdk: &Sdk,
    chunk_start: u64,
    chunk_size: u64,
    settings: RequestSettings,
) -> Result<(u64, Vec<ShieldedEncryptedNote>), Error> {
    let query = ShieldedEncryptedNotesQuery {
        start_index: chunk_start,
        count: chunk_size as u32,
    };

    debug!(chunk_start, chunk_size, "fetching shielded notes chunk");

    let result = ShieldedEncryptedNotes::fetch_with_settings(sdk, query, settings).await?;

    let notes = match result {
        Some(ShieldedEncryptedNotes(notes)) => notes,
        None => Vec::new(),
    };

    debug!(
        chunk_start,
        notes_returned = notes.len(),
        "shielded notes chunk fetched"
    );

    Ok((chunk_start, notes))
}
