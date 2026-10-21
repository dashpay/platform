#![allow(clippy::field_reassign_with_default)]

//! `schema::identity_keys::load_state` reads `identity_keys` rows back
//! into a keyless `IdentityKeysChangeSet`, bit-exact, fail-hard on a
//! corrupt blob.

use crate::common;
use std::collections::BTreeSet;

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
        derivation_rejected: BTreeSet::new(),
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
    rejected.derivation_rejected = BTreeSet::from([IdentityKeyDerivationIndices {
        identity_index: 0,
        key_index: 0x11,
    }]);
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
    cleared.derivation_rejected = BTreeSet::new();
    assert_eq!(cs.upserts.get(&(id, 0)), Some(&cleared));
    assert_eq!(cs.upserts.get(&(id, 1)), Some(&key_entry(id, 1, 0x22)));
}

/// A rejection refutes only the coordinates it was checked at: a breadcrumb
/// stored at other coordinates (supplied explicitly through `add_keys`) is
/// kept.
#[test]
fn should_keep_a_stored_breadcrumb_a_rejection_does_not_name() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD6);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x10; 32]);
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
    // Key id 3, stored with the breadcrumb `(0, 0x33)` its caller supplied.
    let mut stored = key_entry(id, 3, 0x33);
    stored.public_key = key_entry(id, 3, 0x33).public_key;
    store(stored.clone());
    // A load checks `(0, 3)` and rejects it.
    let mut rejected = stored.clone();
    rejected.derivation_indices = None;
    rejected.derivation_rejected = BTreeSet::from([IdentityKeyDerivationIndices {
        identity_index: 0,
        key_index: 3,
    }]);
    store(rejected);
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state");
    drop(conn);
    assert_eq!(
        cs.upserts
            .get(&(id, 3))
            .and_then(|row| row.derivation_indices),
        stored.derivation_indices
    );
}

/// Two rejections of one key at different coordinates give the same stored
/// row whether they are written one by one or combined in a buffered
/// segment: the legacy inferred breadcrumb either names is cleared.
#[test]
fn should_clear_a_breadcrumb_two_combined_rejections_name_as_one_by_one() {
    for mode in [
        platform_wallet_storage::FlushMode::Immediate,
        platform_wallet_storage::FlushMode::Manual,
    ] {
        let (persister, _tmp, path) = common::fresh_persister_with_mode(mode);
        let w = wid(0xD7);
        ensure_wallet_meta(&persister, &w);
        let id = Identifier::from([0x11; 32]);
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
        // The legacy inferred breadcrumb at `(0, 3)`.
        let mut inferred = key_entry(id, 3, 0x44);
        inferred.derivation_indices = Some(IdentityKeyDerivationIndices {
            identity_index: 0,
            key_index: 3,
        });
        store(inferred.clone());
        persister.flush(w).unwrap();

        let rejecting = |identity_index| {
            let mut entry = inferred.clone();
            entry.derivation_indices = None;
            entry.derivation_rejected = BTreeSet::from([IdentityKeyDerivationIndices {
                identity_index,
                key_index: 3,
            }]);
            entry
        };
        store(rejecting(0));
        store(rejecting(1));
        persister.flush(w).unwrap();
        drop(persister);

        let p2 = reopen(&path);
        let conn = p2.lock_conn_for_test();
        let cs = identity_keys::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .expect("load_state");
        drop(conn);
        assert_eq!(
            cs.upserts.get(&(id, 3)).map(|row| row.derivation_indices),
            Some(None),
            "{mode:?}"
        );
    }
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

/// Store `entries` under `w` as one identity-keys changeset.
fn store_entries(
    persister: &platform_wallet_storage::SqlitePersister,
    w: [u8; 32],
    entries: Vec<IdentityKeyEntry>,
) {
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
        .expect("store identity keys");
}

/// Reopen the database at `path` and read `w`'s identity keys back.
fn reload(path: &std::path::Path, w: &[u8; 32]) -> IdentityKeysChangeSet {
    let persister = reopen(path);
    let conn = persister.lock_conn_for_test();
    identity_keys::load_state(&conn, w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state")
}

/// `entry` as an upsert that carries no derivation breadcrumb.
fn without_breadcrumb(mut entry: IdentityKeyEntry) -> IdentityKeyEntry {
    entry.derivation_indices = None;
    entry
}

/// Insert the row `(id, upsert.key_id)` under `w` directly, bypassing the
/// writer, with `stored` as its blob and `hash_column` as its typed
/// `public_key_hash`, so the blob can disagree with the row's columns. Then
/// store `upsert` (which carries no breadcrumb) over it and return the row a
/// reload reads.
fn reload_after_upsert_over_crafted_row(
    w: [u8; 32],
    id: Identifier,
    stored: &IdentityKeyEntry,
    hash_column: [u8; 20],
    upsert: &IdentityKeyEntry,
) -> Option<IdentityKeyEntry> {
    assert!(upsert.derivation_indices.is_none());
    let (persister, _tmp, path) = fresh_persister();
    ensure_wallet_meta(&persister, &w);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            rusqlite::params![
                w.as_slice(),
                id.as_slice(),
                i64::from(upsert.key_id),
                identity_keys::encode_entry_for_test(stored).unwrap(),
                &hash_column[..],
            ],
        )
        .unwrap();
    store_entries(&persister, w, vec![upsert.clone()]);
    drop(persister);
    reload(&path, &w).upserts.remove(&(id, upsert.key_id))
}

/// A kept breadcrumb keeps the wallet scope the row's blob was stored with:
/// an upsert that carries neither a breadcrumb nor a `wallet_id` takes both
/// from the row, so the reloaded entry still names its wallet.
#[test]
fn should_keep_a_stored_wallet_id_with_the_breadcrumb_when_an_upsert_carries_neither() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE0);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x20; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let mut stored = key_entry(id, 0, 0x11);
    stored.wallet_id = Some(w);
    store_entries(&persister, w, vec![stored.clone()]);

    let mut upsert = without_breadcrumb(stored.clone());
    upsert.wallet_id = None;
    if let IdentityPublicKey::V0(key) = &mut upsert.public_key {
        key.disabled_at = Some(9);
    }
    store_entries(&persister, w, vec![upsert.clone()]);
    drop(persister);

    let row = reload(&path, &w).upserts.remove(&(id, 0)).expect("row");
    assert_eq!(row.wallet_id, Some(w));
    assert_eq!(row.derivation_indices, stored.derivation_indices);
    assert_eq!(row.public_key, upsert.public_key);
}

/// A stored row within the size limit that does not decode holds no
/// breadcrumb to keep: the upsert replaces it and the store succeeds.
#[test]
fn should_replace_a_stored_row_that_does_not_decode() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE1);
    ensure_wallet_meta(&persister, &w);
    // The identity and key id the crafted blob itself names.
    let id = Identifier::from([0xAA; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let crafted = identity_keys::crafted_entry_blob_with_bad_pk_bincode_for_test();
    assert!(identity_keys::decode_entry(&crafted).is_err());
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, 0, ?3, ?4, NULL)",
            rusqlite::params![
                w.as_slice(),
                id.as_slice(),
                crafted,
                &upsert.public_key_hash[..]
            ],
        )
        .unwrap();

    store_entries(&persister, w, vec![upsert.clone()]);
    drop(persister);

    assert_eq!(reload(&path, &w).upserts.get(&(id, 0)), Some(&upsert));
}

/// Control for the crafted-row tests below: a row inserted directly whose
/// blob agrees with its columns keeps its breadcrumb, so each of those tests
/// loses the breadcrumb only to the one disagreement it introduces.
#[test]
fn should_keep_the_breadcrumb_of_a_crafted_row_that_agrees_with_its_columns() {
    let id = Identifier::from([0x21; 32]);
    let stored = key_entry(id, 0, 0x11);
    let upsert = without_breadcrumb(stored.clone());
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE2),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(stored));
}

/// A stored blob naming another identity than its `identity_id` column
/// holds no breadcrumb to keep.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_whose_blob_names_another_identity() {
    let id = Identifier::from([0x22; 32]);
    let stored = key_entry(Identifier::from([0x23; 32]), 0, 0x11);
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE3),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}

/// A stored blob naming another key id than its `key_id` column holds no
/// breadcrumb to keep, even when its public key's own id matches the column.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_whose_blob_names_another_key_id() {
    let id = Identifier::from([0x24; 32]);
    let mut stored = key_entry(id, 0, 0x11);
    stored.key_id = 1;
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE4),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}

/// A stored blob whose public key carries another id than the row's
/// `key_id` holds no breadcrumb to keep, even when the blob's own `key_id`
/// matches the column.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_whose_public_key_has_another_id() {
    let id = Identifier::from([0x25; 32]);
    let mut stored = key_entry(id, 0, 0x11);
    if let IdentityPublicKey::V0(key) = &mut stored.public_key {
        key.id = 5;
    }
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE5),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}

/// A row whose typed `public_key_hash` column disagrees with the hash in its
/// blob holds no breadcrumb to keep, even when the blob's hash matches the
/// upsert's.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_whose_hash_column_disagrees_with_its_blob() {
    let id = Identifier::from([0x26; 32]);
    let stored = key_entry(id, 0, 0x11);
    let upsert = without_breadcrumb(stored.clone());
    let row = reload_after_upsert_over_crafted_row(wid(0xE6), id, &stored, [0x99; 20], &upsert);
    assert_eq!(row, Some(upsert));
}

/// A row stored under another public-key hash than the upsert's holds no
/// breadcrumb to keep, even when the key bytes and the row's columns agree.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_stored_under_another_public_key_hash() {
    let id = Identifier::from([0x27; 32]);
    let mut stored = key_entry(id, 0, 0x11);
    stored.public_key_hash = [0x99; 20];
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE7),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}

/// A stored blob whose `wallet_id` names another wallet than the scope it is
/// read under holds no breadcrumb to keep.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_whose_blob_names_another_wallet() {
    let id = Identifier::from([0x28; 32]);
    let mut stored = key_entry(id, 0, 0x11);
    stored.wallet_id = Some(wid(0xEF));
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE8),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}

/// A row holding other public-key bytes than the upsert's holds no
/// breadcrumb to keep, even when it is stored under the upsert's hash.
#[test]
fn should_not_keep_the_breadcrumb_of_a_row_holding_other_public_key_bytes() {
    let id = Identifier::from([0x29; 32]);
    let mut stored = key_entry(id, 0, 0x11);
    if let IdentityPublicKey::V0(key) = &mut stored.public_key {
        key.data = BinaryData::new(vec![0x44; 33]);
    }
    let upsert = without_breadcrumb(key_entry(id, 0, 0x11));
    let row = reload_after_upsert_over_crafted_row(
        wid(0xE9),
        id,
        &stored,
        stored.public_key_hash,
        &upsert,
    );
    assert_eq!(row, Some(upsert));
}
