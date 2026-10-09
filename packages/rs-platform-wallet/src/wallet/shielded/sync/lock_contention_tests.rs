//! Store-lock behaviour of a sync pass: short wallet operations (sends,
//! reservations, activity writes) must not wait on the pass's network I/O,
//! and a pass whose snapshot a concurrent writer invalidated must abandon
//! its commit rather than write over that change.
//!
//! Every test drives `sync_notes_from_source` with a hand-fed stream, so a
//! pass can be held "mid-download" for as long as the test needs.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dash_sdk::platform::shielded::notes_sync::types::{DecryptedNote, ShieldedChunkBatch};
use drive_proof_verifier::types::ShieldedEncryptedNote;
use futures::channel::mpsc;
use futures::StreamExt;
use grovedb_commitment_tree::{ExtractedNoteCommitment, Note, NoteValue, RandomSeed, Rho};
use rand::rngs::OsRng;
use rand::RngCore;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;

use super::{sync_notes_from_source, NoteSyncOutcome, StoreEpoch};
use crate::error::PlatformWalletError;
use crate::wallet::shielded::coordinator::ShieldedTreeProgressCallback;
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::store::{ShieldedNote, ShieldedStore, SubwalletId};

/// How long the fake network withholds the next batch.
const STREAM_STALL: Duration = Duration::from_millis(1500);
/// Bound on a store-lock wait while the pass is stalled on the network.
/// Before the fix the wait was the whole stall.
const LOCK_WAIT_BOUND: Duration = Duration::from_millis(250);

/// Bound on taking the store lock while a pass is parked on the network,
/// where the lock should be free. A pass holding it across `stream.next()`
/// would otherwise deadlock the test feeding that stream instead of failing.
const MID_PASS_LOCK_BOUND: Duration = Duration::from_secs(5);

type Store = Arc<RwLock<FileBackedShieldedStore>>;

async fn mid_pass_write(store: &Store) -> RwLockWriteGuard<'_, FileBackedShieldedStore> {
    tokio::time::timeout(MID_PASS_LOCK_BOUND, store.write())
        .await
        .expect("sync held the store lock while waiting for the next batch")
}

async fn mid_pass_read(store: &Store) -> RwLockReadGuard<'_, FileBackedShieldedStore> {
    tokio::time::timeout(MID_PASS_LOCK_BOUND, store.read())
        .await
        .expect("sync held the store lock while waiting for the next batch")
}

type BatchSender = mpsc::UnboundedSender<Result<ShieldedChunkBatch, dash_sdk::Error>>;
type PassHandle = JoinHandle<Result<NoteSyncOutcome, PlatformWalletError>>;

const WALLET: [u8; 32] = [0x5A; 32];

fn subwallet() -> SubwalletId {
    SubwalletId::new(WALLET, 0)
}

fn temp_tree_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("sync_lock_{tag}_{nanos}.sqlite"))
}

/// Removes the test's SQLite tree on drop, panics included.
struct TempTree(std::path::PathBuf);

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A fresh file-backed store plus the epoch its passes watch. Keep the
/// `TempTree` alive for the whole test.
fn open_store(
    tag: &str,
) -> (
    Arc<RwLock<FileBackedShieldedStore>>,
    Arc<StoreEpoch>,
    TempTree,
) {
    let path = temp_tree_path(tag);
    let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
    (
        Arc::new(RwLock::new(store)),
        Arc::new(StoreEpoch::default()),
        TempTree(path),
    )
}

/// A canonical (small) Pallas base-field element per tree position, so the
/// commitment is a valid `ExtractedNoteCommitment` and witnesses can be
/// turned into roots.
fn cmx(position: u64) -> [u8; 32] {
    let mut c = [0u8; 32];
    c[..8].copy_from_slice(&(position + 1).to_le_bytes());
    c
}

/// A batch of `len` undecryptable notes at `start_index..`, each carrying a
/// distinct nullifier, as the SDK stream would yield it.
fn batch(start_index: u64, len: u64) -> ShieldedChunkBatch {
    ShieldedChunkBatch {
        start_index,
        notes: (start_index..start_index + len)
            .map(|p| ShieldedEncryptedNote {
                cmx: cmx(p).to_vec(),
                nullifier: {
                    let mut nf = [0xEE; 32];
                    nf[..8].copy_from_slice(&p.to_le_bytes());
                    nf.to_vec()
                },
                cv_net: Vec::new(),
                encrypted_note: Vec::new(),
            })
            .collect(),
        decrypted: Vec::new(),
        block_height: 7,
        is_partial: false,
        total_count: start_index + len,
    }
}

/// Start a pass over `store` fed by the returned sender. The returned
/// receiver yields the tree-progress numerator once per batch, right after
/// the batch's commitments are appended; on the current-thread test runtime
/// the pass then finishes the batch (trial decryption) and parks on the
/// "network" before the test task runs again.
fn start_pass(
    store: &Arc<RwLock<FileBackedShieldedStore>>,
    epoch: &Arc<StoreEpoch>,
) -> (BatchSender, mpsc::UnboundedReceiver<u64>, PassHandle) {
    let views = OrchardKeySet::from_seed(&[0x42; 64], dashcore::Network::Testnet, 0)
        .unwrap()
        .viewing_keys();
    let subwallets = vec![(subwallet(), views)];
    let (tx, rx) = mpsc::unbounded();
    let (progress_tx, progress_rx) = mpsc::unbounded();
    let on_tree: ShieldedTreeProgressCallback = Arc::new(move |committed, _| {
        let _ = progress_tx.unbounded_send(committed);
    });
    let store = Arc::clone(store);
    let epoch = Arc::clone(epoch);
    let pass = tokio::spawn(async move {
        let watch = epoch.watch();
        sync_notes_from_source(&store, &watch, &subwallets, Some(&on_tree), move |_, _| rx).await
    });
    (tx, progress_rx, pass)
}

fn expect_completed(outcome: NoteSyncOutcome) -> super::MultiSyncNotesResult {
    match outcome {
        NoteSyncOutcome::Completed(result) => result,
        NoteSyncOutcome::Superseded => panic!("pass was unexpectedly superseded"),
    }
}

fn expect_superseded(outcome: NoteSyncOutcome) {
    assert!(
        matches!(outcome, NoteSyncOutcome::Superseded),
        "pass should have been superseded, got {outcome:?}"
    );
}

/// The latency regression: a send's store work — reserving notes, probing
/// the anchor/witnesses — must not wait for a sync pass's next network
/// batch. Before the fix the pass held the store write lock across
/// `stream.next().await`, so this wait equalled the whole network stall.
#[tokio::test]
async fn should_not_hold_store_lock_while_sync_waits_for_network() {
    let (store, epoch, _tree) = open_store("contention");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);

    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));

    // The "network" now stalls before the next batch.
    let release = tokio::spawn(async move {
        tokio::time::sleep(STREAM_STALL).await;
        tx.unbounded_send(Ok(batch(2048, 100))).unwrap();
    });

    let started = Instant::now();
    {
        // A send reserving its notes (operations.rs `reserve_notes`).
        let mut guard = store.write().await;
        let waited = started.elapsed();
        guard.mark_pending(subwallet(), &[1; 32]).unwrap();
        eprintln!("send: store write-lock wait while sync stalled on network: {waited:?}");
        assert!(
            waited < LOCK_WAIT_BOUND,
            "store write lock waited {waited:?} behind a sync pass stalled on the network"
        );
    }
    let started = Instant::now();
    {
        // A send's anchor/witness probe (operations.rs
        // `extract_spends_and_anchor`).
        let guard = store.read().await;
        let waited = started.elapsed();
        guard.tree_size().unwrap();
        eprintln!("send: store read-lock wait while sync stalled on network: {waited:?}");
        assert!(waited < LOCK_WAIT_BOUND, "read lock waited {waited:?}");
    }

    release.await.unwrap();
    assert_eq!(progress.next().await, Some(2148));
    drop(progress);
    // `release` dropped the sender, ending the stream.
    let result = expect_completed(pass.await.unwrap().unwrap());
    assert_eq!(result.total_scanned, 2148);

    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 2148);
    assert_eq!(store.last_synced_note_index(subwallet()).unwrap(), 2148);
    assert!(
        store.witness_at_depth(2147, 0).unwrap().is_some(),
        "the pass checkpoints the tree at its final size on commit"
    );
}

/// The worst-case store-lock wait a send sees while a pass ingests a large
/// catch-up batch: bounded by one `APPEND_SLICE` of appends, not by the
/// batch (let alone the download).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_bound_store_lock_hold_while_appending_a_large_batch() {
    let (store, epoch, _tree) = open_store("append_slices");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);

    // The prober queues for the store lock back to back from its own OS
    // thread, so it is always holding the lock or waiting for it, whatever
    // the async scheduler does. tokio's RwLock is fair: a prober waiting
    // while the pass holds one slice's lock gets it before the pass's next
    // slice, and records the partly appended tree it sees there.
    let done = StopOnDrop(Arc::new(std::sync::atomic::AtomicBool::new(false)));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let prober = {
        let store = Arc::clone(&store);
        let done = Arc::clone(&done.0);
        std::thread::spawn(move || {
            let mut ready = Some(ready_tx);
            let mut probes = Vec::new();
            while !done.load(std::sync::atomic::Ordering::Acquire) {
                let started = Instant::now();
                let guard = store.blocking_write();
                probes.push((started.elapsed(), guard.tree_size().unwrap()));
                drop(guard);
                if let Some(ready) = ready.take() {
                    let _ = ready.send(());
                }
            }
            probes
        })
    };
    ready_rx
        .recv_timeout(MID_PASS_LOCK_BOUND)
        .expect("lock prober could not acquire the store before the first batch");

    let batch_started = Instant::now();
    tx.unbounded_send(Ok(batch(0, 4 * 2048))).unwrap();
    assert_eq!(progress.next().await, Some(4 * 2048));
    let batch_elapsed = batch_started.elapsed();
    drop(done);
    drop(tx);
    expect_completed(pass.await.unwrap().unwrap());

    let probes = prober.join().unwrap();
    let mid_ingest: BTreeSet<u64> = probes
        .iter()
        .map(|(_, size)| *size)
        .filter(|size| (1..4 * 2048).contains(size))
        .collect();
    // The batch spans 32 append slices. A hold per batch would leave the
    // tree empty or full at every probe; a hold per handful of slices would
    // show only a few distinct sizes.
    assert!(
        mid_ingest.len() >= 8,
        "probes saw only {mid_ingest:?} mid-ingest: {probes:?}"
    );
    let max_wait = probes.iter().map(|(wait, _)| *wait).max().unwrap();
    eprintln!(
        "8192-leaf batch ingested in {batch_elapsed:?}; {} lock probes, {} distinct \
         mid-ingest sizes, max wait {max_wait:?}",
        probes.len(),
        mid_ingest.len()
    );
    assert!(
        max_wait < batch_elapsed / 2,
        "a probe waited {max_wait:?} of a {batch_elapsed:?} batch ingest"
    );
}

/// Stops the prober thread when dropped, so a failed assertion doesn't
/// leave it contending for the store under later tests.
struct StopOnDrop(Arc<std::sync::atomic::AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Appends a pass makes between lock holds are invisible to a send: the
/// spend path witnesses only at checkpoints, and shardtree computes a
/// checkpoint-depth witness as of that checkpoint's position. So a send
/// that probes mid-pass gets exactly the anchor it would have got before
/// the pass started, and after the pass commits that anchor is still
/// reachable one checkpoint deeper.
#[tokio::test]
async fn should_keep_spend_witnesses_stable_while_a_pass_is_mid_download() {
    let (store, epoch, _tree) = open_store("witness_stable");
    let note_cmx = ExtractedNoteCommitment::from_bytes(&cmx(0)).unwrap();
    {
        let mut guard = store.write().await;
        for p in 0..3 {
            guard.append_commitment(&cmx(p), true).unwrap();
        }
        guard.checkpoint_tree(3).unwrap();
    }
    let root_before = {
        let guard = store.read().await;
        guard
            .witness_at_depth(0, 0)
            .unwrap()
            .unwrap()
            .root(note_cmx)
    };

    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));
    {
        let guard = mid_pass_read(&store).await;
        assert_eq!(guard.tree_size().unwrap(), 2048, "batch appended mid-pass");
        let mid_pass = guard
            .witness_at_depth(0, 0)
            .unwrap()
            .unwrap()
            .root(note_cmx);
        assert_eq!(
            mid_pass.to_bytes(),
            root_before.to_bytes(),
            "uncheckpointed mid-pass appends must not move the depth-0 anchor"
        );
    }

    drop(tx);
    expect_completed(pass.await.unwrap().unwrap());
    let guard = store.read().await;
    let depth0 = guard
        .witness_at_depth(0, 0)
        .unwrap()
        .unwrap()
        .root(note_cmx);
    let depth1 = guard
        .witness_at_depth(0, 1)
        .unwrap()
        .unwrap()
        .root(note_cmx);
    assert_ne!(depth0.to_bytes(), root_before.to_bytes());
    assert_eq!(depth1.to_bytes(), root_before.to_bytes());
}

/// A lifecycle change landing mid-download (here `unregister_wallet`'s
/// purge, which bumps the epoch under its store guard) supersedes the
/// pass: it appends nothing more and commits no watermark — which would
/// otherwise resurrect sync state for the wallet that was just removed.
#[tokio::test]
async fn should_supersede_pass_when_a_lifecycle_change_lands_mid_download() {
    let (store, epoch, _tree) = open_store("superseded_purge");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));

    {
        let mut guard = mid_pass_write(&store).await;
        epoch.bump_wallet(&mut guard, WALLET);
        guard.purge_wallet(WALLET).unwrap();
    }
    tx.unbounded_send(Ok(batch(2048, 100))).unwrap();
    drop(tx);

    expect_superseded(pass.await.unwrap().unwrap());
    let guard = store.read().await;
    assert_eq!(
        guard.tree_size().unwrap(),
        2048,
        "no append after the change"
    );
    assert_eq!(
        guard.last_synced_note_index(subwallet()).unwrap(),
        0,
        "the purged wallet's watermark must not be resurrected"
    );
    assert!(
        guard.witness_at_depth(0, 0).unwrap().is_none(),
        "a superseded pass must not checkpoint"
    );
}

/// Clear resets the tree mid-download; the pass must not commit (or append)
/// on top of the emptied tree even when the change lands after the last
/// batch, right before the final commit.
#[tokio::test]
async fn should_supersede_pass_when_tree_is_reset_before_commit() {
    let (store, epoch, _tree) = open_store("superseded_clear");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));

    {
        let mut guard = mid_pass_write(&store).await;
        epoch.bump_all(&mut guard);
        guard.reset_commitment_tree().unwrap();
        guard.purge_all_subwallets().unwrap();
    }
    drop(tx);

    expect_superseded(pass.await.unwrap().unwrap());
    let guard = store.read().await;
    assert_eq!(guard.tree_size().unwrap(), 0);
    assert_eq!(guard.last_synced_note_index(subwallet()).unwrap(), 0);
}

/// Backstop for writers that don't bump the epoch: if the tree's leaf count
/// is not exactly what the pass itself appended, the pass stops before
/// appending at positions that are no longer the ones it fetched (which
/// would duplicate or misplace leaves and corrupt every later witness).
#[tokio::test]
async fn should_supersede_pass_when_another_writer_moves_the_tree() {
    let (store, epoch, _tree) = open_store("superseded_foreign");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));

    mid_pass_write(&store)
        .await
        .append_commitment(&cmx(9_999), true)
        .unwrap();
    tx.unbounded_send(Ok(batch(2048, 100))).unwrap();
    drop(tx);

    expect_superseded(pass.await.unwrap().unwrap());
    let guard = store.read().await;
    assert_eq!(guard.tree_size().unwrap(), 2049, "no append after the move");
    assert_eq!(guard.last_synced_note_index(subwallet()).unwrap(), 0);
}

/// A change scoped to another wallet (its restore, removal, or account
/// purge) does not supersede a pass that isn't scanning that wallet.
#[tokio::test]
async fn should_not_supersede_pass_for_another_wallets_lifecycle_change() {
    let (store, epoch, _tree) = open_store("unrelated_wallet");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));
    {
        let mut guard = mid_pass_write(&store).await;
        epoch.bump_wallet(&mut guard, [0x77; 32]);
        guard.purge_wallet([0x77; 32]).unwrap();
    }
    tx.unbounded_send(Ok(batch(2048, 100))).unwrap();
    drop(tx);
    expect_completed(pass.await.unwrap().unwrap());
    assert_eq!(
        store
            .read()
            .await
            .last_synced_note_index(subwallet())
            .unwrap(),
        2148
    );
}

/// A superseded pass leaves only uncheckpointed leaves behind — the state an
/// interrupted pass always could. The retry resumes from the unchanged
/// watermark and gate-skips those leaves; even when the chain has not moved
/// (so the retry appends nothing itself) it must checkpoint them, or the
/// notes it saves there would be unwitnessable at spend time.
#[tokio::test]
async fn should_checkpoint_leaves_left_by_a_superseded_pass_on_retry() {
    let (store, epoch, _tree) = open_store("retry");
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    assert_eq!(progress.next().await, Some(2048));
    {
        // E.g. a host snapshot restore of the wallet being scanned.
        let mut guard = mid_pass_write(&store).await;
        epoch.bump_wallet(&mut guard, WALLET);
    }
    drop(tx);
    expect_superseded(pass.await.unwrap().unwrap());
    assert!(
        store.read().await.witness_at_depth(0, 0).unwrap().is_none(),
        "superseded pass left its leaves uncheckpointed"
    );

    // Retry over an unchanged chain: the same 2048 leaves, nothing new.
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    drop(tx);
    assert_eq!(
        progress.next().await,
        Some(2048),
        "re-fetched leaves skipped"
    );
    expect_completed(pass.await.unwrap().unwrap());

    let guard = store.read().await;
    assert_eq!(guard.tree_size().unwrap(), 2048, "no leaf duplicated");
    assert_eq!(guard.last_synced_note_index(subwallet()).unwrap(), 2048);
    let first = guard.witness_at_depth(0, 0).unwrap().expect("witnessable");
    let last = guard
        .witness_at_depth(2047, 0)
        .unwrap()
        .expect("witnessable");
    let cmx_at = |p| ExtractedNoteCommitment::from_bytes(&cmx(p)).unwrap();
    assert_eq!(
        first.root(cmx_at(0)).to_bytes(),
        last.root(cmx_at(2047)).to_bytes(),
        "every position witnesses against the same committed root"
    );
    drop(guard);

    // A further caught-up pass re-checkpoints at the same size: a no-op.
    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    tx.unbounded_send(Ok(batch(0, 2048))).unwrap();
    drop(tx);
    assert_eq!(progress.next().await, Some(2048));
    expect_completed(pass.await.unwrap().unwrap());
    let guard = store.read().await;
    assert!(
        guard.witness_at_depth(0, 1).unwrap().is_none(),
        "an unchanged tree must not gain a second checkpoint"
    );
}

/// A real Orchard note to `keys`' default address, plus its nullifier.
fn owned_note(keys: &OrchardKeySet, value: u64) -> (Note, [u8; 32]) {
    let mut rng = OsRng;
    let rho = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rho) = Rho::from_bytes(&b).into_option() {
            break rho;
        }
    };
    let rseed = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rseed) = RandomSeed::from_bytes(b, &rho).into_option() {
            break rseed;
        }
    };
    let note = Note::from_parts(keys.address_at(0), NoteValue::from_raw(value), rho, rseed)
        .into_option()
        .expect("valid note parts");
    let nullifier = note.nullifier(&keys.full_viewing_key).to_bytes();
    (note, nullifier)
}

/// A send can confirm a note spent while a pass is downloading. When the
/// pass re-scans that note (here: a restored notes-only snapshot left the
/// watermark at 0) from scan data older than the spend, committing the
/// receipt must keep it spent, not resurrect it as selectable balance.
#[tokio::test]
async fn should_keep_a_spend_confirmed_mid_download_when_rescanning_its_note() {
    let (store, epoch, _tree) = open_store("rescan_spent");
    let keys = OrchardKeySet::from_seed(&[0x42; 64], dashcore::Network::Testnet, 0).unwrap();
    let (note, nullifier) = owned_note(&keys, 1_000);
    let position = 5;
    let note_cmx = ExtractedNoteCommitment::from(note.commitment()).to_bytes();
    store
        .write()
        .await
        .save_note(
            subwallet(),
            &ShieldedNote {
                note_data: super::serialize_note(&note),
                position,
                cmx: note_cmx,
                nullifier,
                block_height: 7,
                is_spent: false,
                value: 1_000,
            },
        )
        .unwrap();

    let (tx, mut progress, pass) = start_pass(&store, &epoch);
    let mut scanned = batch(0, 2048);
    scanned.decrypted.push(DecryptedNote {
        position,
        note,
        address: keys.address_at(0),
        nullifier,
        cmx: note_cmx,
    });
    tx.unbounded_send(Ok(scanned)).unwrap();
    assert_eq!(progress.next().await, Some(2048));

    {
        // The send's reservation and its confirmation (`mark_notes_spent`).
        let mut guard = mid_pass_write(&store).await;
        guard.mark_pending(subwallet(), &nullifier).unwrap();
        assert!(guard.mark_spent(subwallet(), &nullifier).unwrap());
    }
    drop(tx);
    let result = expect_completed(pass.await.unwrap().unwrap());

    let guard = store.read().await;
    let stored = guard.get_all_notes(subwallet()).unwrap();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].is_spent, "the confirmed spend was overwritten");
    assert!(guard.get_unspent_notes(subwallet()).unwrap().is_empty());
    assert_eq!(guard.spendable_balance(subwallet()).unwrap(), 0);
    let emitted = &result.changeset.notes_saved[&subwallet()];
    assert!(
        emitted
            .iter()
            .all(|n| n.nullifier != nullifier || n.is_spent),
        "the changeset must not persist an unspent replacement"
    );
}
