use super::decrypt::try_decrypt_note;
use super::fetch_chunk::fetch_chunk as do_fetch_chunk;
use super::types::{DecryptedNote, ShieldedSyncConfig, ShieldedSyncResult};
use crate::{Error, Sdk};
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::stream::{FuturesUnordered, StreamExt};
use grovedb_commitment_tree::PreparedIncomingViewingKey;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use tracing::debug;

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
    if chunk_size > 0 && !start_index.is_multiple_of(chunk_size) {
        return Err(Error::Generic(format!(
            "start_index {} is not chunk-aligned; must be a multiple of {}",
            start_index, chunk_size
        )));
    }

    let max_concurrent = config.max_concurrent.max(1);
    let settings = config.request_settings;

    type ChunkFuture =
        Pin<Box<dyn Future<Output = Result<(u64, Vec<ShieldedEncryptedNote>, u64), Error>> + Send>>;

    // Sliding-window parallel fetch using FuturesUnordered.
    // Each future fetches one chunk and returns (chunk_start_index, notes, block_height).
    let mut futures: FuturesUnordered<ChunkFuture> = FuturesUnordered::new();
    let mut next_chunk_index = start_index;
    let mut reached_end = false;

    // Seed initial batch of chunk queries
    for _ in 0..max_concurrent {
        let chunk_idx = next_chunk_index;
        next_chunk_index += chunk_size;
        let sdk = sdk.clone();
        futures.push(Box::pin(async move {
            do_fetch_chunk(&sdk, chunk_idx, chunk_size, settings).await
        }));
    }

    // Collect results keyed by chunk start_index for ordered reassembly
    let mut chunk_results: BTreeMap<u64, Vec<ShieldedEncryptedNote>> = BTreeMap::new();
    let mut max_block_height: u64 = 0;

    while let Some(result) = futures.next().await {
        let (chunk_idx, notes, block_height) = result?;
        let is_partial = (notes.len() as u64) < chunk_size;
        chunk_results.insert(chunk_idx, notes);
        max_block_height = max_block_height.max(block_height);

        if is_partial {
            reached_end = true;
        }

        // Queue the next chunk if we haven't reached the end
        if !reached_end {
            let chunk_idx = next_chunk_index;
            next_chunk_index += chunk_size;
            let sdk = sdk.clone();
            futures.push(Box::pin(async move {
                do_fetch_chunk(&sdk, chunk_idx, chunk_size, settings).await
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

    // Determine next_start_index before consuming chunk_results.
    // If the last non-empty chunk is partial (fewer notes than chunk_size),
    // resume from that chunk's start -- the BulkAppendTree buffer chunk is
    // mutable and may receive more notes before the next sync.
    let next_start_index = if chunk_size > 0 {
        let last_partial = chunk_results
            .iter()
            .rev()
            .find(|(_, notes)| !notes.is_empty())
            .filter(|(_, notes)| (notes.len() as u64) < chunk_size);

        match last_partial {
            Some((&chunk_start, _)) => chunk_start,
            None => start_index + total_notes_scanned,
        }
    } else {
        start_index + total_notes_scanned
    };

    // Move notes out of the BTreeMap in order
    for (_, notes) in chunk_results {
        all_notes.extend(notes);
    }

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
        block_height: max_block_height,
    })
}