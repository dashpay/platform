#![cfg(feature = "shielded")]

use crate::common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ShieldedChangeSet,
};
use platform_wallet::wallet::shielded::SubwalletId;
use platform_wallet_storage::{SqlitePersister, WalletStorageError};

fn changeset(id: SubwalletId, viewing_key: [u8; 96]) -> PlatformWalletChangeSet {
    let mut shielded = ShieldedChangeSet::default();
    shielded.record_viewing_key(id, viewing_key);
    PlatformWalletChangeSet {
        shielded: Some(shielded),
        ..Default::default()
    }
}

#[test]
fn shielded_viewing_keys_round_trip_across_wallets_and_upsert() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet_a = wid(0x31);
    let wallet_b = wid(0x32);
    let subwallet_a = SubwalletId::new(wallet_a, 4);
    let subwallet_b = SubwalletId::new(wallet_b, 9);
    ensure_wallet_meta(&persister, &wallet_a);
    ensure_wallet_meta(&persister, &wallet_b);

    persister
        .store(wallet_a, changeset(subwallet_a, [0xA1; 96]))
        .expect("store wallet A viewing key");
    persister
        .store(wallet_b, changeset(subwallet_b, [0xB2; 96]))
        .expect("store wallet B viewing key");
    persister
        .store(wallet_a, changeset(subwallet_a, [0xC3; 96]))
        .expect("upsert wallet A viewing key");

    let state = persister.load().expect("load viewing keys");
    assert_eq!(state.shielded.viewing_keys.len(), 2);
    assert_eq!(
        state.shielded.viewing_keys.get(&subwallet_a),
        Some(&vec![0xC3; 96])
    );
    assert_eq!(
        state.shielded.viewing_keys.get(&subwallet_b),
        Some(&vec![0xB2; 96])
    );
}

#[test]
fn shielded_viewing_key_wallet_mismatch_is_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let submitted_wallet = wid(0x41);
    let entry_wallet = wid(0x42);
    ensure_wallet_meta(&persister, &submitted_wallet);

    let error = persister
        .store(
            submitted_wallet,
            changeset(SubwalletId::new(entry_wallet, 2), [0xD4; 96]),
        )
        .expect_err("cross-wallet viewing key must be rejected");

    let PersistenceError::Backend { source, .. } = error else {
        panic!("expected typed backend error");
    };
    assert!(matches!(
        source.downcast_ref::<WalletStorageError>(),
        Some(WalletStorageError::WalletIdMismatch { expected, found })
            if *expected == submitted_wallet && *found == entry_wallet
    ));
}

#[test]
fn sqlite_advertises_shielded_viewing_key_capability() {
    let (persister, _tmp, _path) = fresh_persister();
    assert!(persister
        .persistence_capabilities()
        .contains(PersistenceCapabilities::SHIELDED_VIEWING_KEYS));
}

#[test]
fn sqlite_does_not_attest_wallet_drain_without_wallet_restore() {
    let (persister, _tmp, _path) = fresh_persister();
    let capabilities = persister.persistence_capabilities();
    assert!(!capabilities.contains(PersistenceCapabilities::WALLET_DRAIN));
    assert!(capabilities
        .missing(PersistenceCapabilities::WALLET_DRAIN)
        .names()
        .contains(&"wallet_restore"));
    // Native asset-lock persistence remains available independently of the
    // unsupported all-wallet shielded recovery contract.
    assert!(capabilities.contains(PersistenceCapabilities::TRACKED_ASSET_LOCKS));
    assert!(capabilities.contains(PersistenceCapabilities::ASSET_LOCK_OPERATION_IDS));
    assert!(capabilities.contains(PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS));
}

#[test]
fn delete_wallet_cascades_shielded_viewing_keys() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = wid(0x61);
    let subwallet_id = SubwalletId::new(wallet_id, 7);
    ensure_wallet_meta(&persister, &wallet_id);
    persister
        .store(wallet_id, changeset(subwallet_id, [0xA7; 96]))
        .expect("store viewing key");

    persister
        .delete_wallet(wallet_id)
        .expect("inherent deletion must succeed");

    let remaining: i64 = common::ro_conn(&path)
        .query_row(
            "SELECT COUNT(*) FROM shielded_viewing_keys WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
}

fn viewing_key_count(path: &std::path::Path, wallet_id: &[u8; 32]) -> i64 {
    common::ro_conn(path)
        .query_row(
            "SELECT COUNT(*) FROM shielded_viewing_keys WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap()
}

fn insert_mainnet_wallet(persister: &SqlitePersister, wallet_id: &[u8; 32]) {
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'mainnet', 0)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .unwrap();
}

#[test]
fn shielded_wipe_drops_the_networks_viewing_keys_and_keeps_other_networks() {
    let (persister, _tmp, path) = fresh_persister();
    let testnet_wallet = wid(0x71);
    let mainnet_wallet = wid(0x72);
    ensure_wallet_meta(&persister, &testnet_wallet);
    insert_mainnet_wallet(&persister, &mainnet_wallet);
    for wallet in [testnet_wallet, mainnet_wallet] {
        persister
            .store(wallet, changeset(SubwalletId::new(wallet, 0), [0xE1; 96]))
            .expect("store viewing key");
    }

    assert!(persister
        .persistence_capabilities()
        .contains(PersistenceCapabilities::SHIELDED_STATE_WIPE));
    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("wipe testnet");

    assert_eq!(viewing_key_count(&path, &testnet_wallet), 0);
    assert_eq!(viewing_key_count(&path, &mainnet_wallet), 1);
}

#[test]
fn shielded_wipe_drops_the_networks_buffered_viewing_keys() {
    let (persister, _tmp, path) =
        common::fresh_persister_with_mode(platform_wallet_storage::FlushMode::Manual);
    let wallet = wid(0x73);
    ensure_wallet_meta(&persister, &wallet);
    persister
        .store(wallet, changeset(SubwalletId::new(wallet, 0), [0xE2; 96]))
        .expect("buffer viewing key");

    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("wipe testnet");
    persister.commit_writes().expect("flush after wipe");

    assert_eq!(viewing_key_count(&path, &wallet), 0);
}

#[test]
fn shielded_wipe_does_not_fail_on_the_networks_other_buffered_writes() {
    let (persister, _tmp, path) =
        common::fresh_persister_with_mode(platform_wallet_storage::FlushMode::Manual);
    let wallet = wid(0x7B);
    ensure_wallet_meta(&persister, &wallet);
    persister
        .store(
            wallet,
            with_metadata(
                changeset(SubwalletId::new(wallet, 0), [0xE9; 96]),
                dashcore::Network::Testnet,
            ),
        )
        .expect("buffer a viewing key beside another write");
    persister.force_next_flush_to_fail(WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    )));

    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("the wallet's other buffered write does not fail the wipe");

    let report = persister.commit_writes().expect("commit report");
    assert_eq!(
        report.failed.len(),
        1,
        "the wipe flushed nothing, so the primed failure is still armed"
    );
    persister.commit_writes().expect("flush after wipe");
    assert_eq!(viewing_key_count(&path, &wallet), 0);
}

#[test]
fn shielded_wipe_leaves_other_networks_buffered_writes_unflushed() {
    let (persister, _tmp, path) =
        common::fresh_persister_with_mode(platform_wallet_storage::FlushMode::Manual);
    let testnet_wallet = wid(0x74);
    let mainnet_wallet = wid(0x75);
    ensure_wallet_meta(&persister, &testnet_wallet);
    insert_mainnet_wallet(&persister, &mainnet_wallet);
    persister
        .store(
            mainnet_wallet,
            changeset(SubwalletId::new(mainnet_wallet, 0), [0xE3; 96]),
        )
        .expect("buffer the mainnet viewing key");
    persister.force_next_flush_to_fail(WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    )));

    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("a failing mainnet changeset does not fail the testnet wipe");

    let report = persister.commit_writes().expect("commit report");
    assert_eq!(
        report.failed.len(),
        1,
        "the wipe never flushed the mainnet wallet, so the primed failure is still armed"
    );
    assert_eq!(viewing_key_count(&path, &mainnet_wallet), 0);
}

fn with_metadata(
    mut cs: PlatformWalletChangeSet,
    network: dashcore::Network,
) -> PlatformWalletChangeSet {
    cs.wallet_metadata = Some(platform_wallet::changeset::WalletMetadataEntry {
        network,
        wallet_group_id: [0u8; 32],
        birth_height: 0,
    });
    cs
}

#[test]
fn shielded_wipe_classifies_a_wallet_with_no_row_by_its_buffered_network() {
    let (persister, _tmp, path) =
        common::fresh_persister_with_mode(platform_wallet_storage::FlushMode::Manual);
    let testnet_wallet = wid(0x76);
    let mainnet_wallet = wid(0x77);
    persister
        .store(
            testnet_wallet,
            with_metadata(
                changeset(SubwalletId::new(testnet_wallet, 0), [0xE4; 96]),
                dashcore::Network::Testnet,
            ),
        )
        .expect("buffer the new testnet wallet");
    persister
        .store(
            mainnet_wallet,
            with_metadata(
                changeset(SubwalletId::new(mainnet_wallet, 0), [0xE5; 96]),
                dashcore::Network::Mainnet,
            ),
        )
        .expect("buffer the new mainnet wallet");

    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("wipe testnet");

    let wallet_rows = |wallet_id: &[u8; 32]| -> i64 {
        common::ro_conn(&path)
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
                rusqlite::params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(wallet_rows(&testnet_wallet), 0, "the wipe flushes nothing");
    assert_eq!(wallet_rows(&mainnet_wallet), 0, "the wipe flushes nothing");

    persister.commit_writes().expect("flush after wipe");
    assert_eq!(
        wallet_rows(&testnet_wallet),
        1,
        "its other writes stayed buffered"
    );
    assert_eq!(
        viewing_key_count(&path, &testnet_wallet),
        0,
        "its buffered viewing key was dropped"
    );
    assert_eq!(viewing_key_count(&path, &mainnet_wallet), 1);
}

#[test]
fn shielded_wipe_drops_viewing_keys_of_wallets_with_no_row_or_metadata() {
    let (persister, _tmp, path) =
        common::fresh_persister_with_mode(platform_wallet_storage::FlushMode::Manual);
    let orphan_wallet = wid(0x78);
    let mainnet_wallet = wid(0x79);
    let stored_orphan = wid(0x7A);
    insert_mainnet_wallet(&persister, &mainnet_wallet);
    {
        // A row the foreign key would refuse today, as a database written
        // without enforcement could still hold.
        let conn = persister.lock_conn_for_test();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute(
            "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
             VALUES (?1, 0, ?2)",
            rusqlite::params![stored_orphan.as_slice(), [0xE8u8; 96].as_slice()],
        )
        .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    }
    persister
        .store(
            mainnet_wallet,
            changeset(SubwalletId::new(mainnet_wallet, 0), [0xE6; 96]),
        )
        .expect("buffer the mainnet viewing key");
    persister.commit_writes().expect("store the mainnet key");
    persister
        .store(
            orphan_wallet,
            changeset(SubwalletId::new(orphan_wallet, 0), [0xE7; 96]),
        )
        .expect("buffer a viewing key with no wallet row or metadata");

    persister
        .wipe_shielded_state(dashcore::Network::Testnet)
        .expect("wipe testnet");
    persister.commit_writes().expect("flush after wipe");

    assert_eq!(viewing_key_count(&path, &orphan_wallet), 0);
    assert_eq!(viewing_key_count(&path, &stored_orphan), 0);
    assert_eq!(viewing_key_count(&path, &mainnet_wallet), 1);
}

struct NoEvents;
impl platform_wallet::events::EventHandler for NoEvents {}
impl platform_wallet::PlatformEventHandler for NoEvents {}

/// A recovery-mode persister cannot write, so it cannot wipe either: it does
/// not attest `SHIELDED_STATE_WIPE`, and a shielded Clear over it fails at
/// its capability check, before the native reset, leaving the tree, the
/// notes and the watermark as they were.
#[tokio::test]
async fn recovery_persister_attests_no_shielded_wipe_and_clear_resets_nothing() {
    use platform_wallet::wallet::shielded::{ShieldedNote, ShieldedStore};

    let (persister, tmp, _path) = common::fresh_recovery_persister(|_| {});
    assert!(!persister
        .persistence_capabilities()
        .contains(PersistenceCapabilities::SHIELDED_STATE_WIPE));
    assert!(persister
        .persistence_capabilities()
        .contains(PersistenceCapabilities::SHIELDED_VIEWING_KEYS));

    let manager = platform_wallet::PlatformWalletManager::new(
        std::sync::Arc::new(dash_sdk::Sdk::new_mock()),
        std::sync::Arc::new(persister),
        std::sync::Arc::new(NoEvents),
    );
    manager
        .configure_shielded(tmp.path().join("tree.sqlite"))
        .await
        .expect("the shielded store is configured");
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("the manager's coordinator");
    let id = SubwalletId::new(wid(0x61), 0);
    let note = ShieldedNote {
        position: 0,
        cmx: [0x2A; 32],
        nullifier: [0xE0; 32],
        block_height: 100,
        is_spent: false,
        value: 1_000,
        note_data: vec![0u8; 115],
    };
    {
        let mut store = coordinator.store().write().await;
        store.append_commitment(&note.cmx, true).unwrap();
        store.save_note(id, &note).unwrap();
        store.set_last_synced_note_index(id, 1).unwrap();
    }
    let epoch = coordinator.clear_epoch().await;

    let error = manager
        .clear_shielded()
        .await
        .expect_err("Clear refuses a backend that cannot wipe");
    assert!(error.to_string().contains("nothing was reset"), "{error}");

    let store = coordinator.store().read().await;
    assert_eq!(store.tree_size().unwrap(), 1, "the tree is intact");
    let notes = store.get_all_notes(id).unwrap();
    assert_eq!(notes.len(), 1, "the note is intact");
    assert_eq!(notes[0].nullifier, note.nullifier);
    assert_eq!(store.last_synced_note_index(id).unwrap(), 1);
    drop(store);
    assert_eq!(coordinator.clear_epoch().await, epoch);
}
