//! Budget/checkpoint behavior of the foreign-key (one-time-invitation)
//! note scan — dashpay/platform#4306.
//!
//! Exercises the extracted, stream-generic
//! [`super::scan_foreign_stream_with_budget`] with synthetic
//! [`ShieldedChunkBatch`]es, the same way the Part-A tests exercise
//! `apply_scanned_nullifier_spends` instead of the full network path:
//! `sync_shielded_notes_stream` is the sole production stream, and nothing
//! here depends on how it fetches.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::{DecryptedNote, ShieldedChunkBatch};
use dashcore::Network;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::stream;
use grovedb_commitment_tree::{ExtractedNoteCommitment, Note, NoteValue, RandomSeed, Rho};

use super::{
    foreign_scan_checkpoint_key, scan_foreign_stream_with_budget, ForeignScanCheckpointCache,
    CHUNK_SIZE, FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET,
};
use crate::error::PlatformWalletError;
use crate::wallet::shielded::keys::OrchardKeySet;

/// Any deterministic keyset works — the FVK derives nullifiers for decrypted
/// notes, and the default address is the recipient the note builder uses.
fn keyset() -> OrchardKeySet {
    OrchardKeySet::from_seed(&[0x42; 32], Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed")
}

fn fvk() -> grovedb_commitment_tree::FullViewingKey {
    keyset().full_viewing_key
}

/// Build a REAL decrypted note at tree `position` worth `value` credits, to
/// the keyset's default address — the exact shape the production stream's
/// trial decryption yields, so the scan's nullifier/serialization path runs
/// on it unmodified. Canonical `rho`/`rseed` are found by deterministic
/// rejection sampling seeded from `position` (distinct positions get
/// distinct note material).
fn decrypted_note_at(keys: &OrchardKeySet, position: u64, value: u64) -> DecryptedNote {
    let rho = (0u64..u64::MAX)
        .find_map(|attempt| {
            let mut b = [0u8; 32];
            b[0..8].copy_from_slice(&position.to_le_bytes());
            b[8..16].copy_from_slice(&attempt.to_le_bytes());
            Rho::from_bytes(&b).into_option()
        })
        .expect("a canonical rho exists");
    let rseed = (0u64..u64::MAX)
        .find_map(|attempt| {
            let mut b = [0u8; 32];
            b[0..8].copy_from_slice(&attempt.to_le_bytes());
            b[16..24].copy_from_slice(&position.to_le_bytes());
            RandomSeed::from_bytes(b, &rho).into_option()
        })
        .expect("a canonical rseed exists");
    let note = Note::from_parts(keys.default_address, NoteValue::from_raw(value), rho, rseed)
        .into_option()
        .expect("valid note parts");
    DecryptedNote {
        position,
        nullifier: note.nullifier(&keys.full_viewing_key).to_bytes(),
        cmx: ExtractedNoteCommitment::from(note.commitment()).to_bytes(),
        address: keys.default_address,
        note,
    }
}

/// A wire note whose contents never matter — the driver IVK decrypted
/// nothing, so only `notes.len()` (the chunk's coverage) is read.
fn wire_note() -> ShieldedEncryptedNote {
    ShieldedEncryptedNote {
        cmx: vec![0u8; 32],
        nullifier: vec![0u8; 32],
        cv_net: vec![0u8; 32],
        encrypted_note: vec![0u8; 216],
    }
}

/// A FULL chunk batch at `start_index` covering exactly [`CHUNK_SIZE`]
/// notes, none of which decrypted under the driver key.
fn full_batch(start_index: u64) -> ShieldedChunkBatch {
    full_batch_with(start_index, Vec::new())
}

/// A FULL chunk batch at `start_index` whose trial decryption yielded
/// `decrypted` (positions must lie inside the chunk).
fn full_batch_with(start_index: u64, decrypted: Vec<DecryptedNote>) -> ShieldedChunkBatch {
    ShieldedChunkBatch {
        start_index,
        notes: (0..CHUNK_SIZE).map(|_| wire_note()).collect(),
        decrypted,
        block_height: 7,
        is_partial: false,
        total_count: 0,
    }
}

/// The final (buffer) chunk — a short read signalling end-of-stream.
fn partial_batch(start_index: u64) -> ShieldedChunkBatch {
    ShieldedChunkBatch {
        start_index,
        notes: vec![wire_note()],
        decrypted: Vec::new(),
        block_height: 7,
        is_partial: true,
        total_count: 0,
    }
}

type BatchResult = Result<ShieldedChunkBatch, std::convert::Infallible>;

/// THE #4306 guarantee: a valid-but-unfunded key cannot drive an unbounded
/// walk. The scan pauses at its per-attempt batch budget with the RETRYABLE
/// typed error, the checkpoint records exactly how far it got, and the next
/// attempt resumes from that position instead of restarting — so attempts
/// compound toward a genuinely deep note while each stays bounded.
#[tokio::test]
async fn budget_exhaustion_pauses_with_checkpoint_and_the_retry_resumes() {
    let fvk = fvk();
    let checkpoints = ForeignScanCheckpointCache::default();
    let key = foreign_scan_checkpoint_key(&fvk);

    // Attempt 1: four full chunks available, budget of two.
    let batches: Vec<BatchResult> = (0..4).map(|i| Ok(full_batch(i * CHUNK_SIZE))).collect();
    let err = scan_foreign_stream_with_budget(
        stream::iter(batches),
        &checkpoints,
        key,
        &fvk,
        u64::MAX, // value never covered — the unfunded-key shape
        0,
        Arc::from(Vec::new()),
        0,
        2,
    )
    .await
    .expect_err("exhausting the budget before the value must pause, not scan on");

    // Two full chunks were consumed, so coverage reached 2 × CHUNK_SIZE.
    let paused_at = match err {
        PlatformWalletError::ShieldedForeignScanBudgetExhausted { scanned_through } => {
            scanned_through
        }
        other => panic!("expected ShieldedForeignScanBudgetExhausted, got {other:?}"),
    };
    assert_eq!(paused_at, 2 * CHUNK_SIZE);
    assert_eq!(
        checkpoints
            .load(&key)
            .expect("pause must checkpoint its progress")
            .resume_position,
        2 * CHUNK_SIZE,
        "the checkpoint and the error must agree on how far the scan got"
    );

    // Attempt 2 — as the production caller would run it: resume from the
    // checkpoint, feed the REMAINING chunks, and let the tree end. The
    // exhausted tree is the ordinary Ok(found) return, not a budget pause.
    let resume = checkpoints.load(&key).unwrap().resume_position;
    let remaining: Vec<BatchResult> = vec![
        Ok(full_batch(resume)),
        Ok(full_batch(resume + CHUNK_SIZE)),
        Ok(partial_batch(resume + 2 * CHUNK_SIZE)),
    ];
    let found = scan_foreign_stream_with_budget(
        stream::iter(remaining),
        &checkpoints,
        key,
        &fvk,
        u64::MAX,
        resume,
        Arc::from(Vec::new()),
        0,
        // Budget 3: the scan cannot know a NEXT batch is the last, so ending
        // within budget means strictly fewer FULL batches than the budget —
        // two fulls under a budget of two would pause again (correctly; the
        // next attempt's first batch would be the partial).
        3,
    )
    .await
    .expect("an exhausted tree returns Ok with whatever was found");
    assert!(found.is_empty(), "nothing decrypted — the key is unfunded");

    // The buffer chunk may still receive notes, so the checkpoint holds AT
    // its start — the next attempt rescans only the mutable chunk.
    assert_eq!(
        checkpoints.load(&key).unwrap().resume_position,
        resume + 2 * CHUNK_SIZE
    );
}

/// A partial batch is end-of-stream: it must never trip the budget, or a
/// one-chunk tree scanned with a one-batch budget would loop "retry" forever
/// without ever reaching the ordinary exhausted-tree return.
#[tokio::test]
async fn the_final_partial_batch_never_trips_the_budget() {
    let fvk = fvk();
    let checkpoints = ForeignScanCheckpointCache::default();
    let key = foreign_scan_checkpoint_key(&fvk);

    let batches: Vec<BatchResult> = vec![Ok(partial_batch(0))];
    let found = scan_foreign_stream_with_budget(
        stream::iter(batches),
        &checkpoints,
        key,
        &fvk,
        u64::MAX,
        0,
        Arc::from(Vec::new()),
        0,
        1, // tightest possible budget
    )
    .await
    .expect("a stream that ENDS within budget is the ordinary exhausted-tree return");
    assert!(found.is_empty());
    assert_eq!(checkpoints.load(&key).unwrap().resume_position, 0);
}

/// A mid-stream error still checkpoints the progress made before it — the
/// pre-existing contract, re-pinned here because the budget refactor moved
/// the loop into the stream-generic helper.
#[tokio::test]
async fn a_stream_error_checkpoints_partial_progress() {
    let fvk = fvk();
    let checkpoints = ForeignScanCheckpointCache::default();
    let key = foreign_scan_checkpoint_key(&fvk);

    let batches: Vec<Result<ShieldedChunkBatch, String>> = vec![
        Ok(full_batch(0)),
        Err("connection reset".to_string()),
        Ok(full_batch(CHUNK_SIZE)),
    ];
    let err = scan_foreign_stream_with_budget(
        stream::iter(batches),
        &checkpoints,
        key,
        &fvk,
        u64::MAX,
        0,
        Arc::from(Vec::new()),
        0,
        16,
    )
    .await
    .expect_err("the stream error must surface");
    assert!(matches!(err, PlatformWalletError::ShieldedSyncFailed(_)));
    assert_eq!(
        checkpoints.load(&key).unwrap().resume_position,
        CHUNK_SIZE,
        "the retry after a stream error resumes past the chunk already covered"
    );
}

/// The #4313 memory bound (review finding d6b7be21f4a4): the per-attempt
/// batch budget bounds WORK, but an untrusted inviter controls how many
/// notes decrypt under a published invitation key, and every note found
/// below the resume position used to be RETAINED in the checkpoint across
/// budgeted retries. A many-note funding must not pin unbounded memory —
/// AND must not stall progress (review finding df7d9c0f41ab): whatever the
/// cache holds for the key after a flood stays within
/// [`FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET`], while the resume position still
/// records exactly how far the paused scan got.
#[tokio::test]
async fn a_note_flood_cannot_pin_unbounded_checkpoint_memory() {
    let keys = keyset();
    let checkpoints = ForeignScanCheckpointCache::default();
    let key = foreign_scan_checkpoint_key(&keys.full_viewing_key);

    // A chunk carrying more dust notes than the retained-note budget, whose
    // values never cover the requested amount, followed by enough chunks to
    // trip the batch budget — the compounding-retry shape a malicious
    // inviter forces.
    let flood = FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET + 4;
    let flood_notes: Vec<DecryptedNote> = (0..flood)
        .map(|i| decrypted_note_at(&keys, i as u64, 1))
        .collect();
    let batches: Vec<BatchResult> = vec![
        Ok(full_batch_with(0, flood_notes)),
        Ok(full_batch(CHUNK_SIZE)),
        Ok(full_batch(2 * CHUNK_SIZE)),
    ];
    let err = scan_foreign_stream_with_budget(
        stream::iter(batches),
        &checkpoints,
        key,
        &keys.full_viewing_key,
        u64::MAX, // never covered — the scan runs to its batch budget
        0,
        Arc::from(Vec::new()),
        0,
        2,
    )
    .await
    .expect_err("the budget must pause the scan before the value is covered");
    assert!(matches!(
        err,
        PlatformWalletError::ShieldedForeignScanBudgetExhausted { .. }
    ));

    // THE BOUND — and the PROGRESS. Pre-fix, the pause checkpointed every
    // flood note; the first repair refused the over-budget checkpoint whole,
    // which capped memory but froze the resume position (every retry
    // rescanned the same window forever). The save now retains the
    // highest-value notes within the budget AND advances the position.
    let cp = checkpoints
        .load(&key)
        .expect("a flooded pause must still checkpoint its progress");
    assert!(
        cp.notes.len() <= FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET,
        "a note flood must not be retained past the budget (got {})",
        cp.notes.len()
    );
    assert_eq!(
        cp.resume_position,
        2 * CHUNK_SIZE,
        "the flooded checkpoint must record the coverage the pause reached"
    );
}

/// THE round-5 liveness guarantee (review finding df7d9c0f41ab): more dust
/// notes than the retained-note budget, followed by a funding note DEEPER
/// than the first attempt's batch-budget window. Refusing the over-budget
/// checkpoint whole (the first repair of d6b7be21f4a4) froze the resume
/// position here: every retry reloaded the same old checkpoint, rediscovered
/// the same over-budget prefix, had its save refused again, and exhausted
/// the same window — a genuinely funded invitation became permanently
/// unclaimable. The bounded save must instead ADVANCE the position while
/// retaining the highest-value notes, so attempts compound toward the deep
/// funding note and the claim completes.
#[tokio::test]
async fn a_note_flood_beyond_the_first_window_still_reaches_a_deep_funding_note() {
    let keys = keyset();
    let checkpoints = ForeignScanCheckpointCache::default();
    let key = foreign_scan_checkpoint_key(&keys.full_viewing_key);

    // Dust with DISTINCT values 1..=20 (budget + 4): the retained set is
    // then observable — the 16 most valuable notes (values 5..=20, summing
    // to 200) survive, values 1..=4 are dropped.
    let flood = FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET + 4;
    let flood_notes: Vec<DecryptedNote> = (0..flood)
        .map(|i| decrypted_note_at(&keys, i as u64, i as u64 + 1))
        .collect();

    // Attempt 1: the flood chunk, then a budget pause two chunks in. The
    // funding note lies beyond this window.
    let batches: Vec<BatchResult> = vec![
        Ok(full_batch_with(0, flood_notes)),
        Ok(full_batch(CHUNK_SIZE)),
        Ok(full_batch(2 * CHUNK_SIZE)),
    ];
    scan_foreign_stream_with_budget(
        stream::iter(batches),
        &checkpoints,
        key,
        &keys.full_viewing_key,
        u64::MAX,
        0,
        Arc::from(Vec::new()),
        0,
        2,
    )
    .await
    .expect_err("attempt 1 pauses at its batch budget");

    // MONOTONIC PROGRESS: the flooded pause must have advanced the
    // checkpoint past the chunks it covered — this is exactly what the
    // refuse-whole behavior failed to do.
    let cp = checkpoints
        .load(&key)
        .expect("the flooded pause must leave a resumable checkpoint");
    assert_eq!(
        cp.resume_position,
        2 * CHUNK_SIZE,
        "attempt 2 must resume past attempt 1's window, not repeat it"
    );
    assert_eq!(
        cp.notes.len(),
        FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET,
        "the checkpoint retains exactly the budget's worth of notes"
    );
    assert!(
        cp.notes.iter().all(|n| n.value >= 5),
        "the retained notes must be the HIGHEST-value ones (dust 1..=4 dropped)"
    );

    // Attempt 2 — as the production caller would run it: resume from the
    // checkpoint with its carried notes, and find the deep funding note in
    // the first fresh chunk. The stop value (450) needs the funding note
    // AND most of the carried value (200 + 300 = 500 ≥ 450), so a claim
    // that forgot the retained notes — or never reached the funding note —
    // fails the total.
    let carried = cp.notes;
    let carried_total: u64 = carried.iter().map(|n| n.value).sum();
    assert_eq!(carried_total, 200, "values 5..=20 sum to 200");
    let resume = cp.resume_position;
    let stop_at_value = 450;
    let batches2: Vec<BatchResult> = vec![
        Ok(full_batch_with(
            resume,
            vec![decrypted_note_at(&keys, resume + 3, 300)],
        )),
        Ok(partial_batch(resume + CHUNK_SIZE)),
    ];
    let found = scan_foreign_stream_with_budget(
        stream::iter(batches2),
        &checkpoints,
        key,
        &keys.full_viewing_key,
        stop_at_value,
        resume,
        carried,
        carried_total,
        8,
    )
    .await
    .expect("the resumed attempt completes the claim the pause deferred");

    assert_eq!(
        found.len(),
        FOREIGN_SCAN_CHECKPOINT_NOTE_BUDGET + 1,
        "the retained notes plus the funding note must all be selectable"
    );
    let total: u64 = found.iter().map(|n| n.value).sum();
    assert_eq!(
        total, 500,
        "retained value (200) plus the funding note (300) — nothing forgotten"
    );
    assert!(
        found
            .iter()
            .any(|n| n.position == resume + 3 && n.value == 300),
        "the deep funding note itself must be present"
    );
}
