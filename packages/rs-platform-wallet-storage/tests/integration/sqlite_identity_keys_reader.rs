#![allow(clippy::field_reassign_with_default)]

//! `schema::identity_keys::load_state` reads `identity_keys` rows back
//! into a keyless `IdentityKeysChangeSet`, bit-exact, fail-hard on a
//! corrupt blob.

use crate::common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::schema::identity_keys;
use platform_wallet_storage::WalletStorageError;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

fn key_entry(identity: Identifier, key_id: u32, byte: u8) -> IdentityKeyEntry {
    IdentityKeyEntry {
        identity_id: identity,
        key_id,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        }),
        public_key_hash: [byte; 20],
        wallet_id: None,
        derivation_indices: Some(IdentityKeyDerivationIndices {
            identity_index: 0,
            key_index: u32::from(byte),
        }),
        derivation_rejected: false,
    }
}

/// An upsert without a derivation breadcrumb keeps the one the row holds
/// and takes the rest of the entry: a key's derivation never changes, and a
/// write that only knows the key's new state (a raised limit, a disable)
/// must not turn a wallet-derived key watch-only. A row without one stays
/// without one.
#[test]
fn should_keep_a_stored_breadcrumb_when_an_upsert_carries_none() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x0C; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let store = |entries: Vec<IdentityKeyEntry>| {
        let mut keys = IdentityKeysChangeSet::default();
        for entry in entries {
            keys.upserts
                .insert((entry.identity_id, entry.key_id), entry);
        }
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    identity_keys: Some(keys),
                    ..Default::default()
                },
            )
            .unwrap();
    };

    let derived = key_entry(id, 0, 0x11);
    let mut watch_only = key_entry(id, 1, 0x22);
    watch_only.derivation_indices = None;
    store(vec![derived.clone(), watch_only.clone()]);

    let mut disabled = derived.clone();
    disabled.derivation_indices = None;
    if let IdentityPublicKey::V0(key) = &mut disabled.public_key {
        key.disabled_at = Some(7);
    }
    let mut watch_only_disabled = watch_only.clone();
    if let IdentityPublicKey::V0(key) = &mut watch_only_disabled.public_key {
        key.disabled_at = Some(8);
    }
    store(vec![disabled.clone(), watch_only_disabled.clone()]);
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);

    let mut expected = disabled;
    expected.derivation_indices = derived.derivation_indices;
    assert_eq!(cs.upserts.get(&(id, 0)), Some(&expected));
    assert_eq!(cs.upserts.get(&(id, 1)), Some(&watch_only_disabled));
}

/// An upsert that settles a key as not derived (`derivation_rejected`)
/// clears a stored breadcrumb, which can only be a stale inferred one, while
/// one that leaves the key's origin unknown keeps a valid same-key one.
#[test]
fn should_clear_a_stored_breadcrumb_a_load_rejects_and_keep_an_unknown_one() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD5);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x0F; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let store = |entries: Vec<IdentityKeyEntry>| {
        let mut keys = IdentityKeysChangeSet::default();
        for entry in entries {
            keys.upserts
                .insert((entry.identity_id, entry.key_id), entry);
        }
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    identity_keys: Some(keys),
                    ..Default::default()
                },
            )
            .unwrap();
    };
    // Key 0: an external key carrying the legacy inferred breadcrumb; key 1:
    // a wallet-derived key with its verified one.
    store(vec![key_entry(id, 0, 0x11), key_entry(id, 1, 0x22)]);

    let mut rejected = key_entry(id, 0, 0x11);
    rejected.derivation_indices = None;
    rejected.derivation_rejected = true;
    let mut unknown = key_entry(id, 1, 0x22);
    unknown.derivation_indices = None;
    store(vec![rejected.clone(), unknown]);
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);
    let mut cleared = rejected;
    cleared.derivation_rejected = false;
    assert_eq!(cs.upserts.get(&(id, 0)), Some(&cleared));
    assert_eq!(cs.upserts.get(&(id, 1)), Some(&key_entry(id, 1, 0x22)));
}

/// A stored row over the blob size limit is not read into memory to look for
/// a breadcrumb; the upsert replaces it.
#[test]
fn should_replace_an_oversized_stored_row_without_reading_it() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD4);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x0E; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let entry = key_entry(id, 0, 0x11);
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, 0, zeroblob(?3), ?4, NULL)",
            rusqlite::params![
                w.as_slice(),
                id.as_slice(),
                (platform_wallet_storage::SIZE_LIMIT_BYTES + 1) as i64,
                &entry.public_key_hash[..],
            ],
        )
        .unwrap();

    let mut upsert = entry.clone();
    upsert.derivation_indices = None;
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((id, 0), upsert.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);
    assert_eq!(cs.upserts.get(&(id, 0)), Some(&upsert));
}

/// A breadcrumb belongs to one key: an upsert of another public key under a
/// stored row's key id does not take the row's breadcrumb.
#[test]
fn should_not_keep_a_breadcrumb_for_another_public_key() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD3);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x0D; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let store = |entry: IdentityKeyEntry| {
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts
            .insert((entry.identity_id, entry.key_id), entry);
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    identity_keys: Some(keys),
                    ..Default::default()
                },
            )
            .unwrap();
    };
    store(key_entry(id, 0, 0x11));
    let mut other = key_entry(id, 0, 0x44);
    other.derivation_indices = None;
    store(other.clone());
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);
    assert_eq!(cs.upserts.get(&(id, 0)), Some(&other));
}

/// Identity-key rows round-trip bit-exact into the keyless
/// `IdentityKeysChangeSet`.
#[test]
fn gk1_identity_keys_roundtrip() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);
    let id_a = Identifier::from([0x0A; 32]);
    let id_b = Identifier::from([0x0B; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id_a.as_slice().try_into().unwrap(),
    )
    .unwrap();
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id_b.as_slice().try_into().unwrap(),
    )
    .unwrap();

    let e1 = key_entry(id_a, 0, 0x11);
    let e2 = key_entry(id_a, 1, 0x22);
    let e3 = key_entry(id_b, 0, 0x33);
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((id_a, 0), e1.clone());
    keys.upserts.insert((id_a, 1), e2.clone());
    keys.upserts.insert((id_b, 0), e3.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);

    assert_eq!(cs.upserts.len(), 3);
    assert_eq!(cs.upserts.get(&(id_a, 0)), Some(&e1));
    assert_eq!(cs.upserts.get(&(id_a, 1)), Some(&e2));
    assert_eq!(cs.upserts.get(&(id_b, 0)), Some(&e3));
    assert!(cs.removed.is_empty());
}

/// An empty wallet yields an empty changeset, not an error.
#[test]
fn gk2_empty_identity_keys_is_ok() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);
    assert!(cs.upserts.is_empty());
}

/// A corrupt `public_key_blob` is a typed hard error, never a silent
/// skip.
#[test]
fn gk3_corrupt_blob_is_hard_error() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD3);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x0C; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, 0, X'00', ?3, NULL)",
            rusqlite::params![w.as_slice(), id.as_slice(), &[0u8; 20][..]],
        )
        .unwrap();
    }
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let result = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict());
    drop(conn);
    assert!(
        matches!(result, Err(WalletStorageError::BincodeDecode { .. })),
        "corrupt public_key_blob must be a typed BincodeDecode; got {result:?}"
    );
}
