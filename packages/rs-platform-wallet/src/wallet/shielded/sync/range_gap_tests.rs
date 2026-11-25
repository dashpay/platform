//! A note scan stops at a gap between fetched ranges
//! (dashpay/platform#6109).
//!
//! Adjacent ranges are fetched concurrently and each is proved at its own
//! height. A range answered a block earlier comes back short of the next
//! range's start, while the next one, answered after the tree grew, is not
//! empty. Feeds `sync_notes_from_stream` that pair of batches: the pass must
//! keep the commitments it read without a gap, publish nothing, and leave
//! every watermark where it was, and the next pass must find every note,
//! those at the positions the first one skipped included.

use std::sync::Arc;

use dash_sdk::platform::shielded::notes_sync::types::{DecryptedNote, ShieldedChunkBatch};
use dash_sdk::platform::shielded::try_decrypt_note;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::{sync_notes_from_stream, NoteScan};
use crate::wallet::shielded::cancelled_scan_tests::{note_to, opaque_note};
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{ShieldedStore, SubwalletId};

/// How many positions one fetched range spans here.
const RANGE: u64 = 4;
/// The height the pool held its first two notes at, and the next one.
const EARLIER: u64 = 1_000;
const LATER: u64 = 1_001;
/// The value of the note received before the gap, which the action past it
/// spends.
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

fn subwallet_of(account: u32) -> (SubwalletId, AccountViewingKeys) {
    let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, account)
        .expect("derive viewing keys")
        .viewing_keys();
    (SubwalletId::new([0x11; 32], account), views)
}

fn subwallet() -> (SubwalletId, AccountViewingKeys) {
    subwallet_of(0)
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
) -> NoteScan {
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

/// The pool as the two answers of a pass that meets the gap give it: the
/// first range answered at [`EARLIER`], when the pool held two notes, and
/// the second at [`LATER`], after it grew to six. Positions 2 and 3 are in
/// neither. `count_seen_first` is the pool's note count the first range
/// carries: 2 when its answer was handed over before the later one arrived,
/// 6 when the later answer had already arrived.
fn gapped(
    views: &AccountViewingKeys,
    pool: &[ShieldedEncryptedNote],
    count_seen_first: u64,
) -> Vec<ShieldedChunkBatch> {
    vec![
        range(views, 0, &pool[..2], count_seen_first, EARLIER),
        range(views, RANGE, &pool[4..], 6, LATER),
    ]
}

/// The same pool with every range answered at [`LATER`].
fn whole(views: &AccountViewingKeys, pool: &[ShieldedEncryptedNote]) -> Vec<ShieldedChunkBatch> {
    vec![
        range(views, 0, &pool[..4], 6, LATER),
        range(views, RANGE, &pool[4..], 6, LATER),
    ]
}

/// A pass that stopped at a gap says so, and hands back no result: no note,
/// spent mark, watermark or covered height leaves it.
fn assert_stopped_at_gap(scan: &NoteScan, case: &str) {
    assert!(matches!(scan, NoteScan::StoppedAtGap), "{case}: {scan:?}");
}

/// A pass that meets the gap keeps the two commitments it read and publishes
/// nothing: no note, no spent mark, no watermark. The next pass ends with
/// the tree and the notes of a scan that never met the gap, whichever of the
/// two answers arrived first.
#[tokio::test]
async fn should_publish_nothing_at_a_gap_between_ranges_and_find_every_note_next_pass() {
    for (case, count_seen_first) in [
        ("the earlier answer was handed over first", 2),
        ("the later answer had already arrived", 6),
    ] {
        let subwallets = [subwallet()];
        let (id, views) = &subwallets[0];
        let pool = pool(views);
        let (store, path) = open("gap");

        let first = scan(&store, &subwallets, gapped(views, &pool, count_seen_first)).await;

        assert_stopped_at_gap(&first, case);
        {
            let store = store.read().await;
            assert_eq!(
                store.tree_size().unwrap(),
                2,
                "{case}: no commitment past the gap is appended"
            );
            assert_eq!(
                store.last_synced_note_index(*id).unwrap(),
                0,
                "{case}: the watermark does not move"
            );
            assert!(
                store.get_all_notes(*id).unwrap().is_empty(),
                "{case}: a receipt read before the gap does not enter the ledger"
            );
        }

        let second = scan(&store, &subwallets, whole(views, &pool))
            .await
            .finished();

        assert_eq!(
            second.per_subwallet_new_notes.get(id),
            Some(&2),
            "{case}: the note at a position the first pass skipped is found too"
        );
        assert_eq!(second.per_subwallet_newly_spent.get(id), Some(&1), "{case}");
        assert_eq!(second.covered_height, Some(LATER), "{case}");
        assert_eq!(
            unspent_values(&store, *id).await,
            vec![SKIPPED_VALUE],
            "{case}"
        );

        // A store that never met the gap ends with the same tree.
        let (reference, reference_path) = open("reference");
        scan(&reference, &subwallets, whole(views, &pool)).await;
        {
            let (store, reference) = (store.read().await, reference.read().await);
            assert_eq!(store.tree_size().unwrap(), 6, "{case}");
            assert_eq!(store.last_synced_note_index(*id).unwrap(), 6, "{case}");
            assert_eq!(
                store.tree_anchor().unwrap(),
                reference.tree_anchor().unwrap(),
                "{case}: every commitment sits at its own position"
            );
            let skipped = store
                .get_unspent_notes(*id)
                .unwrap()
                .into_iter()
                .next()
                .expect("the skipped note");
            assert_eq!(skipped.position, 2, "{case}");
            assert!(
                store.witness(skipped.position).unwrap().is_some(),
                "{case}: the recovered note can be spent"
            );
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&reference_path);
    }
}

/// An account bound after another one already scanned the shared tree to the
/// pool's end is scanned from position 0. A pass that meets the gap then
/// reads the account's receipt before the gap and the action that spends it
/// after: it must not show the account that receipt as spendable. It shows
/// it nothing, and the next pass shows it the receipt spent.
#[tokio::test]
async fn should_show_a_late_bound_account_nothing_from_a_pass_that_stops_at_a_gap() {
    let first_bound = subwallet_of(0);
    let late_bound = subwallet_of(1);
    let (late_id, late_views) = (late_bound.0, late_bound.1.clone());
    // The pool's notes belong to the account bound late.
    let pool = pool(&late_views);
    let (store, path) = open("late_bound");

    // The first account scans the whole pool: the shared tree is caught up.
    let caught_up = [first_bound.clone()];
    scan(&store, &caught_up, whole(&caught_up[0].1, &pool)).await;
    assert_eq!(store.read().await.tree_size().unwrap(), 6);

    let both = [first_bound.clone(), late_bound];
    let stopped = scan(&store, &both, gapped(&both[0].1, &pool, 6)).await;

    assert_stopped_at_gap(&stopped, "late bound");
    {
        let store = store.read().await;
        assert!(
            store.get_all_notes(late_id).unwrap().is_empty(),
            "the receipt whose spend lies past the gap is not shown"
        );
        assert_eq!(store.last_synced_note_index(late_id).unwrap(), 0);
        assert_eq!(
            store.last_synced_note_index(first_bound.0).unwrap(),
            6,
            "the caught-up account keeps its watermark"
        );
        assert_eq!(store.tree_size().unwrap(), 6);
    }

    let completed = scan(&store, &both, whole(&both[0].1, &pool))
        .await
        .finished();

    assert_eq!(completed.per_subwallet_new_notes.get(&late_id), Some(&2));
    assert_eq!(completed.per_subwallet_newly_spent.get(&late_id), Some(&1));
    assert_eq!(
        unspent_values(&store, late_id).await,
        vec![SKIPPED_VALUE],
        "the receipt reads spent, the other note spendable"
    );
    assert_eq!(
        store.read().await.last_synced_note_index(late_id).unwrap(),
        6
    );

    let _ = std::fs::remove_file(&path);
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

    assert_stopped_at_gap(&result, "empty range");
    let store = store.read().await;
    assert_eq!(store.tree_size().unwrap(), 0);
    assert_eq!(store.last_synced_note_index(*id).unwrap(), 0);

    let _ = std::fs::remove_file(&path);
}
