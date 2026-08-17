//! Budget/checkpoint behavior of the foreign-key (one-time-invitation)
//! note scan — dashpay/platform#4306.
//!
//! Exercises the extracted, stream-generic
//! [`super::scan_foreign_stream_with_budget`] with synthetic
//! [`ShieldedChunkBatch`]es, the same way the Part-A tests exercise
//! `apply_scanned_nullifier_spends` instead of the full network path:
//! `sync_shielded_notes_stream` is the sole production stream, and nothing
//! here depends on how it fetches.

use dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch;
use dashcore::Network;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::stream;

use super::{
    foreign_scan_checkpoint_key, scan_foreign_stream_with_budget, ForeignScanCheckpointCache,
    CHUNK_SIZE,
};
use crate::error::PlatformWalletError;
use crate::wallet::shielded::keys::OrchardKeySet;

/// Any deterministic keyset works — the FVK is only consulted to derive
/// nullifiers for decrypted notes, and these batches carry none.
fn fvk() -> grovedb_commitment_tree::FullViewingKey {
    OrchardKeySet::from_seed(&[0x42; 32], Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed")
        .full_viewing_key
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
    ShieldedChunkBatch {
        start_index,
        notes: (0..CHUNK_SIZE).map(|_| wire_note()).collect(),
        decrypted: Vec::new(),
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
        Vec::new(),
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
        Vec::new(),
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
        Vec::new(),
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
        Vec::new(),
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
