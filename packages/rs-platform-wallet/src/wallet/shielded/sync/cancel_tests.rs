//! Cancellation of a note-sync pass (dashpay/platform#5200), and the
//! height a completed pass covers.
//!
//! Feeds `sync_notes_from_stream` a synthetic batch stream so the
//! cancel path runs without a network. The stream yields real-shaped
//! batches, then a tail future that cancels the token and pends forever
//! — exactly the moment a stop would land between two chunk fetches.
//! A cut-short pass keeps the tree progress it made and publishes
//! nothing else; the next pass re-walks from the completed watermark.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::{Stream, StreamExt};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::{sync_notes_from_stream, MultiSyncNotesResult, NoteSyncOutcome, StoreEpoch};
use crate::wallet::shielded::balance::ShieldedBalanceSource;
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{InMemoryShieldedStore, ShieldedStore, SubwalletId};

type BatchItem = Result<ShieldedChunkBatch, dash_sdk::Error>;

fn temp_tree_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_sync_cancel_{tag}_{nanos}.sqlite"))
}

fn subwallet(account: u32) -> (SubwalletId, AccountViewingKeys) {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, account)
        .expect("derive viewing keys")
        .viewing_keys();
    (SubwalletId::new([0x11; 32], account), views)
}

fn one_subwallet() -> Vec<(SubwalletId, AccountViewingKeys)> {
    vec![subwallet(0)]
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
) -> NoteSyncOutcome {
    let epoch = StoreEpoch::default();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sync_notes_from_stream(
            store,
            &epoch.watch(),
            subwallets,
            None,
            cancel,
            move |_, _| stream,
        ),
    )
    .await
    .expect("a cancelled pass must return promptly")
    .expect("pass must succeed")
}

fn assert_cancelled(outcome: NoteSyncOutcome) {
    assert!(
        matches!(outcome, NoteSyncOutcome::Cancelled),
        "a cut-short pass publishes nothing, got {outcome:?}"
    );
}

fn expect_completed(outcome: NoteSyncOutcome) -> MultiSyncNotesResult {
    match outcome {
        NoteSyncOutcome::Completed(result) => result,
        other => panic!("pass should have completed, got {other:?}"),
    }
}

#[tokio::test]
async fn should_keep_tree_progress_and_publish_nothing_when_cancelled_mid_stream() {
    let path = temp_tree_path("prefix");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    let cancel = CancellationToken::new();

    let outcome = run(
        &store,
        &subwallets,
        &cancel,
        batches_then_cancel(vec![batch(0, 4, 12), batch(4, 4, 12)], cancel.clone()),
    )
    .await;

    // The appended leaves stay, checkpointed; nothing else is published.
    assert_cancelled(outcome);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 8, "both batches appended");
    assert!(
        store.witness(7).unwrap().is_some(),
        "a checkpoint covers the appended leaves"
    );
    assert_eq!(
        store.last_synced_note_index(sub).unwrap(),
        0,
        "the watermark only moves when a pass reaches the tip"
    );
    let balance = store.local_account_balance(sub).unwrap();
    assert_eq!(balance.last_scanned_index, None);
    assert_eq!(balance.source, ShieldedBalanceSource::NoHistory);

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

    let outcome = sync_notes_from_stream(
        &store,
        &StoreEpoch::default().watch(),
        &subwallets,
        None,
        &cancel,
        |_, _| -> futures::stream::Pending<BatchItem> {
            panic!("a pass cancelled before the scan must not build the stream")
        },
    )
    .await
    .expect("pass must succeed");

    assert_cancelled(outcome);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0);
    assert_eq!(store.last_synced_note_index(sub).unwrap(), 0);

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn should_rewalk_the_appended_prefix_without_reappending_on_next_pass() {
    let path = temp_tree_path("resume");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;

    // Pass 1: cancelled after two batches.
    let cancel = CancellationToken::new();
    assert_cancelled(
        run(
            &store,
            &subwallets,
            &cancel,
            batches_then_cancel(vec![batch(0, 4, 12), batch(4, 4, 12)], cancel.clone()),
        )
        .await,
    );
    assert_eq!(store.read().await.last_synced_note_index(sub).unwrap(), 0);
    assert_eq!(store.read().await.tree_size().unwrap(), 8);

    // Pass 2: re-walks from the completed watermark (0), so the stream
    // re-delivers the first two batches — gate-skipped by the tree-size
    // snapshot — and then the new one. Runs to completion.
    let cancel = CancellationToken::new();
    let result = expect_completed(
        run(
            &store,
            &subwallets,
            &cancel,
            futures::stream::iter(
                vec![batch(0, 4, 12), batch(4, 4, 12), batch(8, 4, 12)]
                    .into_iter()
                    .map(Ok),
            ),
        )
        .await,
    );

    assert_eq!(
        result.total_scanned, 12,
        "the whole range from the completed watermark is scanned"
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
    let balance = store.local_account_balance(sub).unwrap();
    assert_eq!(balance.last_scanned_index, Some(12));
    assert_eq!(
        balance.source,
        ShieldedBalanceSource::ScannedThisSession,
        "the completed pass certifies the scan"
    );

    let _ = std::fs::remove_file(&path);
}

/// Two subwallets, one already past where the cancelled prefix ends.
/// `aligned_start` is the union MIN, so the pass walks 0..8 while the
/// second subwallet sits at 12: neither watermark moves, and the
/// changeset carries nothing to the host.
#[tokio::test]
async fn should_leave_every_watermark_alone_when_cancelled() {
    let path = temp_tree_path("no_rewind");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = vec![subwallet(0), subwallet(1)];
    let (lagging, advanced) = (subwallets[0].0, subwallets[1].0);
    store
        .write()
        .await
        .set_last_synced_note_index(advanced, 12)
        .expect("seed the advanced watermark");
    let cancel = CancellationToken::new();

    let outcome = run(
        &store,
        &subwallets,
        &cancel,
        batches_then_cancel(vec![batch(0, 4, 12), batch(4, 4, 12)], cancel.clone()),
    )
    .await;

    assert_cancelled(outcome);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 8);
    assert_eq!(store.last_synced_note_index(lagging).unwrap(), 0);
    assert_eq!(
        store.last_synced_note_index(advanced).unwrap(),
        12,
        "a partial prefix below a subwallet's watermark must not rewind it"
    );

    let _ = std::fs::remove_file(&path);
}

/// Cancelled after the store lock but before the first batch: nothing
/// was consumed, so the store and the changeset stay untouched.
#[tokio::test]
async fn should_leave_store_and_changeset_empty_when_cancelled_before_first_batch() {
    let path = temp_tree_path("first_batch");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    store
        .write()
        .await
        .set_last_synced_note_index(sub, 4)
        .expect("seed the watermark");
    let cancel = CancellationToken::new();

    let outcome = run(
        &store,
        &subwallets,
        &cancel,
        batches_then_cancel(vec![], cancel.clone()),
    )
    .await;

    assert_cancelled(outcome);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0);
    assert_eq!(store.last_synced_note_index(sub).unwrap(), 4);

    let _ = std::fs::remove_file(&path);
}

/// Cancelled while another writer holds the store: the pass has touched
/// nothing yet, so it must return without waiting for that writer.
#[tokio::test]
async fn should_return_while_waiting_for_the_store_lock_when_cancelled() {
    let path = temp_tree_path("lock_wait");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    let cancel = CancellationToken::new();

    let held = store.write().await;
    let epoch = StoreEpoch::default();
    let watch = epoch.watch();
    let pass = sync_notes_from_stream(&store, &watch, &subwallets, None, &cancel, |_, _| {
        futures::stream::iter(vec![batch(0, 4, 4)].into_iter().map(Ok))
    });
    futures::pin_mut!(pass);
    assert!(
        futures::poll!(pass.as_mut()).is_pending(),
        "the pass is queued behind the held guard"
    );

    cancel.cancel();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), pass.as_mut())
        .await
        .expect("a cancelled pass returns without the guard")
        .expect("pass must succeed");
    assert_cancelled(outcome);

    drop(held);
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0, "nothing was appended");
    assert_eq!(store.last_synced_note_index(sub).unwrap(), 0);

    let _ = std::fs::remove_file(&path);
}

/// A batch of `len` notes at `start_index`, proved at `block_height`, whose
/// proof (with every one read before it) shows `total_count` notes.
fn proved_batch(
    start_index: u64,
    len: u64,
    total_count: u64,
    block_height: u64,
) -> ShieldedChunkBatch {
    ShieldedChunkBatch {
        block_height,
        ..batch(start_index, len, total_count)
    }
}

/// A completed pass covers the proved height of the last batch of its run
/// whose proof shows the tree ending within that run, and no height when
/// every proof shows a longer tree than the run reached.
#[tokio::test]
async fn should_report_the_height_a_completed_scan_covers() {
    let cases: [(&str, Vec<ShieldedChunkBatch>, Option<u64>); 4] = [
        (
            "the last batch ends the tree",
            vec![proved_batch(0, 4, 6, 1_000), proved_batch(4, 2, 6, 1_004)],
            Some(1_004),
        ),
        (
            "a proof of a longer tree than the run reached",
            vec![proved_batch(0, 4, 9, 1_000), proved_batch(4, 2, 9, 1_004)],
            None,
        ),
        (
            "an empty chunk past a full one, at a later height",
            vec![proved_batch(0, 4, 4, 1_000), proved_batch(4, 0, 4, 1_005)],
            Some(1_005),
        ),
        (
            "a batch past a gap in the run",
            vec![proved_batch(0, 4, 4, 1_000), proved_batch(8, 0, 4, 1_009)],
            Some(1_000),
        ),
    ];
    for (case, batches, covered) in cases {
        let path = temp_tree_path("covered");
        let store = Arc::new(RwLock::new(
            FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
        ));
        let result = expect_completed(
            run(
                &store,
                &one_subwallet(),
                &CancellationToken::new(),
                futures::stream::iter(batches.into_iter().map(Ok)),
            )
            .await,
        );
        assert_eq!(result.covered_height, covered, "{case}");
        let _ = std::fs::remove_file(&path);
    }
}

/// A pass that sees a saved spend's fee note spent keeps, durably and
/// before the mark drops the spend's record, the highest height any of its
/// batches was proven at, even when a later batch was proven lower: the
/// token note sweep must not judge that spend from a token pool scan below
/// it, after a restart too.
#[tokio::test]
async fn should_keep_the_highest_batch_height_before_a_spend_drops_its_record() {
    use crate::wallet::shielded::store::{PendingRedrive, ShieldedNote};
    use crate::wallet::shielded::token_pool::{raise_sweep_height, TokenPoolConnection};

    let path = temp_tree_path("spend_floor");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    // Every note of the synthetic batches reveals this nullifier.
    let fee_nf = [0xAA; 32];
    {
        let mut store = store.write().await;
        store
            .save_note(
                sub,
                &ShieldedNote {
                    position: 0,
                    cmx: [0x11; 32],
                    nullifier: fee_nf,
                    block_height: 10,
                    is_spent: false,
                    value: 1_000,
                    note_data: vec![0u8; 115],
                },
            )
            .unwrap();
        store
            .arm_redrive(
                sub,
                PendingRedrive {
                    activity_id: [0xA1; 32],
                    anchor: [0xA2; 32],
                    nullifiers: vec![fee_nf],
                    st_bytes: vec![0],
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                    anchor_height: Some(1),
                },
            )
            .unwrap();
    }
    let mut first = batch(0, 4, 8);
    first.block_height = 1_011;
    let mut last = batch(4, 4, 8);
    last.block_height = 1_010;
    let cancel = CancellationToken::new();
    run(
        &store,
        &subwallets,
        &cancel,
        futures::stream::iter(vec![Ok(first), Ok(last)]),
    )
    .await;

    let floor = |store: &FileBackedShieldedStore| {
        store
            .with_token_pool_conn(|conn| raise_sweep_height(conn, 0))
            .unwrap()
    };
    {
        let store = store.read().await;
        assert!(
            store.pending_redrives(sub).unwrap().is_empty(),
            "the fee note seen spent dropped the record"
        );
        assert_eq!(
            floor(&store),
            1_011,
            "the highest batch height, not the last"
        );
    }
    drop(store);
    let reopened = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
    assert_eq!(floor(&reopened), 1_011, "kept across a restart");
    let _ = std::fs::remove_file(&path);
}

/// The spend-scan height is kept before anything the pass marks: when that
/// write fails, the pass fails with the fee note still unspent and the saved
/// spend's record still there, so a retry sees the spend again.
#[tokio::test]
async fn should_mark_nothing_spent_when_the_spend_scan_height_is_not_kept() {
    use crate::wallet::shielded::store::{PendingRedrive, ShieldedNote};
    use crate::wallet::shielded::token_pool::TokenPoolConnection;

    let path = temp_tree_path("spend_floor_fails");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let sub = subwallets[0].0;
    let fee_nf = [0xAA; 32];
    {
        let mut store = store.write().await;
        store
            .save_note(
                sub,
                &ShieldedNote {
                    position: 0,
                    cmx: [0x11; 32],
                    nullifier: fee_nf,
                    block_height: 10,
                    is_spent: false,
                    value: 1_000,
                    note_data: vec![0u8; 115],
                },
            )
            .unwrap();
        store
            .arm_redrive(
                sub,
                PendingRedrive {
                    activity_id: [0xA1; 32],
                    anchor: [0xA2; 32],
                    nullifiers: vec![fee_nf],
                    st_bytes: vec![0],
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                    anchor_height: Some(1),
                },
            )
            .unwrap();
        // The height write fails from here on.
        store
            .with_token_pool_conn(|conn| {
                conn.execute("DROP TABLE shielded_token_pool_sweep", [])
                    .expect("drop the sweep table");
                Ok(())
            })
            .unwrap();
    }
    let cancel = CancellationToken::new();
    let epoch = StoreEpoch::default();
    let pass = sync_notes_from_stream(
        &store,
        &epoch.watch(),
        &subwallets,
        None,
        &cancel,
        |_, _| futures::stream::iter(vec![Ok(batch(0, 4, 4))]),
    )
    .await;
    assert!(pass.is_err(), "the pass fails when the height is not kept");

    let store = store.read().await;
    assert!(
        store
            .get_unspent_notes(sub)
            .unwrap()
            .iter()
            .any(|note| note.nullifier == fee_nf),
        "the fee note is not marked spent"
    );
    assert_eq!(
        store
            .pending_redrives(sub)
            .unwrap()
            .into_iter()
            .map(|record| record.activity_id)
            .collect::<Vec<_>>(),
        vec![[0xA1; 32]],
        "the saved spend's record is still there"
    );
    drop(store);
    let _ = std::fs::remove_file(&path);
}

/// A pass that appended leaves and then failed before its checkpoint (a
/// batch error here; a process killed at the same point leaves the same
/// tree) must not leave those leaves past the latest checkpoint for good.
/// The next pass, after a relaunch, gate-skips them and appends nothing;
/// it still checkpoints, so a note saved at one of them can be witnessed
/// at depth 0 instead of failing the spend with
/// `ShieldedMerkleWitnessUnavailable`.
#[tokio::test]
async fn should_checkpoint_leaves_a_failed_pass_appended_on_the_next_pass() {
    let path = temp_tree_path("failed_then_resume");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();

    // Pass 1: a fully checkpointed first batch, then a pass that appends a
    // second batch and fails on the next one, before its checkpoint.
    let cancel = CancellationToken::new();
    run(
        &store,
        &subwallets,
        &cancel,
        futures::stream::iter(vec![batch(0, 4, 4)].into_iter().map(Ok)),
    )
    .await
    .finished();
    let failed = sync_notes_from_stream(
        &store,
        &StoreEpoch::default().watch(),
        &subwallets,
        None,
        &cancel,
        |_, _| {
            futures::stream::iter(vec![
                Ok(batch(0, 4, 12)),
                Ok(batch(4, 4, 12)),
                Err(dash_sdk::Error::Generic("transport failed".to_string())),
            ])
        },
    )
    .await;
    assert!(failed.is_err(), "the batch error fails the pass");
    {
        let store = store.read().await;
        assert_eq!(store.tree_size().unwrap(), 8, "the appends are persisted");
        assert!(
            matches!(store.witness(7), Ok(None)),
            "no checkpoint covers the leaves the failed pass appended"
        );
    }

    // Relaunch: reopen the same file, then run a pass that re-delivers only
    // positions the tree already holds.
    drop(store);
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("reopen store"),
    ));
    run(
        &store,
        &subwallets,
        &cancel,
        futures::stream::iter(vec![batch(0, 4, 8), batch(4, 4, 8)].into_iter().map(Ok)),
    )
    .await
    .finished();

    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 8, "nothing is re-appended");
    assert!(
        matches!(store.witness(7), Ok(Some(_))),
        "a pass that appended nothing still checkpoints the tree's leaves"
    );

    let _ = std::fs::remove_file(&path);
}

/// A pass over a tree whose latest checkpoint already sits at its size
/// keeps that checkpoint: the witnesses at depth 0 and depth 1 are the ones
/// before the pass.
#[tokio::test]
async fn should_keep_the_checkpoints_when_nothing_is_past_the_latest() {
    let path = temp_tree_path("no_new_leaves");
    let store = Arc::new(RwLock::new(
        FileBackedShieldedStore::open_path(&path, 100).expect("open store"),
    ));
    let subwallets = one_subwallet();
    let cancel = CancellationToken::new();
    for stream in [vec![batch(0, 4, 4)], vec![batch(0, 4, 8), batch(4, 4, 8)]] {
        run(
            &store,
            &subwallets,
            &cancel,
            futures::stream::iter(stream.into_iter().map(Ok)),
        )
        .await
        .finished();
    }
    let before = {
        let store = store.read().await;
        (
            store
                .witness_at_depth(3, 0)
                .unwrap()
                .map(|p| format!("{p:?}")),
            store
                .witness_at_depth(3, 1)
                .unwrap()
                .map(|p| format!("{p:?}")),
        )
    };

    run(
        &store,
        &subwallets,
        &cancel,
        futures::stream::iter(vec![batch(0, 4, 8), batch(4, 4, 8)].into_iter().map(Ok)),
    )
    .await
    .finished();

    let store = store.read().await;
    let after = (
        store
            .witness_at_depth(3, 0)
            .unwrap()
            .map(|p| format!("{p:?}")),
        store
            .witness_at_depth(3, 1)
            .unwrap()
            .map(|p| format!("{p:?}")),
    );
    assert!(before.0.is_some() && before.1.is_some());
    assert_eq!(before, after, "no checkpoint is added or moved");

    let _ = std::fs::remove_file(&path);
}

/// The in-memory store honours the same `checkpoint_tree` contract as the
/// file-backed one: a pass that appends nothing re-uses the tree size as its
/// checkpoint identifier, and that adds no checkpoint.
#[tokio::test]
async fn should_add_no_checkpoint_on_an_idle_pass_over_the_in_memory_store() {
    let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
    let subwallets = one_subwallet();
    let cancel = CancellationToken::new();
    for _ in 0..2 {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            sync_notes_from_stream(
                &store,
                &StoreEpoch::default().watch(),
                &subwallets,
                None,
                &cancel,
                |_, _| futures::stream::iter(vec![batch(0, 4, 4)].into_iter().map(Ok)),
            ),
        )
        .await
        .expect("the pass returns")
        .expect("pass must succeed")
        .finished();
    }

    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 4);
    assert_eq!(
        store.checkpoint_ids(),
        &[4],
        "the idle second pass adds no checkpoint"
    );
}
