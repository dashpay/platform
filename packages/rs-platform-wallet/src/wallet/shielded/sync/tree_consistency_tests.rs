use std::collections::BTreeMap;
use std::sync::Arc;

use dash_async::ThreadRegistry;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::Sdk;
use dashcore::Network;
use drive_proof_verifier::types::{
    NoParamQuery, ShieldedAnchors, ShieldedEncryptedNote, ShieldedEncryptedNotes,
    ShieldedEncryptedNotesQuery,
};
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, OrchardDomain,
    OrchardNoteEncryption, RandomSeed, Rho,
};
use tokio::sync::RwLock;

use super::{serialize_note, sync_notes_across};
use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::store::{ShieldedNote, ShieldedStore, SubwalletId};
use crate::wallet::shielded::{FileBackedShieldedStore, NetworkShieldedCoordinator};

fn owned_note(keys: &OrchardKeySet, position: u64) -> (ShieldedNote, ShieldedEncryptedNote) {
    let mut rho_bytes = [0; 32];
    rho_bytes[0] = position as u8 + 1;
    let rho = Rho::from_bytes(&rho_bytes).unwrap();
    let rseed = (1u16..=u16::MAX)
        .find_map(|n| {
            let mut bytes = [0; 32];
            bytes[2..4].copy_from_slice(&n.to_le_bytes());
            RandomSeed::from_bytes(bytes, &rho).into_option()
        })
        .unwrap();
    let note =
        Note::from_parts(keys.default_address, NoteValue::from_raw(100), rho, rseed).unwrap();
    let cmx = ExtractedNoteCommitment::from(note.commitment()).to_bytes();
    let encryption = OrchardNoteEncryption::<DashMemo>::new(None, note, [0; 36]);
    let mut ciphertext = OrchardDomain::<DashMemo>::epk_bytes(encryption.epk())
        .0
        .to_vec();
    ciphertext.extend_from_slice(encryption.encrypt_note_plaintext().as_ref());
    ciphertext.extend_from_slice(&[0; 80]);
    (
        ShieldedNote {
            note_data: serialize_note(&note),
            position,
            cmx,
            nullifier: note.nullifier(&keys.full_viewing_key).to_bytes(),
            block_height: 1,
            is_spent: false,
            value: 100,
        },
        ShieldedEncryptedNote {
            cmx: cmx.to_vec(),
            nullifier: rho_bytes.to_vec(),
            cv_net: vec![0; 32],
            encrypted_note: ciphertext,
        },
    )
}

async fn sdk_with_notes(notes: Vec<ShieldedEncryptedNote>) -> Arc<Sdk> {
    let mut sdk = Sdk::new_mock();
    let count = 2048
        * u32::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    for chunk in 0..16 {
        sdk.mock()
            .expect_fetch(
                ShieldedEncryptedNotesQuery {
                    start_index: u64::from(count) * chunk,
                    count,
                },
                Some(ShieldedEncryptedNotes {
                    notes: if chunk == 0 {
                        notes.clone()
                    } else {
                        Vec::new()
                    },
                    total_count: notes.len() as u64,
                }),
            )
            .await
            .unwrap();
    }
    Arc::new(sdk)
}

#[tokio::test]
async fn should_rebuild_tree_preserving_notes_and_spends_then_match_recorded_anchor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tree.sqlite");
    let keys = OrchardKeySet::from_seed(&[42; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([42; 32], 0);
    let (first, first_wire) = owned_note(&keys, 0);
    let (mut spent, spent_wire) = owned_note(&keys, 1);
    spent.is_spent = true;
    let sdk = sdk_with_notes(vec![first_wire, spent_wire]).await;
    let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
    for note in [&first, &spent] {
        store.append_commitment(&note.cmx, true).unwrap();
        store.save_note(id, note).unwrap();
    }
    store.set_tree_sync_height(0).unwrap();
    store.checkpoint_tree(2).unwrap();
    let recorded_anchor = store.tree_anchor().unwrap();
    store.append_commitment(&spent.cmx, true).unwrap();
    store.checkpoint_tree(3).unwrap();
    store.set_last_synced_note_index(id, 2).unwrap();
    let coordinator = Arc::new(NetworkShieldedCoordinator::new(
        sdk,
        Network::Testnet,
        path.clone(),
        store,
    ));
    coordinator
        .register_wallet(
            id.wallet_id,
            BTreeMap::from([(0, keys.viewing_keys())]),
            WalletPersister::new(id.wallet_id, Arc::new(NoPlatformPersistence)),
        )
        .await
        .unwrap();

    assert!(matches!(
        coordinator.try_sync(true).await,
        Err(PlatformWalletError::ShieldedTreeInconsistent {
            local_leaf_count: 3,
            chain_leaf_count: 2,
        })
    ));
    let sync_manager = ShieldedSyncManager::new(
        Arc::new(PlatformEventManager::new(Vec::new())),
        Arc::new(RwLock::new(Some(Arc::clone(&coordinator)))),
        ThreadRegistry::new(),
    );
    assert!(sync_manager
        .sync_wallet(&[99; 32], true)
        .await
        .unwrap()
        .is_none());
    assert!(matches!(
        sync_manager.sync_wallet(&id.wallet_id, true).await,
        Err(PlatformWalletError::ShieldedTreeInconsistent {
            local_leaf_count: 3,
            chain_leaf_count: 2,
        })
    ));
    coordinator.rebuild_shielded_tree().await.unwrap();
    {
        let store = coordinator.store().read().await;
        assert_eq!(store.tree_size().unwrap(), 0);
        assert_eq!(store.last_synced_note_index(id).unwrap(), 0);
        let notes = store.get_all_notes(id).unwrap();
        assert_eq!(notes.len(), 2);
        assert!(notes
            .iter()
            .any(|n| n.nullifier == spent.nullifier && n.is_spent));
    }
    assert_eq!(
        FileBackedShieldedStore::open_path(&path, 100)
            .unwrap()
            .tree_size()
            .unwrap(),
        0
    );
    let summary = coordinator.sync(false).await;
    assert_eq!(summary.error_count(), 0);
    assert_eq!(
        summary.success_count(),
        1,
        "registration must survive rebuild"
    );
    let store = coordinator.store().read().await;
    assert_eq!(store.tree_size().unwrap(), 2);
    assert_eq!(store.last_synced_note_index(id).unwrap(), 2);
    let notes = store.get_all_notes(id).unwrap();
    assert_eq!(notes.len(), 2);
    assert!(notes
        .iter()
        .any(|n| n.nullifier == spent.nullifier && n.is_spent));
    assert_eq!(store.get_unspent_notes(id).unwrap().len(), 1);
    let witness = store.witness_at_depth(0, 0).unwrap().unwrap();
    let root = witness
        .root(ExtractedNoteCommitment::from_bytes(&first.cmx).unwrap())
        .to_bytes();
    drop(store);
    let mut anchors_sdk = Sdk::new_mock();
    anchors_sdk
        .mock()
        .expect_fetch(
            NoParamQuery {},
            Some(ShieldedAnchors(vec![recorded_anchor])),
        )
        .await
        .unwrap();
    let anchors = ShieldedAnchors::fetch_current(&anchors_sdk).await.unwrap();
    assert!(anchors.0.contains(&root));
    // Enter the caught-up cooldown, then prove explicit rebuild bypasses it.
    coordinator.sync(false).await;
    coordinator.rebuild_shielded_tree().await.unwrap();
    let summary = coordinator.sync(false).await;
    assert_eq!(summary.error_count(), 0);
    assert_eq!(coordinator.store().read().await.tree_size().unwrap(), 2);
}

#[tokio::test]
async fn should_rescan_empty_tree_despite_restored_watermark() {
    let dir = tempfile::tempdir().unwrap();
    let mut store =
        FileBackedShieldedStore::open_path(dir.path().join("tree.sqlite"), 100).unwrap();
    let keys = OrchardKeySet::from_seed(&[42; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([42; 32], 0);
    let (note, wire) = owned_note(&keys, 0);
    store.save_note(id, &note).unwrap();
    // A host can restore its old watermark after a rebuild and process restart.
    store.set_last_synced_note_index(id, 4096).unwrap();
    let store = Arc::new(RwLock::new(store));
    sync_notes_across(
        &sdk_with_notes(vec![wire]).await,
        &store,
        &[(id, keys.viewing_keys())],
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(store.read().await.tree_size().unwrap(), 1);
    assert_eq!(store.read().await.get_all_notes(id).unwrap().len(), 1);
}

#[tokio::test]
async fn should_preserve_tree_height_across_reopen_and_ignore_one_block_behind_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tree.sqlite");
    let keys = OrchardKeySet::from_seed(&[42; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([42; 32], 0);
    let (first, wire) = owned_note(&keys, 0);
    let (second, _) = owned_note(&keys, 1);
    {
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        // The tree was built from height 1; SDK mock responses are at height 0.
        store.set_tree_sync_height(1).unwrap();
        store.append_commitment(&first.cmx, true).unwrap();
        store.append_commitment(&second.cmx, true).unwrap();
        store.checkpoint_tree(2).unwrap();
    }
    let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
    assert_eq!(store.tree_sync_height().unwrap(), Some(1));
    store.set_last_synced_note_index(id, 2).unwrap();
    let store = Arc::new(RwLock::new(store));
    sync_notes_across(&sdk_with_notes(vec![wire]).await, &store, &[(id, keys.viewing_keys())], None, None)
        .await.expect("one-block-behind source must not trigger inconsistency");
    let mut store = store.write().await;
    assert_eq!(store.tree_size().unwrap(), 2);
    assert_eq!(store.tree_sync_height().unwrap(), Some(1));
    assert_eq!(store.last_synced_note_index(id).unwrap(), 2);
    store.reset_commitment_tree().unwrap();
    assert_eq!(store.tree_sync_height().unwrap(), None);
}

#[test]
fn should_lookup_only_the_discovered_nullifier_spent_status() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileBackedShieldedStore::open_path(dir.path().join("tree.sqlite"), 100).unwrap();
    let keys = OrchardKeySet::from_seed(&[42; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([42; 32], 0);
    let (note, _) = owned_note(&keys, 0);
    assert_eq!(store.note_spent_status(id, &note.nullifier).unwrap(), None);
    store.save_note(id, &note).unwrap();
    assert_eq!(store.note_spent_status(id, &note.nullifier).unwrap(), Some(false));
    store.mark_spent(id, &note.nullifier).unwrap();
    assert_eq!(store.note_spent_status(id, &note.nullifier).unwrap(), Some(true));
}
