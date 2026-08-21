#![cfg(feature = "shielded")]

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ShieldedChangeSet,
};
use platform_wallet::wallet::shielded::SubwalletId;
use platform_wallet_storage::WalletStorageError;

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
fn delete_wallet_trait_cascades_shielded_viewing_keys() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = wid(0x61);
    let subwallet_id = SubwalletId::new(wallet_id, 7);
    ensure_wallet_meta(&persister, &wallet_id);
    persister
        .store(wallet_id, changeset(subwallet_id, [0xA7; 96]))
        .expect("store viewing key");

    PlatformWalletPersistence::delete_wallet(&persister, wallet_id)
        .expect("trait deletion must succeed");

    let remaining: i64 = common::ro_conn(&path)
        .query_row(
            "SELECT COUNT(*) FROM shielded_viewing_keys WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
}
