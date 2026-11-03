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

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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
use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};

const WALLET: [u8; 32] = [7; 32];
const FIRST_VALUE: u64 = 100;
const SECOND_VALUE: u64 = 200;

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

async fn coordinator_over(sdk: Arc<Sdk>, keys: &AccountViewingKeys) -> NetworkShieldedCoordinator {
    let path = tree_path();
    let coordinator = NetworkShieldedCoordinator::new(
        sdk,
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(path, 100).expect("open test store"),
    );
    coordinator
        .register_wallet(
            WALLET,
            [(0, keys.clone())].into_iter().collect(),
            WalletPersister::new(WALLET, Arc::new(NoPlatformPersistence)),
        )
        .await
        .expect("register test wallet");
    coordinator
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
    let baseline = coordinator_over(baseline_sdk, &keys).await;
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
    let coordinator = coordinator_over(sdk, &keys).await;
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
