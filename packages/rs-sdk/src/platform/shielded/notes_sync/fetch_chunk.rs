use crate::platform::Fetch;
use crate::{Error, Sdk};
use drive_proof_verifier::types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery,
};
use rs_dapi_client::RequestSettings;
use tracing::debug;

/// Fetch a single chunk of encrypted notes from the network.
///
/// Returns `(chunk_start_index, notes, block_height)`. An empty vec means no
/// notes exist at this position (past end of tree).
pub async fn fetch_chunk(
    sdk: &Sdk,
    chunk_start: u64,
    chunk_size: u64,
    settings: RequestSettings,
) -> Result<(u64, Vec<ShieldedEncryptedNote>, u64), Error> {
    let query = ShieldedEncryptedNotesQuery {
        start_index: chunk_start,
        count: chunk_size as u32,
    };

    debug!(chunk_start, chunk_size, "fetching shielded notes chunk");

    let (result, metadata) =
        ShieldedEncryptedNotes::fetch_with_metadata(sdk, query, Some(settings)).await?;

    let notes = match result {
        Some(ShieldedEncryptedNotes(notes)) => notes,
        None => Vec::new(),
    };

    debug!(
        chunk_start,
        notes_returned = notes.len(),
        block_height = metadata.height,
        "shielded notes chunk fetched"
    );

    Ok((chunk_start, notes, metadata.height))
}
