use crate::platform::Fetch;
use crate::{Error, Sdk};
use drive_proof_verifier::types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery,
};
use rs_dapi_client::RequestSettings;
use tracing::{debug, warn};

/// Fetch a single chunk of encrypted notes from the network.
///
/// Returns `(chunk_start_index, notes, block_height, total_count)`. An empty
/// vec means no notes exist at this position (past end of tree).
///
/// `total_count` is the on-chain total number of notes in the shielded
/// `CommitmentTree`, extracted from the SAME note-fetch proof (the parent
/// CommitmentTree element is always present in it). It is stable across a
/// sync and arrives on every chunk — including the empty final chunk — so a
/// wallet gets the sync progress-bar denominator for free with no separate
/// `GetShieldedNotesCount` RPC.
pub async fn fetch_chunk(
    sdk: &Sdk,
    chunk_start: u64,
    chunk_size: u64,
    settings: RequestSettings,
) -> Result<(u64, Vec<ShieldedEncryptedNote>, u64, u64), Error> {
    let query = ShieldedEncryptedNotesQuery {
        start_index: chunk_start,
        count: chunk_size as u32,
    };

    debug!(chunk_start, chunk_size, "fetching shielded notes chunk");

    // `Instant` is unavailable on wasm32; a chunk fetched there logs
    // `elapsed_ms=None` rather than a fabricated duration.
    #[cfg(not(target_arch = "wasm32"))]
    let started = std::time::Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let elapsed_ms = move || Some(started.elapsed().as_millis() as u64);
    #[cfg(target_arch = "wasm32")]
    let elapsed_ms = || None::<u64>;

    let fetched = ShieldedEncryptedNotes::fetch_with_metadata(sdk, query, Some(settings)).await;

    let (result, metadata) = match fetched {
        Ok(v) => v,
        Err(e) => {
            warn!(
                chunk_start,
                elapsed_ms = ?elapsed_ms(),
                error = %e,
                "shielded notes chunk fetch failed"
            );
            return Err(e);
        }
    };

    let (notes, total_count) = match result {
        Some(ShieldedEncryptedNotes { notes, total_count }) => (notes, total_count),
        None => (Vec::new(), 0),
    };

    debug!(
        chunk_start,
        notes_returned = notes.len(),
        block_height = metadata.height,
        total_count,
        elapsed_ms = ?elapsed_ms(),
        "shielded notes chunk fetched"
    );

    Ok((chunk_start, notes, metadata.height, total_count))
}
