use super::decrypt::try_decrypt_note;
use super::fetch_chunk::fetch_chunk as do_fetch_chunk;
use super::types::{DecryptedNote, ShieldedChunkBatch, ShieldedSyncConfig, ShieldedSyncResult};
use crate::{Error, Sdk};
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::stream::{FuturesUnordered, Stream, StreamExt};
use grovedb_commitment_tree::PreparedIncomingViewingKey;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use tracing::debug;

/// Resolve the on-chain MMR chunk size and the per-request fetch size.
///
/// `mmr_chunk_size` is the unit that `start_index` must align to.
/// `fetch_size` is how many notes we pull per request: under
/// `max_query_chunks` the server packs that many MMR chunks into one
/// proof, so each request advances by `max_query_chunks ×` the MMR
/// chunk size. Decoupling the two lets the SDK opportunistically
/// request larger spans without touching the on-chain tree shape.
fn resolve_sizes(sdk: &Sdk) -> (u64, u64) {
    let mmr_chunk_size: u64 = 1u64 << drive::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER;
    let max_query_chunks = sdk
        .version()
        .drive_abci
        .query
        .shielded_queries
        .max_query_chunks as u64;
    let fetch_size = mmr_chunk_size
        .saturating_mul(max_query_chunks)
        .max(mmr_chunk_size);
    (mmr_chunk_size, fetch_size)
}

type ChunkFuture = Pin<
    Box<dyn Future<Output = Result<(u64, Vec<ShieldedEncryptedNote>, u64, u64), Error>> + Send>,
>;

/// Pure, network-free reorder buffer + emit watermark.
///
/// Chunk fetches complete out of order under the sliding-window
/// `FuturesUnordered`, but tree-position consumers require strictly
/// ascending `start_index`. This buffer holds early-finishing later
/// chunks until every predecessor has been emitted. The watermark
/// advances by `chunk_size` on each successful pop, so a chunk at
/// `start_index = watermark` can only emit once its predecessor at
/// `watermark - chunk_size` already has.
///
/// Generic over the buffered payload `T` so the ordering logic can be
/// unit-tested in isolation (no `Sdk`, no trial-decryption).
struct ReorderBuffer<T> {
    chunk_size: u64,
    /// Completed chunks waiting on a predecessor, keyed by start index.
    buffered: BTreeMap<u64, T>,
    /// Next start index allowed to emit (tree order).
    watermark: u64,
}

impl<T> ReorderBuffer<T> {
    fn new(start_index: u64, chunk_size: u64) -> Self {
        Self {
            chunk_size,
            buffered: BTreeMap::new(),
            watermark: start_index,
        }
    }

    /// Buffer a completed chunk by its start index.
    fn insert(&mut self, chunk_idx: u64, payload: T) {
        self.buffered.insert(chunk_idx, payload);
    }

    /// If the chunk at the current watermark is buffered, remove it,
    /// advance the watermark by `chunk_size`, and return
    /// `(start_index, payload)`. Otherwise `None`.
    fn pop_ready(&mut self) -> Option<(u64, T)> {
        let start_index = self.watermark;
        let payload = self.buffered.remove(&start_index)?;
        self.watermark += self.chunk_size;
        Some((start_index, payload))
    }
}

/// Mutable driver state for the streaming chunk pipeline.
///
/// Owned by the [`futures::stream::unfold`] closure inside
/// [`sync_shielded_notes_stream`]. Because the stream is pull-based,
/// the `FuturesUnordered` only advances when the consumer polls for the
/// next batch — that is the backpressure: a slow consumer (e.g. a
/// wallet tree-append) simply doesn't poll, so no further chunk fetch
/// is queued and in-flight network requests stay capped at
/// `max_concurrent`. The reorder buffer is bounded by the same window.
struct StreamState {
    sdk: Sdk,
    ivk: PreparedIncomingViewingKey,
    chunk_size: u64,
    /// The sync's absolute start position. Added to `cumulative_scanned`
    /// when firing the download progress callback so the "Downloaded"
    /// bar is absolute-toward-total — it shares a baseline with the
    /// wallet's "Checked" (= tree leaf count) signal, keeping
    /// Checked <= Downloaded even on a resume where the tree is full.
    start_index: u64,
    settings: rs_dapi_client::RequestSettings,
    on_progress: Option<super::types::ProgressCallback>,
    /// In-flight chunk fetches (sliding window of `max_concurrent`).
    futures: FuturesUnordered<ChunkFuture>,
    /// Next chunk start index to dispatch a fetch for.
    next_chunk_index: u64,
    /// Out-of-order completed chunks waiting for their predecessor to be
    /// emitted, plus the emit watermark. Drained in ascending order.
    /// Payload: `(notes, block_height)`. The progress denominator is NOT
    /// buffered per chunk — the emitted batch carries `self.total_count`
    /// (the monotonic max-seen) so out-of-order completion can never make
    /// the determinate progress bar regress mid-sync.
    reorder: ReorderBuffer<(Vec<ShieldedEncryptedNote>, u64)>,
    /// Set once a partial (short) chunk is observed — stops queuing new
    /// fetches.
    reached_end: bool,
    /// Cumulative notes seen across every completed chunk — fed into the
    /// "downloaded" progress callback.
    cumulative_scanned: u64,
    /// Max block height across every completed chunk so far.
    max_block_height: u64,
    /// On-chain total note count extracted from the note-fetch proofs —
    /// max/last-seen across completed chunks. Carried into every emitted
    /// batch as the sync progress-bar denominator.
    total_count: u64,
}

impl StreamState {
    /// Queue one more chunk fetch if we haven't hit end-of-stream.
    fn queue_next(&mut self) {
        if self.reached_end {
            return;
        }
        let chunk_idx = self.next_chunk_index;
        self.next_chunk_index += self.chunk_size;
        let sdk = self.sdk.clone();
        let chunk_size = self.chunk_size;
        let settings = self.settings;
        self.futures.push(Box::pin(async move {
            do_fetch_chunk(&sdk, chunk_idx, chunk_size, settings).await
        }));
    }

    /// If the chunk at the emit watermark is buffered, build its batch,
    /// advance the watermark, and return it. Otherwise `None`.
    fn pop_ready(&mut self) -> Option<ShieldedChunkBatch> {
        let (start_index, (notes, block_height)) = self.reorder.pop_ready()?;
        let is_partial = (notes.len() as u64) < self.chunk_size;

        // Trial-decrypt this chunk before emission (moved out of the
        // post-loop the one-shot path used to run).
        let mut decrypted = Vec::new();
        for (i, note) in notes.iter().enumerate() {
            if let Some((dec, address)) = try_decrypt_note(&self.ivk, note) {
                let nf: [u8; 32] = note.nullifier.as_slice().try_into().unwrap_or([0u8; 32]);
                let cmx: [u8; 32] = note.cmx.as_slice().try_into().unwrap_or([0u8; 32]);
                decrypted.push(DecryptedNote {
                    position: start_index + i as u64,
                    note: dec,
                    address,
                    nullifier: nf,
                    cmx,
                });
            }
        }

        Some(ShieldedChunkBatch {
            start_index,
            notes,
            decrypted,
            block_height,
            is_partial,
            // Emit the monotonic max-seen denominator, not this chunk's
            // own proven total. An older chunk completing after a newer
            // proof already raised the count must NOT lower it again.
            total_count: self.total_count,
        })
    }
}

/// Streaming variant of [`sync_shielded_notes`]: fetch shielded
/// encrypted notes starting at `start_index` with the existing
/// sliding-window parallelism, and yield each chunk **as soon as it is
/// the next contiguous one in tree order**.
///
/// Out-of-order network completions wait in an internal reorder buffer
/// until their predecessor has emitted, so the returned stream is
/// guaranteed to produce [`ShieldedChunkBatch`]es in strictly ascending
/// `start_index` order. Trial decryption against `ivk` happens per
/// chunk just before emission.
///
/// # Backpressure
///
/// The stream is pull-based ([`futures::stream::unfold`]): the internal
/// `FuturesUnordered` only advances when the consumer polls for the
/// next item, and a new fetch is queued only after a batch is emitted.
/// A consumer slower than the network therefore caps in-flight fetches
/// at `max_concurrent` and bounds the reorder buffer to the same window
/// — memory stays bounded without a separate spawned producer task
/// (which would also be unavailable under `wasm32`).
///
/// # Arguments
///
/// - `sdk` — SDK instance connected to the network
/// - `ivk` — prepared incoming viewing key for trial decryption
/// - `start_index` — first note position to fetch (must be a multiple
///   of the MMR chunk size, typically 2048)
/// - `config` — optional tuning; `None` uses sensible defaults
///
/// # Errors
///
/// A non-chunk-aligned `start_index` yields a single `Err` item, after
/// which the stream is exhausted. A fetch error likewise surfaces as an
/// `Err` item; callers should stop consuming on the first error.
pub fn sync_shielded_notes_stream(
    sdk: &Sdk,
    ivk: &PreparedIncomingViewingKey,
    start_index: u64,
    config: Option<ShieldedSyncConfig>,
) -> impl Stream<Item = Result<ShieldedChunkBatch, Error>> + Send {
    let config = config.unwrap_or_default();
    let (mmr_chunk_size, fetch_size) = resolve_sizes(sdk);

    // Validate alignment against the MMR chunk size (NOT the multi-chunk
    // fetch size). The server only requires per-MMR-chunk alignment; any
    // multiple of `mmr_chunk_size` is a legal start.
    let alignment_error = if mmr_chunk_size > 0 && !start_index.is_multiple_of(mmr_chunk_size) {
        Some(Error::Generic(format!(
            "start_index {} is not chunk-aligned; must be a multiple of {}",
            start_index, mmr_chunk_size
        )))
    } else {
        None
    };

    let max_concurrent = config.max_concurrent.max(1);
    let chunk_size = fetch_size;

    // Seed the initial sliding window of chunk queries.
    let futures: FuturesUnordered<ChunkFuture> = FuturesUnordered::new();
    let mut state = StreamState {
        sdk: sdk.clone(),
        ivk: ivk.clone(),
        chunk_size,
        start_index,
        settings: config.request_settings,
        on_progress: config.on_chunk_completed.clone(),
        futures,
        next_chunk_index: start_index,
        reorder: ReorderBuffer::new(start_index, chunk_size),
        reached_end: false,
        cumulative_scanned: 0,
        max_block_height: 0,
        total_count: 0,
    };
    for _ in 0..max_concurrent {
        state.queue_next();
    }

    // `unfold` yields `Some((item, next_state))` to emit `item`, or
    // `None` to end the stream. Each poll of the returned stream drives
    // exactly enough of the `FuturesUnordered` to produce the next
    // contiguous chunk, which is what makes backpressure pull-based.
    futures::stream::unfold(
        (state, alignment_error, false),
        move |(mut state, mut alignment_error, done)| async move {
            if done {
                return None;
            }
            // Surface a pre-validated alignment error as the sole item.
            if let Some(err) = alignment_error.take() {
                return Some((Err(err), (state, None, true)));
            }

            // A buffered chunk may already be ready (a predecessor just
            // emitted on a prior poll). Emit it before touching the
            // network again.
            if let Some(batch) = state.pop_ready() {
                // Queue replacement work for the chunk we just emitted so
                // the sliding window stays full.
                state.queue_next();
                return Some((Ok(batch), (state, None, false)));
            }

            // Pull completed fetches until the watermark chunk is ready.
            loop {
                let next = state.futures.next().await;
                let Some(result) = next else {
                    // No in-flight fetches and nothing buffered: stream
                    // is exhausted.
                    return None;
                };
                let (chunk_idx, notes, block_height, total_count) = match result {
                    Ok(v) => v,
                    Err(e) => return Some((Err(e), (state, None, true))),
                };

                let is_partial = (notes.len() as u64) < state.chunk_size;
                state.cumulative_scanned += notes.len() as u64;
                state.max_block_height = state.max_block_height.max(block_height);
                // The on-chain total is stable across a sync; take the
                // max-seen so a late-arriving chunk proven at a slightly
                // higher block never lowers the denominator.
                state.total_count = state.total_count.max(total_count);
                if is_partial {
                    state.reached_end = true;
                }
                // "Downloaded" progress fires per network chunk
                // completion, preserving the existing meaning.
                if let Some(cb) = state.on_progress.as_ref() {
                    // Absolute downloaded position (start_index + cumulative
                    // scanned). `start_index` can rewind below prior progress
                    // on a resume, so a consumer that also tracks committed
                    // ("checked") progress must clamp this to its own baseline
                    // to keep "Downloaded" from reading below "Checked" — the
                    // wallet does exactly that in `sync_notes_across`.
                    cb(
                        state.start_index + state.cumulative_scanned,
                        state.max_block_height,
                    );
                }
                state.reorder.insert(chunk_idx, (notes, block_height));

                if let Some(batch) = state.pop_ready() {
                    state.queue_next();
                    return Some((Ok(batch), (state, None, false)));
                }
                // Out-of-order completion: keep draining in-flight
                // fetches until the watermark chunk arrives.
            }
        },
    )
}

/// Fetch all shielded encrypted notes starting from `start_index`, query
/// multiple nodes in parallel, and perform trial decryption.
///
/// This is the one-shot entry point for wallet sync. It drives
/// [`sync_shielded_notes_stream`] to completion and assembles a single
/// [`ShieldedSyncResult`]. It handles:
/// 1. Chunk-aligned pagination (each query covers one BulkAppendTree chunk)
/// 2. Parallel dispatch of chunk queries across network nodes
/// 3. Proof verification on every response
/// 4. Trial decryption with the provided incoming viewing key
///
/// Prefer [`sync_shielded_notes_stream`] when the consumer can overlap
/// per-chunk work (e.g. tree-append) with later fetches.
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
    let stream = sync_shielded_notes_stream(sdk, ivk, start_index, config);
    futures::pin_mut!(stream);

    let mut all_notes: Vec<ShieldedEncryptedNote> = Vec::new();
    let mut decrypted_notes: Vec<DecryptedNote> = Vec::new();
    let mut total_notes_scanned: u64 = 0;
    let mut max_block_height: u64 = 0;
    // On-chain total note count from the note-fetch proofs (max-seen). The
    // value is stable across a sync; max-seen guards against a late chunk
    // proven at a slightly higher block lowering the denominator.
    let mut total_count: u64 = 0;
    // Mirrors the original one-shot logic exactly: track the LAST
    // non-empty chunk's `(start_index, is_partial)`. Batches arrive in
    // ascending `start_index`, so the last non-empty one we observe is
    // the same chunk `chunk_results.iter().rev().find(non-empty)` would
    // have selected. We only rewind `next_start_index` to it if that
    // last non-empty chunk is itself partial (a short buffer chunk that
    // may still grow before the next sync). Trailing empty chunks from
    // the still-draining sliding window are ignored, just like the
    // original `find(non-empty)` skipped them.
    let mut last_nonempty: Option<(u64, bool)> = None;

    while let Some(item) = stream.next().await {
        let batch = item?;
        max_block_height = max_block_height.max(batch.block_height);
        total_count = total_count.max(batch.total_count);
        total_notes_scanned += batch.notes.len() as u64;
        if !batch.notes.is_empty() {
            last_nonempty = Some((batch.start_index, batch.is_partial));
        }
        decrypted_notes.extend(batch.decrypted);
        all_notes.extend(batch.notes);
    }

    // Preserve `next_start_index` semantics exactly: if the last
    // non-empty chunk is partial, rewind to its start; otherwise resume
    // past everything scanned.
    let next_start_index = match last_nonempty {
        Some((s, true)) => s,
        _ => start_index + total_notes_scanned,
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
        block_height: max_block_height,
        total_count,
    })
}

#[cfg(test)]
mod tests {
    use super::ReorderBuffer;

    /// The reorder buffer underpins the stream's core guarantee:
    /// chunks emit in strictly ascending `start_index` even when the
    /// underlying network fetches complete out of order. This drives
    /// completions in a deliberately scrambled order and asserts the
    /// emitted sequence is monotonically increasing and contiguous.
    #[test]
    fn reorder_emits_strictly_ascending_under_out_of_order_arrival() {
        let chunk_size = 8192u64;
        let start = 0u64;
        let mut buf: ReorderBuffer<usize> = ReorderBuffer::new(start, chunk_size);

        // Five chunks at starts 0, 8192, 16384, 24576, 32768. Their
        // network fetches complete in scrambled order; we feed that
        // order in and pull whatever is ready after each arrival.
        let starts: Vec<u64> = (0..5).map(|k| start + k * chunk_size).collect();
        // Arrival order: 2nd, 4th, 1st, 5th, 3rd (indices into `starts`).
        let arrival = [1usize, 3, 0, 4, 2];

        let mut emitted: Vec<u64> = Vec::new();
        for (payload, &idx) in arrival.iter().enumerate() {
            buf.insert(starts[idx], payload);
            // Drain everything that became contiguously ready.
            while let Some((s, _payload)) = buf.pop_ready() {
                emitted.push(s);
            }
        }

        // Every chunk emitted exactly once, in ascending tree order —
        // i.e. identical to the in-order baseline regardless of the
        // scrambled arrival order.
        assert_eq!(
            emitted, starts,
            "reorder buffer must emit chunks in ascending start_index"
        );
        // Strictly ascending check (defensive; redundant with the
        // equality above but pins the invariant explicitly).
        assert!(
            emitted.windows(2).all(|w| w[0] < w[1]),
            "emitted start_index sequence must be strictly ascending"
        );
    }

    /// A later chunk that finishes first must NOT emit before its
    /// predecessor — it waits in the buffer until the watermark reaches
    /// it. Verifies the gate holds across interleaved insert/pop calls.
    #[test]
    fn reorder_holds_back_chunk_until_predecessor_emits() {
        let chunk_size = 4u64;
        let mut buf: ReorderBuffer<&'static str> = ReorderBuffer::new(0, chunk_size);

        // Chunk at start=4 (the second chunk) arrives first.
        buf.insert(4, "second");
        // Nothing is emittable yet — watermark is still 0.
        assert!(buf.pop_ready().is_none());

        // Now the first chunk arrives.
        buf.insert(0, "first");
        // Both should drain, in order.
        assert_eq!(buf.pop_ready(), Some((0, "first")));
        assert_eq!(buf.pop_ready(), Some((4, "second")));
        assert!(buf.pop_ready().is_none());
    }

    /// Non-zero `start_index` (a resume point): the watermark must
    /// begin at `start_index`, not 0, so a mid-tree resume emits its
    /// first chunk immediately.
    #[test]
    fn reorder_respects_nonzero_start_index() {
        let chunk_size = 8192u64;
        let start = 16384u64;
        let mut buf: ReorderBuffer<u64> = ReorderBuffer::new(start, chunk_size);

        buf.insert(start + chunk_size, 1);
        assert!(buf.pop_ready().is_none(), "successor must wait");
        buf.insert(start, 0);
        assert_eq!(buf.pop_ready(), Some((start, 0)));
        assert_eq!(buf.pop_ready(), Some((start + chunk_size, 1)));
    }
}
