//! A cancelled scan pass must not certify what it did not cover
//! (dashpay/platform#5200 review).
//!
//! Drives the real coordinator against a mock Platform whose pool spans
//! two fetch chunks, each holding one owned note, and cancels the pass
//! from the tree-progress callback after the first chunk lands. The
//! consumed prefix is flushed, but it is a resume point, not a complete
//! scan: no activity entry is derived from it and the ledger stays
//! unavailable. The resumed pass then yields exactly what an
//! uninterrupted scan yields — one entry carrying both notes.
//!
//! The host side is checked through a recording persister: the consumed
//! receipt reaches the host, the partial cursor does not, and a restart
//! restored from what the host recorded carries no scan coverage until
//! a pass completes.

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

const WALLET: [u8; 32] = [7; 32];
const FIRST_VALUE: u64 = 100;
const SECOND_VALUE: u64 = 200;

/// A host persister that keeps every shielded changeset it was handed,
/// and can hand them back as the start state a cold bind would restore.
#[derive(Default)]
struct RecordingPersistence {
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
    /// accumulate, the watermark is the highest one received, and sync
    /// state exists only if a watermark was ever received.
    fn start_state(&self) -> ShieldedSyncStartState {
        let mut per_subwallet: BTreeMap<SubwalletId, ShieldedSubwalletStartState> = BTreeMap::new();
        for changeset in self.recorded() {
            for (id, notes) in changeset.notes_saved {
                per_subwallet.entry(id).or_default().notes.extend(notes);
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
    let note = Note::from_parts(keys.default_address, value, rho, rseed)
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
    ShieldedEncryptedNote {
        cmx: cmx.to_bytes().to_vec(),
        nullifier: nf.to_bytes().to_vec(),
        cv_net: cv.to_bytes().to_vec(),
        encrypted_note,
    }
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
    let mut sdk = Sdk::new_mock();
    let count = 2048
        * u64::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    let total_count = count + 1;
    // The first chunk is full, so the stream's 16-wide speculative window
    // slides past it before the short second chunk ends the pool: cover
    // every chunk it may still request.
    for chunk in 0..48u64 {
        let notes = match chunk {
            0 => std::iter::once(first.clone())
                .chain((1..count).map(opaque_note))
                .collect(),
            1 => vec![second.clone()],
            _ => Vec::new(),
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

async fn coordinator_over(
    sdk: Arc<Sdk>,
    keys: &AccountViewingKeys,
) -> (NetworkShieldedCoordinator, Arc<RecordingPersistence>) {
    let host = Arc::new(RecordingPersistence::default());
    let coordinator = coordinator_at(sdk, &tree_path(), keys, Arc::clone(&host)).await;
    (coordinator, host)
}

fn host_synced_index(host: &RecordingPersistence, sub: SubwalletId) -> Option<u64> {
    host.recorded()
        .iter()
        .filter_map(|changeset| changeset.synced_indices.get(&sub).copied())
        .max()
}

fn host_note_values(host: &RecordingPersistence, sub: SubwalletId) -> Vec<u64> {
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
async fn should_defer_activity_and_readiness_until_a_pass_completes() {
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
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    coordinator.install_tree_progress_handler(Some(Arc::new(move |_, _| stop.cancel())));
    let summary = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        coordinator.sync(true, &cancel),
    )
    .await
    .expect("a cancelled pass returns");
    coordinator.install_tree_progress_handler(None);
    assert_eq!(summary.success_count(), 1, "the consumed prefix is flushed");
    {
        let store = coordinator.store().read().await;
        let notes = store.get_all_notes(sub).expect("notes");
        assert_eq!(
            notes.len(),
            1,
            "only the first chunk's receipt was consumed"
        );
        assert_eq!(notes[0].value, FIRST_VALUE);
        assert_eq!(
            store.last_synced_note_index(sub).expect("watermark"),
            count,
            "the resume cursor covers the consumed chunk"
        );
        let balance = store.local_account_balance(sub).expect("balance");
        assert_eq!(
            balance.source,
            ShieldedBalanceSource::NoHistory,
            "a consumed prefix is not a completed scan"
        );
        assert_eq!(balance.last_scanned_index, None);
    }
    assert!(
        activity(&coordinator).await.is_empty(),
        "no activity entry may be derived from a partial scan"
    );
    // The host got the consumed receipt (the advanced in-memory cursor
    // means no later pass re-emits it), but not the cursor itself, which
    // it would restore as completed-scan coverage.
    assert_eq!(host_note_values(&host, sub), vec![FIRST_VALUE]);
    assert_eq!(host_synced_index(&host, sub), None);
    assert!(host
        .recorded()
        .iter()
        .all(|changeset| changeset.activity_entries.is_empty()));
    assert_eq!(
        coordinator
            .local_balance_snapshot(WALLET)
            .await
            .expect("snapshot"),
        ShieldedLocalBalanceState::RestoreIncomplete,
        "an unhydrated account must not read as ready on a partial scan"
    );

    // Pass 2: resumes from the cursor and reaches the tip.
    let summary = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(summary.success_count(), 1);
    {
        let store = coordinator.store().read().await;
        assert_eq!(store.get_all_notes(sub).expect("notes").len(), 2);
        let balance = store.local_account_balance(sub).expect("balance");
        assert_eq!(balance.source, ShieldedBalanceSource::ScannedThisSession);
        assert_eq!(balance.last_scanned_index, Some(count + 1));
        assert_eq!(balance.spendable_credits, FIRST_VALUE + SECOND_VALUE);
    }
    assert_eq!(
        activity(&coordinator).await,
        expected,
        "cancel + resume must derive the same history as one uninterrupted scan"
    );
    assert_eq!(
        host_note_values(&host, sub),
        vec![FIRST_VALUE, SECOND_VALUE]
    );
    assert_eq!(
        host_synced_index(&host, sub),
        Some(count + 1),
        "the completed pass is the first watermark the host sees"
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
/// covered would read as an empty, finished scan). The host restored
/// nothing that claims coverage, and the first pass after the restart
/// finds the receipt the cancelled pass never reached.
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
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        coordinator.install_tree_progress_handler(Some(Arc::new(move |_, _| stop.cancel())));
        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            coordinator.sync(true, &cancel),
        )
        .await
        .expect("a cancelled pass returns");
        let store = coordinator.store().read().await;
        assert_eq!(store.last_synced_note_index(sub).expect("cursor"), count);
        assert!(store.get_all_notes(sub).expect("notes").is_empty());
    }
    assert!(host_note_values(&host, sub).is_empty());
    assert_eq!(host_synced_index(&host, sub), None);

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
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .last_synced_note_index(sub)
            .expect("cursor"),
        0,
        "the restart resumes from the last completed watermark"
    );

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
