use std::sync::Arc;

use dash_sdk::Sdk;
use dashcore::Network;
use drive_proof_verifier::types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery,
};
use tokio::sync::RwLock;

use super::sync_notes_across;
use crate::error::PlatformWalletError;
use crate::wallet::shielded::keys::{AccountViewingKeys, OrchardKeySet};
use crate::wallet::shielded::store::{InMemoryShieldedStore, ShieldedStore, SubwalletId};

async fn pool_sdk(note_count: u8) -> Arc<Sdk> {
    pool_sdk_with_total(note_count, u64::from(note_count)).await
}

async fn pool_sdk_with_total(note_count: u8, total_count: u64) -> Arc<Sdk> {
    let mut sdk = Sdk::new_mock();
    let count = 2048
        * u32::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    // Cover the stream's speculative window; only the first chunk has notes.
    for chunk in 0..16 {
        let notes = if chunk == 0 {
            (1..=note_count)
                .map(|i| ShieldedEncryptedNote {
                    cmx: vec![i; 32],
                    nullifier: vec![i; 32],
                    cv_net: vec![0; 32],
                    encrypted_note: Vec::new(), // No owned receipts in this pool.
                })
                .collect()
        } else {
            Vec::new()
        };
        sdk.mock()
            .expect_fetch(
                ShieldedEncryptedNotesQuery {
                    start_index: u64::from(count) * chunk,
                    count,
                },
                Some(ShieldedEncryptedNotes { notes, total_count }),
            )
            .await
            .unwrap();
    }
    Arc::new(sdk)
}

fn seeded_store(initial_size: u8, caught_up: SubwalletId) -> Arc<RwLock<InMemoryShieldedStore>> {
    let mut store = InMemoryShieldedStore::new();
    for i in 1..=initial_size {
        store.append_commitment(&[i; 32], true).unwrap();
    }
    store
        .set_last_synced_note_index(caught_up, u64::from(initial_size))
        .unwrap();
    Arc::new(RwLock::new(store))
}

async fn assert_concurrent_sync_matches_sequential(initial_size: u8) {
    let subwallets: Vec<_> = [1, 2]
        .into_iter()
        .map(|seed| {
            let keys = OrchardKeySet::from_seed(&[seed; 32], Network::Testnet, 0).unwrap();
            (
                SubwalletId::new([seed; 32], 0),
                AccountViewingKeys::from_full_viewing_key(keys.full_viewing_key),
            )
        })
        .collect();
    let sequential = seeded_store(initial_size, subwallets[0].0);
    let concurrent = seeded_store(initial_size, subwallets[0].0);
    let baseline_sdk = pool_sdk(4).await;
    sync_notes_across(&baseline_sdk, &sequential, &subwallets, None, None)
        .await
        .unwrap();

    let first_sdk = pool_sdk(4).await;
    let second_sdk = pool_sdk(4).await;
    let first = sync_notes_across(&first_sdk, &concurrent, &subwallets, None, None);
    let second = sync_notes_across(&second_sdk, &concurrent, &subwallets, None, None);
    futures::pin_mut!(first, second);

    // Queue both at their first store lock, without sleeps or scheduler races.
    // Releasing the writer grants both queued readers their permits: each can
    // snapshot before either acquires the append lock. An atomic write-locked
    // snapshot instead serializes these same two queued syncs.
    let guard = concurrent.write().await;
    assert!(futures::poll!(first.as_mut()).is_pending());
    assert!(futures::poll!(second.as_mut()).is_pending());
    drop(guard);
    let (first, second) = tokio::join!(first, second);
    first.expect("first concurrent sync");
    second.expect("second concurrent sync");

    {
        let expected = sequential.read().await;
        let actual = concurrent.read().await;
        assert_eq!(expected.tree_size().unwrap(), 4);
        assert_eq!(
            actual.tree_size().unwrap(),
            expected.tree_size().unwrap(),
            "concurrent syncs must append each commitment position only once"
        );
        for (id, _) in &subwallets {
            assert_eq!(expected.last_synced_note_index(*id).unwrap(), 4);
            assert_eq!(
                actual.last_synced_note_index(*id).unwrap(),
                expected.last_synced_note_index(*id).unwrap()
            );
        }
    }

    // A later real position must remain appendable after the overlapping scans.
    let next_sdk = pool_sdk(5).await;
    sync_notes_across(&next_sdk, &concurrent, &subwallets, None, None)
        .await
        .unwrap();
    let actual = concurrent.read().await;
    assert_eq!(actual.tree_size().unwrap(), 5);
    for (id, _) in &subwallets {
        assert_eq!(actual.last_synced_note_index(*id).unwrap(), 5);
    }
}

#[tokio::test]
async fn should_append_commitments_once_across_concurrent_cold_syncs() {
    assert_concurrent_sync_matches_sequential(0).await;
}

#[tokio::test]
async fn should_append_commitments_once_across_concurrent_resumed_syncs() {
    // One subwallet is caught up and the other still needs the existing leaves.
    assert_concurrent_sync_matches_sequential(2).await;
}

#[tokio::test]
async fn should_reject_duplicated_commitments_after_sync() {
    let keys = OrchardKeySet::from_seed(&[1; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([1; 32], 0);
    let subwallets = [(
        id,
        AccountViewingKeys::from_full_viewing_key(keys.full_viewing_key),
    )];
    let store = seeded_store(4, id);
    store.write().await.set_tree_sync_height(0).unwrap();
    store
        .write()
        .await
        .append_commitment(&[4; 32], true)
        .unwrap();
    let result = sync_notes_across(&pool_sdk(4).await, &store, &subwallets, None, None).await;
    assert!(
        matches!(
            result,
            Err(PlatformWalletError::ShieldedTreeInconsistent {
                local_leaf_count: 5,
                chain_leaf_count: 4,
            })
        ),
        "a duplicated tree must fail sync with evidence, got {result:?}"
    );
    assert_eq!(
        store.read().await.tree_size().unwrap(),
        5,
        "detection must not rebuild automatically"
    );
}

#[tokio::test]
async fn should_accept_healthy_and_behind_trees() {
    let keys = OrchardKeySet::from_seed(&[1; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([1; 32], 0);
    let subwallets = [(
        id,
        AccountViewingKeys::from_full_viewing_key(keys.full_viewing_key),
    )];
    for initial_size in [0, 2, 4] {
        let store = seeded_store(initial_size, id);
        sync_notes_across(&pool_sdk(4).await, &store, &subwallets, None, None)
            .await
            .unwrap();
        assert_eq!(store.read().await.tree_size().unwrap(), 4);
    }
}

#[tokio::test]
async fn should_accept_tree_behind_streams_maximum_count() {
    let keys = OrchardKeySet::from_seed(&[1; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([1; 32], 0);
    let store = seeded_store(0, id);
    // The stream's max-seen total can come from a newer speculative response
    // than its final partial chunk. The local tree remains legitimately behind.
    sync_notes_across(
        &pool_sdk_with_total(4, 6).await,
        &store,
        &[(id, keys.viewing_keys())],
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(store.read().await.tree_size().unwrap(), 4);
}

#[tokio::test]
async fn should_skip_lower_count_for_legacy_tree_with_unknown_height() {
    let keys = OrchardKeySet::from_seed(&[1; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([1; 32], 0);
    // A restored healthy tree may have been built at height 1; SDK mock
    // responses have height 0. Without provenance a lower count proves nothing.
    let store = seeded_store(5, id);
    sync_notes_across(&pool_sdk(4).await, &store, &[(id, keys.viewing_keys())], None, None)
        .await.expect("an older source must not diagnose a healthy tree as corrupt");
    assert_eq!(store.read().await.tree_size().unwrap(), 5);
}

#[tokio::test]
async fn should_not_infer_tree_height_from_speculative_total() {
    let keys = OrchardKeySet::from_seed(&[1; 32], Network::Testnet, 0).unwrap();
    let id = SubwalletId::new([1; 32], 0);
    let store = seeded_store(5, id);
    let subwallets = [(id, keys.viewing_keys())];
    // total_count may describe a newer speculative response, not this chunk.
    sync_notes_across(&pool_sdk_with_total(4, 6).await, &store, &subwallets, None, None).await.unwrap();
    assert_eq!(store.read().await.tree_sync_height().unwrap(), None);
    sync_notes_across(&pool_sdk(4).await, &store, &subwallets, None, None).await.unwrap();
    assert_eq!(store.read().await.last_synced_note_index(id).unwrap(), 5);
}
