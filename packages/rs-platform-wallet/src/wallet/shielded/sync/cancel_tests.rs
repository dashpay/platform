//! Cancellation of a note-sync pass (dashpay/platform#5200).
//!
//! Feeds `sync_notes_from_stream` a synthetic batch stream so the
//! cancel and finalize paths run without a network. The stream yields
//! real-shaped batches, then a tail future that cancels the token and
//! pends forever — exactly the moment a stop would land between two
//! chunk fetches.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::{Stream, StreamExt};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::{sync_notes_from_stream, MultiSyncNotesResult};
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{ShieldedStore, SubwalletId};

type BatchItem = Result<ShieldedChunkBatch, dash_sdk::Error>;

fn temp_tree_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_sync_cancel_{tag}_{nanos}.sqlite"))
}

fn one_subwallet() -> Vec<(SubwalletId, AccountViewingKeys)> {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
        .expect("derive viewing keys")
        .viewing_keys();
    vec![(SubwalletId::new([0x11; 32], 0), views)]
}

/// A batch of `len` notes at `start_index`. The notes are opaque to the
/// scanner: an empty ciphertext never OVK-recovers, the single subwallet
/// is the stream driver so no IVK trial-decryption runs, and a nullifier
/// nobody owns is a no-op in the spend replay. Only the commitment is
/// load-bearing — a distinct small value is a valid Pallas element.
fn batch(start_index: u64, len: u64, total_count: u64) -> ShieldedChunkBatch {
    let notes = (0..len)
        .map(|i| {
            let mut cmx = [0u8; 32];
            let pos = start_index + i;
            cmx[0] = (pos as u8) + 1;
            cmx[1] = (pos >> 8) as u8;
            ShieldedEncryptedNote {
                cmx: cmx.to_vec(),
                nullifier: vec![0xAA; 32],
                cv_net: vec![],
                encrypted_note: vec![],
            }
        })
        .collect();
    ShieldedChunkBatch {
        start_index,
        notes,
        decrypted: vec![],
        block_height: 1_000 + start_index,
        is_partial: false,
        total_count,
    }
}

/// `batches`, then a tail that fires `cancel` when first polled and never
/// yields: the consumer sees the cancellation exactly between batches.
fn batches_then_cancel(
    batches: Vec<ShieldedChunkBatch>,
    cancel: CancellationToken,
) -> impl Stream<Item = BatchItem> {
    futures::stream::iter(batches.into_iter().map(Ok)).chain(futures::stream::once(async move {
        cancel.cancel();
        futures::future::pending::<BatchItem>().await
    }))
}

async fn run(
    store: &Arc<RwLock<FileBackedShieldedStore>>,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
    cancel: &CancellationToken,
    stream: impl Stream<Item = BatchItem>,
) -> MultiSyncNotesResult {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sync_notes_from_stream(store, subwallets, None, cancel, move |_, _| stream),
    )
    .await
    .expect("a cancelled pass must return promptly")
    .expect("pass must succeed")
}

#[tokio::test]
async fn should_finalize_consumed_prefix_when_cancelled_mid_stream() {
    let path = temp_tree_path("prefix");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    let cancel = CancellationToken::new();

    let result = run(
        &store,
        &subwallets,
        &cancel,
        batches_then_cancel(vec![batch(0, 4, 12), batch(4, 4, 12)], cancel.clone()),
    )
    .await;

    // The prefix the pass consumed is finalized like a completed pass.
    assert_eq!(result.total_scanned, 8);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 8, "both batches appended");
    assert_eq!(
        store.last_synced_note_index(sub).unwrap(),
        8,
        "watermark covers exactly the consumed batches"
    );
    assert!(
        store.witness(7).unwrap().is_some(),
        "a checkpoint covers the appended leaves"
    );

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn should_return_empty_without_touching_store_when_cancelled_before_scan() {
    let path = temp_tree_path("before");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    let cancel = CancellationToken::new();
    cancel.cancel();

    let result = sync_notes_from_stream(
        &store,
        &subwallets,
        None,
        &cancel,
        |_, _| -> futures::stream::Pending<BatchItem> {
            panic!("a pass cancelled before the scan must not build the stream")
        },
    )
    .await
    .expect("pass must succeed");

    assert_eq!(result.total_scanned, 0);
    assert!(result.per_subwallet_new_notes.is_empty());
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0);
    assert_eq!(store.last_synced_note_index(sub).unwrap(), 0);

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn should_resume_from_partial_watermark_on_next_pass() {
    let path = temp_tree_path("resume");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;

    // Pass 1: cancelled after two batches.
    let cancel = CancellationToken::new();
    run(
        &store,
        &subwallets,
        &cancel,
        batches_then_cancel(vec![batch(0, 4, 12), batch(4, 4, 12)], cancel.clone()),
    )
    .await;
    assert_eq!(store.read().await.last_synced_note_index(sub).unwrap(), 8);

    // Pass 2: the SDK re-fetches from the chunk-aligned start (0 here), so
    // the stream re-delivers the first two batches — gate-skipped by the
    // tree-size snapshot — and then the new one. Runs to completion.
    let cancel = CancellationToken::new();
    let result = run(
        &store,
        &subwallets,
        &cancel,
        futures::stream::iter(
            vec![batch(0, 4, 12), batch(4, 4, 12), batch(8, 4, 12)]
                .into_iter()
                .map(Ok),
        ),
    )
    .await;

    assert_eq!(
        result.total_scanned, 4,
        "only the new notes count as scan volume"
    );
    let store = store.read().await;
    assert_eq!(
        store.tree_size().unwrap(),
        12,
        "re-delivered positions are not re-appended"
    );
    assert_eq!(store.last_synced_note_index(sub).unwrap(), 12);
    assert!(
        store.witness(11).unwrap().is_some(),
        "the second pass checkpoints past the partial one"
    );
    assert!(store.witness(7).unwrap().is_some());

    let _ = std::fs::remove_file(&path);
}
