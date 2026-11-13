//! A cancelled scan pass publishes nothing (dashpay/platform#5200 review).
//!
//! Drives the real coordinator against a mock Platform whose pool spans
//! two fetch chunks, each holding one owned note, and cancels the pass
//! from the tree-progress callback after the first chunk lands. The
//! appended commitments stay in the local tree; no note, spent mark,
//! watermark, activity entry or completion leaves the scan, and the
//! ledger stays unavailable. The next pass re-walks from the completed
//! watermark and yields exactly what an uninterrupted scan yields — one
//! entry carrying both notes.
//!
//! The host side is checked through a recording persister: nothing
//! reaches the host on a cancelled pass, and a restart restored from
//! what the host recorded carries no scan coverage until a pass
//! completes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use dash_sdk::Sdk;
use drive_proof_verifier::types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery,
};
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, Nullifier, OrchardDomain,
    OrchardNoteEncryption, RandomSeed, Rho, ValueCommitTrapdoor, ValueCommitment,
};
use rand::{rngs::OsRng, RngCore};
use tokio_util::sync::CancellationToken;

use super::activity::ShieldedActivityEntry;
use super::balance::{ShieldedBalanceSource, ShieldedLocalBalanceState};
use super::keys::AccountViewingKeys;
use super::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedStore, SubwalletId,
};
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ShieldedChangeSet, ShieldedSubwalletStartState, ShieldedSyncStartState,
};
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

pub(super) const WALLET: [u8; 32] = [7; 32];
const FIRST_VALUE: u64 = 100;
const SECOND_VALUE: u64 = 200;

/// A host persister that keeps every shielded changeset it was handed,
/// and can hand them back as the start state a cold bind would restore.
#[derive(Default)]
pub(super) struct RecordingPersistence {
    changesets: Mutex<Vec<ShieldedChangeSet>>,
}

impl PlatformWalletPersistence for RecordingPersistence {
    fn store(
        &self,
        _wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(shielded) = changeset.shielded {
            self.changesets.lock().expect("recorder").push(shielded);
        }
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

impl RecordingPersistence {
    fn recorded(&self) -> Vec<ShieldedChangeSet> {
        self.changesets.lock().expect("recorder").clone()
    }

    /// Fold the recorded changesets the way a host snapshot does: notes
    /// accumulate, a spent nullifier marks its note spent, the watermark
    /// is the highest one received, and sync state exists only if a
    /// watermark was ever received.
    fn start_state(&self) -> ShieldedSyncStartState {
        let mut per_subwallet: BTreeMap<SubwalletId, ShieldedSubwalletStartState> = BTreeMap::new();
        for changeset in self.recorded() {
            for (id, notes) in changeset.notes_saved {
                per_subwallet.entry(id).or_default().notes.extend(notes);
            }
            for (id, nullifiers) in changeset.nullifiers_spent {
                for note in &mut per_subwallet.entry(id).or_default().notes {
                    if nullifiers.contains(&note.nullifier) {
                        note.is_spent = true;
                    }
                }
            }
            for (id, index) in changeset.synced_indices {
                let sub = per_subwallet.entry(id).or_default();
                sub.has_sync_state = true;
                sub.last_synced_index = sub.last_synced_index.max(index);
            }
            for (id, entries) in changeset.activity_entries {
                per_subwallet
                    .entry(id)
                    .or_default()
                    .activity
                    .extend(entries);
            }
        }
        ShieldedSyncStartState {
            per_subwallet,
            ..Default::default()
        }
    }
}

fn tree_path() -> PathBuf {
    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "shielded_cancelled_scan_{}_{nonce}_{sequence}.sqlite",
        std::process::id()
    ))
}

fn viewing_keys() -> AccountViewingKeys {
    OrchardKeySet::from_seed(&[42; 64], dashcore::Network::Testnet, 0)
        .expect("test viewing key")
        .viewing_keys()
}

/// A note of `value` sent to the test account by someone else, as the
/// network serves it. Real IVK encryption so the SDK's driver decryption
/// finds it; no OVK, so it is a plain receipt and not a self-send; no
/// proof, since nothing here is admitted to consensus.
fn owned_note(keys: &AccountViewingKeys, value: u64) -> ShieldedEncryptedNote {
    owned_note_with_nullifier(keys, value).0
}

/// [`owned_note`] plus the nullifier the account derives for it: the
/// value a later action carries when it spends this note.
fn owned_note_with_nullifier(
    keys: &AccountViewingKeys,
    value: u64,
) -> (ShieldedEncryptedNote, [u8; 32]) {
    let (wire, note) = note_parts_to(keys.default_address, value);
    (wire, note.nullifier(&keys.full_viewing_key).to_bytes())
}

/// A note of `value` sent to `recipient` by someone else, as the network
/// serves it — the same wire form as [`owned_note`], for any address.
pub(super) fn note_to(
    recipient: grovedb_commitment_tree::PaymentAddress,
    value: u64,
) -> ShieldedEncryptedNote {
    note_parts_to(recipient, value).0
}

fn note_parts_to(
    recipient: grovedb_commitment_tree::PaymentAddress,
    value: u64,
) -> (ShieldedEncryptedNote, Note) {
    let mut rng = OsRng;
    let (nf, rho) = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let (Some(nf), Some(rho)) = (
            Nullifier::from_bytes(&b).into_option(),
            Rho::from_bytes(&b).into_option(),
        ) {
            break (nf, rho);
        }
    };
    let rseed = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rseed) = RandomSeed::from_bytes(b, &rho).into_option() {
            break rseed;
        }
    };
    let value = NoteValue::from_raw(value);
    let note = Note::from_parts(recipient, value, rho, rseed)
        .into_option()
        .expect("valid note parts");
    let cmx = ExtractedNoteCommitment::from(note.commitment());
    let rcv = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rcv) = ValueCommitTrapdoor::from_bytes(b).into_option() {
            break rcv;
        }
    };
    let cv = ValueCommitment::derive(value - NoteValue::from_raw(0), rcv);
    let ne = OrchardNoteEncryption::<DashMemo>::new(None, note, [0; 36]);
    let epk = OrchardDomain::<DashMemo>::epk_bytes(ne.epk());
    let enc = ne.encrypt_note_plaintext();
    let out = ne.encrypt_outgoing_plaintext(&cv, &cmx, &mut rng);
    let mut encrypted_note = Vec::with_capacity(216);
    encrypted_note.extend_from_slice(&epk.0);
    encrypted_note.extend_from_slice(enc.as_ref());
    encrypted_note.extend_from_slice(&out);
    (
        ShieldedEncryptedNote {
            cmx: cmx.to_bytes().to_vec(),
            nullifier: nf.to_bytes().to_vec(),
            cv_net: cv.to_bytes().to_vec(),
            encrypted_note,
        },
        note,
    )
}

/// A note nobody owns at tree position `pos`: an empty ciphertext never
/// decrypts or OVK-recovers, and a distinct small commitment is a valid
/// Pallas element the tree can append.
fn opaque_note(pos: u64) -> ShieldedEncryptedNote {
    let mut cmx = [0u8; 32];
    cmx[0] = pos as u8;
    cmx[1] = (pos >> 8) as u8;
    cmx[2] = 1;
    ShieldedEncryptedNote {
        cmx: cmx.to_vec(),
        nullifier: vec![0xAA; 32],
        cv_net: vec![],
        encrypted_note: vec![],
    }
}

/// A pool that spans two fetch chunks: the first is full and opens with
/// the first owned note, the second holds only the second owned note.
/// Both chunks are proven at the mock's one block height, so the two
/// receipts fall into the same activity cluster.
async fn two_chunk_pool(
    first: &ShieldedEncryptedNote,
    second: &ShieldedEncryptedNote,
) -> (Arc<Sdk>, u64) {
    chunked_pool(&[vec![first.clone()], vec![second.clone()]]).await
}

/// A pool of full chunks, each opening with the given notes and padded
/// with opaque ones, except the last chunk, which holds its notes alone
/// and so ends the pool. Returns the SDK and the chunk size.
pub(super) async fn chunked_pool(leading: &[Vec<ShieldedEncryptedNote>]) -> (Arc<Sdk>, u64) {
    let mut sdk = Sdk::new_mock();
    let count = 2048
        * u64::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    let last = leading.len() as u64 - 1;
    let total_count = count * last + leading[last as usize].len() as u64;
    // Full chunks let the stream's 16-wide speculative window slide past
    // them before the short last chunk ends the pool: cover every chunk
    // it may still request.
    for chunk in 0..(48 + 16 * last) {
        let notes: Vec<ShieldedEncryptedNote> = match leading.get(chunk as usize) {
            Some(notes) if chunk == last => notes.clone(),
            Some(notes) => notes
                .iter()
                .cloned()
                .chain((notes.len() as u64..count).map(|i| opaque_note(count * chunk + i)))
                .collect(),
            None => Vec::new(),
        };
        sdk.mock()
            .expect_fetch(
                ShieldedEncryptedNotesQuery {
                    start_index: count * chunk,
                    count: count as u32,
                },
                Some(ShieldedEncryptedNotes { notes, total_count }),
            )
            .await
            .expect("mock pool chunk");
    }
    (Arc::new(sdk), count)
}

async fn coordinator_at(
    sdk: Arc<Sdk>,
    path: &std::path::Path,
    keys: &AccountViewingKeys,
    host: Arc<RecordingPersistence>,
) -> NetworkShieldedCoordinator {
    let coordinator = NetworkShieldedCoordinator::new(
        sdk,
        dashcore::Network::Testnet,
        path.to_path_buf(),
        FileBackedShieldedStore::open_path(path, 100).expect("open test store"),
    );
    coordinator
        .register_wallet(
            WALLET,
            [(0, keys.clone())].into_iter().collect(),
            WalletPersister::new(WALLET, host),
        )
        .await
        .expect("register test wallet");
    coordinator
}

pub(super) async fn coordinator_over(
    sdk: Arc<Sdk>,
    keys: &AccountViewingKeys,
) -> (NetworkShieldedCoordinator, Arc<RecordingPersistence>) {
    let host = Arc::new(RecordingPersistence::default());
    let coordinator = coordinator_at(sdk, &tree_path(), keys, Arc::clone(&host)).await;
    (coordinator, host)
}

/// Cancel once the `after`-th chunk has been committed to the tree.
fn cancel_after(cancel: &CancellationToken, after: u64) -> Arc<dyn Fn(u64, u64) + Send + Sync> {
    let stop = cancel.clone();
    let committed = Arc::new(AtomicU64::new(0));
    Arc::new(move |_, _| {
        if committed.fetch_add(1, Ordering::SeqCst) + 1 >= after {
            stop.cancel();
        }
    })
}

/// Run one pass that is stopped after `after` chunks, bounded in time.
async fn cancelled_pass(
    coordinator: &NetworkShieldedCoordinator,
    after: u64,
) -> crate::manager::shielded_sync::ShieldedSyncPassSummary {
    let cancel = CancellationToken::new();
    coordinator.install_tree_progress_handler(Some(cancel_after(&cancel, after)));
    let summary = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        coordinator.sync(true, &cancel),
    )
    .await
    .expect("a cancelled pass returns");
    coordinator.install_tree_progress_handler(None);
    summary
}

fn host_synced_index(host: &RecordingPersistence, sub: SubwalletId) -> Option<u64> {
    host.recorded()
        .iter()
        .filter_map(|changeset| changeset.synced_indices.get(&sub).copied())
        .max()
}

pub(super) fn host_note_values(host: &RecordingPersistence, sub: SubwalletId) -> Vec<u64> {
    let mut values: Vec<u64> = host
        .recorded()
        .iter()
        .flat_map(|changeset| changeset.notes_saved.get(&sub).cloned().unwrap_or_default())
        .map(|note| note.value)
        .collect();
    values.sort_unstable();
    values
}

async fn activity(coordinator: &NetworkShieldedCoordinator) -> Vec<ShieldedActivityEntry> {
    let mut entries = coordinator
        .store()
        .read()
        .await
        .get_activity(SubwalletId::new(WALLET, 0), 0, usize::MAX)
        .expect("read activity");
    for entry in &mut entries {
        entry.note_cmxs.sort();
    }
    entries.sort_by_key(|entry| entry.id);
    entries
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_publish_nothing_when_cancelled_and_match_an_uninterrupted_scan_after() {
    let keys = viewing_keys();
    let sub = SubwalletId::new(WALLET, 0);
    let first = owned_note(&keys, FIRST_VALUE);
    let second = owned_note(&keys, SECOND_VALUE);

    // The same pool, scanned in one go: the reference history.
    let (baseline_sdk, _) = two_chunk_pool(&first, &second).await;
    let (baseline, _) = coordinator_over(baseline_sdk, &keys).await;
    let summary = baseline.sync(true, &CancellationToken::new()).await;
    assert_eq!(
        baseline
            .store()
            .read()
            .await
            .get_all_notes(sub)
            .expect("notes")
            .len(),
        2,
        "the uninterrupted scan finds both receipts: {summary:?}"
    );
    let expected = activity(&baseline).await;
    assert_eq!(expected.len(), 1, "both receipts share one cluster");
    assert_eq!(expected[0].amount, FIRST_VALUE + SECOND_VALUE);
    assert_eq!(expected[0].note_cmxs.len(), 2);

    // Pass 1: the stop lands after the first chunk is committed.
    let (sdk, count) = two_chunk_pool(&first, &second).await;
    let (coordinator, host) = coordinator_over(sdk, &keys).await;
    let summary = cancelled_pass(&coordinator, 1).await;
    assert!(
        summary.wallet_results.is_empty(),
        "a cut-short pass reports no completion: {summary:?}"
    );
    {
        let store = coordinator.store().read().await;
        assert_eq!(
            store.tree_size().expect("tree"),
            count,
            "the first chunk's commitments stay appended"
        );
        assert!(
            store.get_all_notes(sub).expect("notes").is_empty(),
            "a receipt from a consumed prefix does not enter the ledger"
        );
        assert_eq!(
            store.last_synced_note_index(sub).expect("watermark"),
            0,
            "the watermark only moves when a pass reaches the tip"
        );
        let balance = store.local_account_balance(sub).expect("balance");
        assert_eq!(balance.source, ShieldedBalanceSource::NoHistory);
        assert_eq!(balance.last_scanned_index, None);
    }
    assert!(activity(&coordinator).await.is_empty());
    assert!(host.recorded().is_empty(), "nothing reaches the host");
    assert_eq!(
        coordinator
            .local_balance_snapshot(WALLET)
            .await
            .expect("snapshot"),
        ShieldedLocalBalanceState::RestoreIncomplete,
        "an unhydrated account must not read as ready after a cut-short pass"
    );

    // Pass 2: re-walks from the completed watermark (appends are skipped
    // by the tree size) and reaches the tip.
    let summary = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(summary.success_count(), 1);
    {
        let store = coordinator.store().read().await;
        assert_eq!(store.tree_size().expect("tree"), count + 1);
        assert_eq!(store.get_all_notes(sub).expect("notes").len(), 2);
        let balance = store.local_account_balance(sub).expect("balance");
        assert_eq!(balance.source, ShieldedBalanceSource::ScannedThisSession);
        assert_eq!(balance.last_scanned_index, Some(count + 1));
        assert_eq!(balance.spendable_credits, FIRST_VALUE + SECOND_VALUE);
    }
    assert_eq!(
        activity(&coordinator).await,
        expected,
        "cancel + re-walk must derive the same history as one uninterrupted scan"
    );
    assert_eq!(
        host_note_values(&host, sub),
        vec![FIRST_VALUE, SECOND_VALUE]
    );
    assert_eq!(
        host_synced_index(&host, sub),
        Some(count + 1),
        "the completed pass is the first thing the host sees"
    );
    match coordinator
        .local_balance_snapshot(WALLET)
        .await
        .expect("snapshot")
    {
        ShieldedLocalBalanceState::Ready(snapshot) => {
            assert_eq!(
                snapshot.accounts[&0].spendable_credits,
                FIRST_VALUE + SECOND_VALUE
            );
        }
        other => panic!("expected a ready ledger, got {other:?}"),
    }
}

/// Cancel → persist → restart → restore, with no owned note in the
/// cancelled prefix (the worst case: a restored account that reads as
/// covered would read as an empty, finished scan). The host has nothing
/// to restore, the reopened tree still holds the appended prefix, and
/// the first pass after the restart finds the receipt the cancelled
/// pass never reached.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_restore_scan_coverage_from_a_cancelled_prefix() {
    let keys = viewing_keys();
    let sub = SubwalletId::new(WALLET, 0);
    // No receipt in the first chunk: make the "first" note someone else's
    // by encrypting it to a different account.
    let foreign = OrchardKeySet::from_seed(&[43; 64], dashcore::Network::Testnet, 0)
        .expect("foreign viewing key")
        .viewing_keys();
    let first = owned_note(&foreign, FIRST_VALUE);
    let second = owned_note(&keys, SECOND_VALUE);
    let path = tree_path();

    // Session 1: the pass is stopped after the first (foreign-only) chunk.
    let host = Arc::new(RecordingPersistence::default());
    {
        let (sdk, count) = two_chunk_pool(&first, &second).await;
        let coordinator = coordinator_at(sdk, &path, &keys, Arc::clone(&host)).await;
        cancelled_pass(&coordinator, 1).await;
        let store = coordinator.store().read().await;
        assert_eq!(store.tree_size().expect("tree"), count);
        assert_eq!(store.last_synced_note_index(sub).expect("cursor"), 0);
        assert!(store.get_all_notes(sub).expect("notes").is_empty());
    }
    assert!(host.recorded().is_empty());

    // Session 2: reopen the same tree file, restore what the host kept,
    // and read the ledger offline as a cold bind does.
    let (sdk, count) = two_chunk_pool(&first, &second).await;
    let coordinator = coordinator_at(sdk, &path, &keys, Arc::clone(&host)).await;
    coordinator
        .restore_for_wallet(WALLET, &host.start_state())
        .await
        .expect("restore");
    coordinator.mark_hydrated(WALLET, true).await;
    match coordinator
        .local_balance_snapshot(WALLET)
        .await
        .expect("snapshot")
    {
        ShieldedLocalBalanceState::Ready(snapshot) => {
            let account = snapshot.accounts[&0];
            assert_eq!(account.spendable_credits, 0);
            assert_eq!(
                account.last_scanned_index, None,
                "a cancelled prefix must not come back as scan coverage"
            );
            assert_eq!(account.source, ShieldedBalanceSource::NoHistory);
        }
        other => panic!("expected the hydrated ledger, got {other:?}"),
    }
    {
        let store = coordinator.store().read().await;
        assert_eq!(
            store.tree_size().expect("tree"),
            count,
            "the reopened tree keeps the appended prefix"
        );
        assert_eq!(store.last_synced_note_index(sub).expect("cursor"), 0);
    }

    // The first pass after the restart re-walks the prefix (gate-skipped
    // by the persisted tree) and reaches the receipt.
    let summary = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(summary.success_count(), 1);
    let store = coordinator.store().read().await;
    let balance = store.local_account_balance(sub).expect("balance");
    assert_eq!(balance.spendable_credits, SECOND_VALUE);
    assert_eq!(balance.last_scanned_index, Some(count + 1));
    assert_eq!(balance.source, ShieldedBalanceSource::ScannedThisSession);
    drop(store);
    assert_eq!(host_note_values(&host, sub), vec![SECOND_VALUE]);
    assert_eq!(host_synced_index(&host, sub), Some(count + 1));
    let _ = std::fs::remove_file(&path);
}

/// The receipt is in chunk 0 and the action that spends it in chunk 1.
/// Session 1 is cancelled after chunk 1 and session 2, after a restart,
/// after chunk 0 — the one order in which a published prefix would have
/// shown the receipt as spendable (receipt in, spend not yet replayed).
/// Neither stop publishes anything, so the receipt never appears
/// unspent in the ledger, in a balance or at the host; the pass that
/// reaches the tip records it spent from the start.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_never_show_a_spent_receipt_as_spendable_across_cancelled_restarts() {
    let keys = viewing_keys();
    let sub = SubwalletId::new(WALLET, 0);
    let (receipt, own_nullifier) = owned_note_with_nullifier(&keys, FIRST_VALUE);
    let mut spend = opaque_note(u64::MAX);
    spend.nullifier = own_nullifier.to_vec();
    let tail = owned_note(&keys, SECOND_VALUE);
    let layout = [vec![receipt], vec![spend], vec![tail]];
    let path = tree_path();

    // Session 1: receipt and its spend consumed, then stopped.
    let host = Arc::new(RecordingPersistence::default());
    {
        let (sdk, count) = chunked_pool(&layout).await;
        let coordinator = coordinator_at(sdk, &path, &keys, Arc::clone(&host)).await;
        cancelled_pass(&coordinator, 2).await;
        let store = coordinator.store().read().await;
        assert_eq!(store.tree_size().expect("tree"), 2 * count);
        assert!(store.get_all_notes(sub).expect("notes").is_empty());
        assert_eq!(store.spendable_balance(sub).expect("balance"), 0);
    }
    assert!(host.recorded().is_empty());

    // Session 2: restart, re-walk chunk 0 only, stop again.
    let (sdk, count) = chunked_pool(&layout).await;
    let coordinator = coordinator_at(sdk, &path, &keys, Arc::clone(&host)).await;
    coordinator
        .restore_for_wallet(WALLET, &host.start_state())
        .await
        .expect("restore");
    coordinator.mark_hydrated(WALLET, true).await;
    cancelled_pass(&coordinator, 1).await;
    {
        let store = coordinator.store().read().await;
        assert!(
            store.get_all_notes(sub).expect("notes").is_empty(),
            "a receipt whose spend was not replayed must not enter the ledger"
        );
        assert_eq!(store.spendable_balance(sub).expect("balance"), 0);
        assert_eq!(store.last_synced_note_index(sub).expect("cursor"), 0);
        assert_eq!(store.tree_size().expect("tree"), 2 * count);
    }
    assert!(host.recorded().is_empty(), "nothing reaches the host");
    match coordinator
        .local_balance_snapshot(WALLET)
        .await
        .expect("snapshot")
    {
        ShieldedLocalBalanceState::Ready(snapshot) => {
            assert_eq!(snapshot.accounts[&0].spendable_credits, 0);
        }
        other => panic!("expected the hydrated ledger, got {other:?}"),
    }

    // The pass that reaches the tip records the receipt spent from the
    // start and finds the tail.
    coordinator.sync(true, &CancellationToken::new()).await;
    let store = coordinator.store().read().await;
    let mut values: Vec<(u64, bool)> = store
        .get_all_notes(sub)
        .expect("notes")
        .iter()
        .map(|note| (note.value, note.is_spent))
        .collect();
    values.sort_unstable();
    assert_eq!(values, vec![(FIRST_VALUE, true), (SECOND_VALUE, false)]);
    assert_eq!(store.spendable_balance(sub).expect("balance"), SECOND_VALUE);
    drop(store);
    assert_eq!(
        host_note_values(&host, sub),
        vec![FIRST_VALUE, SECOND_VALUE]
    );
    assert!(host.recorded().iter().any(|changeset| changeset
        .nullifiers_spent
        .get(&sub)
        .is_some_and(|nfs| nfs.contains(&own_nullifier))));
    let _ = std::fs::remove_file(&path);
}
