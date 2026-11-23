//! A note scan stops at a gap between fetched ranges
//! (dashpay/platform#6109).
//!
//! Adjacent ranges are fetched concurrently and each is proved at its own
//! height. A range answered a block earlier comes back short of the next
//! range's start, while the next one, answered after the tree grew, is not
//! empty. Feeds `sync_notes_from_stream` that pair of batches: the pass must
//! use only the run of positions it read without a gap, and the next pass
//! must pick up every position the first one skipped.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::{DecryptedNote, ShieldedChunkBatch};
use dash_sdk::platform::shielded::try_decrypt_note;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::{sync_notes_from_stream, MultiSyncNotesResult};
use crate::wallet::shielded::cancelled_scan_tests::{note_to, opaque_note};
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{ShieldedStore, SubwalletId};

/// How many positions one fetched range spans here.
const RANGE: u64 = 4;
/// The height the pool held its first two notes at, and the next one.
const EARLIER: u64 = 1_000;
const LATER: u64 = 1_001;
const KEPT_VALUE: u64 = 100;
const SKIPPED_VALUE: u64 = 500;

type Store = Arc<RwLock<FileBackedShieldedStore>>;

fn temp_tree_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_sync_range_gap_{tag}_{nanos}.sqlite"))
}

fn open(tag: &str) -> (Store, std::path::PathBuf) {
    let path = temp_tree_path(tag);
    let store = FileBackedShieldedStore::open_path(&path, 100).expect("open store");
    (Arc::new(RwLock::new(store)), path)
}

fn subwallet() -> (SubwalletId, AccountViewingKeys) {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
        .expect("derive viewing keys")
        .viewing_keys();
    (SubwalletId::new([0x11; 32], 0), views)
}

/// The range starting at `start_index` as the SDK stream hands it over:
/// `notes`, proved at `block_height`, with the driver's notes decrypted.
fn range(
    views: &AccountViewingKeys,
    start_index: u64,
    notes: &[ShieldedEncryptedNote],
    total_count: u64,
    block_height: u64,
) -> ShieldedChunkBatch {
    let decrypted = notes
        .iter()
        .enumerate()
        .filter_map(|(offset, raw)| {
            let (note, address) = try_decrypt_note(&views.prepared_ivk, raw)?;
            Some(DecryptedNote {
                position: start_index + offset as u64,
                note,
                address,
                nullifier: raw.nullifier.as_slice().try_into().expect("nullifier"),
                cmx: raw.cmx.as_slice().try_into().expect("cmx"),
            })
        })
        .collect();
    ShieldedChunkBatch {
        start_index,
        notes: notes.to_vec(),
        decrypted,
        block_height,
        is_partial: (notes.len() as u64) < RANGE,
        total_count,
    }
}

async fn scan(
    store: &Store,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
    batches: Vec<ShieldedChunkBatch>,
) -> MultiSyncNotesResult {
    sync_notes_from_stream(
        store,
        subwallets,
        None,
        &CancellationToken::new(),
        move |_, _| futures::stream::iter(batches.into_iter().map(Ok)),
    )
    .await
    .expect("the pass succeeds")
}

async fn unspent_values(store: &Store, id: SubwalletId) -> Vec<u64> {
    let mut values: Vec<u64> = store
        .read()
        .await
        .get_unspent_notes(id)
        .expect("unspent notes")
        .iter()
        .map(|note| note.value)
        .collect();
    values.sort_unstable();
    values
}

/// The pool as it reads at [`LATER`], six notes:
///
/// - 0: a note the wallet received, there since [`EARLIER`];
/// - 1: someone else's, there since [`EARLIER`];
/// - 2: a note the wallet received, appended after [`EARLIER`];
/// - 3, 4: someone else's;
/// - 5: the action that spends note 0.
fn pool(views: &AccountViewingKeys) -> Vec<ShieldedEncryptedNote> {
    let kept = note_to(views.default_address, KEPT_VALUE);
    let (kept_note, _) = try_decrypt_note(&views.prepared_ivk, &kept).expect("the wallet's note");
    let spends_kept = ShieldedEncryptedNote {
        nullifier: kept_note
            .nullifier(&views.full_viewing_key)
            .to_bytes()
            .to_vec(),
        ..opaque_note(5)
    };
    vec![
        kept,
        opaque_note(1),
        note_to(views.default_address, SKIPPED_VALUE),
        opaque_note(3),
        opaque_note(4),
        spends_kept,
    ]
}

/// The first range was answered at [`EARLIER`], when the pool held two notes,
/// and the second at [`LATER`], after it grew to six: positions 2 and 3 are in
/// neither. The pass stops at position 2, and the next pass reads on from
/// there, ending with the tree and the notes of a scan that never met the gap.
#[tokio::test]
async fn should_stop_at_a_gap_between_ranges_and_recover_the_skipped_notes_next_pass() {
    let subwallets = [subwallet()];
    let (id, views) = &subwallets[0];
    let pool = pool(views);
    let (store, path) = open("gap");

    let first = scan(
        &store,
        &subwallets,
        vec![
            range(views, 0, &pool[..2], 2, EARLIER),
            range(views, RANGE, &pool[4..], 6, LATER),
        ],
    )
    .await;

    assert_eq!(
        first.total_scanned, 2,
        "only the run read without a gap counts as scanned"
    );
    assert_eq!(
        first.covered_height,
        Some(EARLIER),
        "the pass covers the pool as it read at the earlier height"
    );
    assert_eq!(first.per_subwallet_new_notes.get(id), Some(&1));
    assert!(
        first.per_subwallet_newly_spent.is_empty(),
        "a spend read past the gap is not applied"
    );
    {
        let store = store.read().await;
        assert_eq!(
            store.tree_size().unwrap(),
            2,
            "no commitment past the gap is appended"
        );
        assert_eq!(
            store.last_synced_note_index(*id).unwrap(),
            2,
            "the watermark stops at the first position the pass did not read"
        );
    }
    assert_eq!(unspent_values(&store, *id).await, vec![KEPT_VALUE]);

    // The next pass reads the same pool with every range answered at the
    // later height.
    let whole_pool = || {
        vec![
            range(views, 0, &pool[..4], 6, LATER),
            range(views, RANGE, &pool[4..], 6, LATER),
        ]
    };
    let second = scan(&store, &subwallets, whole_pool()).await;

    assert_eq!(
        second.per_subwallet_new_notes.get(id),
        Some(&1),
        "the note at a position the first pass skipped is found"
    );
    assert_eq!(second.per_subwallet_newly_spent.get(id), Some(&1));
    assert_eq!(second.covered_height, Some(LATER));
    assert_eq!(unspent_values(&store, *id).await, vec![SKIPPED_VALUE]);

    // A store that never met the gap ends with the same tree.
    let (reference, reference_path) = open("reference");
    scan(&reference, &subwallets, whole_pool()).await;
    {
        let (store, reference) = (store.read().await, reference.read().await);
        assert_eq!(store.tree_size().unwrap(), 6);
        assert_eq!(store.last_synced_note_index(*id).unwrap(), 6);
        assert_eq!(
            store.tree_anchor().unwrap(),
            reference.tree_anchor().unwrap(),
            "every commitment sits at its own position"
        );
        let skipped = store
            .get_unspent_notes(*id)
            .unwrap()
            .into_iter()
            .next()
            .expect("the skipped note");
        assert_eq!(skipped.position, 2);
        assert!(
            store.witness(skipped.position).unwrap().is_some(),
            "the recovered note can be spent"
        );
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&reference_path);
}

/// A range that came back empty, answered before the tree reached it, with a
/// later range that is not: nothing past the empty range is used.
#[tokio::test]
async fn should_use_nothing_past_an_empty_range_a_later_range_follows() {
    let subwallets = [subwallet()];
    let (id, views) = &subwallets[0];
    let pool = pool(views);
    let (store, path) = open("empty");

    let result = scan(
        &store,
        &subwallets,
        vec![
            range(views, 0, &[], 0, EARLIER),
            range(views, RANGE, &pool[4..], 6, LATER),
        ],
    )
    .await;

    assert_eq!(result.total_scanned, 0);
    assert!(result.per_subwallet_newly_spent.is_empty());
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0);
    assert_eq!(store.last_synced_note_index(*id).unwrap(), 0);

    let _ = std::fs::remove_file(&path);
}
