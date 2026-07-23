#![allow(clippy::field_reassign_with_default)]

//! Pre-read BLOB size-gate regression tests.
//!
//! Proves that each load-path BLOB reader rejects an oversize row with
//! [`WalletStorageError::BlobTooLarge`] **before** materialising the `Vec<u8>`,
//! i.e. the `length(<col>)` gate fires first. The oversize blob is planted
//! directly via raw SQL so the production encode path (which enforces the cap
//! on writes) is bypassed — simulating a tampered / corrupted local wallet DB.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use rusqlite::params;

use platform_wallet_storage::sqlite::schema::{
    accounts, core_pool, core_state, identities, identity_keys,
};
use platform_wallet_storage::WalletStorageError;

/// Blob larger than the 16 MiB cap: one byte over the limit is enough to
/// trigger the pre-read gate without wasting more memory than necessary.
fn oversize_blob() -> Vec<u8> {
    vec![0u8; platform_wallet_storage::SIZE_LIMIT_BYTES + 1]
}

fn p2pkh_script() -> Vec<u8> {
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend([0x11; 20]);
    script.extend([0x88, 0xac]);
    script
}

// ── global SQLITE_LIMIT_LENGTH backstop ─────────────────────────────────────

/// Every connection opened by this crate via `open_conn` must have
/// `SQLITE_LIMIT_LENGTH` set to `SQLITE_MAX_BLOB_BYTES` (32 MiB). This
/// confirms the global backstop is applied even before any per-column gate.
#[test]
fn connection_has_sqlite_limit_length_set() {
    use rusqlite::limits::Limit;
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    // SQLITE_MAX_BLOB_BYTES = 2 × SIZE_LIMIT_BYTES = 32 MiB.
    let expected = (platform_wallet_storage::SIZE_LIMIT_BYTES as i64 * 2) as i32;
    let actual = conn
        .limit(Limit::SQLITE_LIMIT_LENGTH)
        .expect("SQLITE_LIMIT_LENGTH must be readable");
    assert_eq!(
        actual, expected,
        "connection must have SQLITE_LIMIT_LENGTH = {expected} (32 MiB), got {actual}"
    );
}

// ── core_state::load_state — core_utxos script ──────────────────────────────

/// An oversize `script` blob in `core_utxos` is caught by the pre-read
/// `length(script)` gate in `core_state::load_state` and returned as
/// `BlobTooLarge` **before** the Vec is allocated.
#[test]
fn blob_gate_core_utxos_load_state_rejects_oversize_script() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&persister, &w);

    let oversize_script = oversize_blob();
    // A 33-byte outpoint: bincode encodes txid(32 bytes) + vout(1 byte for 0).
    // The outpoint gate passes (33 bytes << 16 MiB); only the script gate fires.
    let tiny_op = vec![0u8; 33];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 0, ?3, 0)",
        params![w.as_slice(), tiny_op.as_slice(), oversize_script.as_slice()],
    )
    .expect("insert oversize script row");

    let err = core_state::load_state(&conn, &w, dashcore::Network::Testnet)
        .expect_err("load_state must reject an oversize script blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge for oversize script, got {err:?}"
    );
}

// ── core_state::load_state — last_applied_chain_lock ────────────────────────

/// An oversize `last_applied_chain_lock` blob is caught by the pre-read
/// `length()` gate in `core_state::load_state` and returned as `BlobTooLarge`
/// **before** the Vec is allocated.
#[test]
fn blob_gate_core_state_load_state_rejects_oversize_chain_lock() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);

    let blob = oversize_blob();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_sync_state \
            (wallet_id, last_processed_height, synced_height, last_applied_chain_lock) \
         VALUES (?1, 0, 0, ?2)",
        params![w.as_slice(), blob.as_slice()],
    )
    .expect("insert oversize chain_lock row");

    let err = core_state::load_state(&conn, &w, dashcore::Network::Testnet)
        .expect_err("load_state must reject an oversize last_applied_chain_lock blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

// ── core_pool::load_used_addresses — core_address_pool script ────────────────

/// An oversize `script` blob in `core_address_pool` is caught by the pre-read
/// `length(script)` gate in `core_pool::load_used_addresses` and returned as
/// `BlobTooLarge` **before** the Vec is allocated.
#[test]
fn blob_gate_core_pool_load_used_addresses_rejects_oversize_script() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE1);
    ensure_wallet_meta(&persister, &w);

    let oversize_script = oversize_blob();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_address_pool \
            (wallet_id, account_type, account_index, key_class, pool_type, \
             address_index, script, used) \
         VALUES (?1, 'standard_bip44', 0, 0, 0, 0, ?2, 1)",
        params![w.as_slice(), oversize_script.as_slice()],
    )
    .expect("insert oversize pool script row");

    let err = core_pool::load_used_addresses(&conn, &w, dashcore::Network::Testnet)
        .expect_err("load_used_addresses must reject an oversize pool script blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge for oversize pool script, got {err:?}"
    );
}

#[test]
fn blob_gate_core_pool_owning_account_rejects_oversize_user_identity_id() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE3);
    ensure_wallet_meta(&persister, &w);

    let script = p2pkh_script();
    let oversize_id = oversize_blob();
    let zero_id = [0u8; 32];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_address_pool \
            (wallet_id, account_type, account_index, key_class, user_identity_id, \
             friend_identity_id, pool_type, address_index, script, used) \
         VALUES (?1, 'dashpay_receiving', 0, 0, ?2, ?3, 0, 0, ?4, 0)",
        params![
            w.as_slice(),
            oversize_id.as_slice(),
            &zero_id[..],
            script.as_slice()
        ],
    )
    .expect("insert pool row with oversize user identity id");
    conn.execute(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 0, ?3, 0)",
        params![w.as_slice(), &[0x01u8], script.as_slice()],
    )
    .expect("insert matching UTXO");

    let err = core_state::load_used_addresses(&conn, &w, dashcore::Network::Testnet)
        .expect_err("ownership lookup must reject an oversize user identity id");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

#[test]
fn blob_gate_core_pool_load_used_addresses_rejects_oversize_friend_identity_id() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE4);
    ensure_wallet_meta(&persister, &w);

    let script = p2pkh_script();
    let oversize_id = oversize_blob();
    let zero_id = [0u8; 32];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_address_pool \
            (wallet_id, account_type, account_index, key_class, user_identity_id, \
             friend_identity_id, pool_type, address_index, script, used) \
         VALUES (?1, 'dashpay_receiving', 0, 0, ?2, ?3, 0, 0, ?4, 1)",
        params![
            w.as_slice(),
            &zero_id[..],
            oversize_id.as_slice(),
            script.as_slice()
        ],
    )
    .expect("insert used pool row with oversize friend identity id");

    let err = core_pool::load_used_addresses(&conn, &w, dashcore::Network::Testnet)
        .expect_err("used-address load must reject an oversize friend identity id");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

// ── core_state::load_used_addresses — core_utxos script ──────────────────────

/// An oversize `script` blob in `core_utxos` is caught by the pre-read
/// `length(script)` gate in `core_state::load_used_addresses` and returned as
/// `BlobTooLarge` **before** the Vec is allocated.
#[test]
fn blob_gate_core_state_load_used_addresses_rejects_oversize_script() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE2);
    ensure_wallet_meta(&persister, &w);

    let oversize_script = oversize_blob();
    // 33-byte outpoint (txid 32 + vout 1); its own gate passes, only the
    // script gate fires.
    let tiny_op = vec![0u8; 33];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 0, ?3, 0)",
        params![w.as_slice(), tiny_op.as_slice(), oversize_script.as_slice()],
    )
    .expect("insert oversize utxo script row");

    let err = core_state::load_used_addresses(&conn, &w, dashcore::Network::Testnet)
        .expect_err("load_used_addresses must reject an oversize utxo script blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge for oversize utxo script, got {err:?}"
    );
}

// ── platform_addrs — address column (fixed 20 bytes) ────────────────────────

/// A `platform_addresses` row whose `address` column is wider than 20 bytes
/// but within the BLOB cap is rejected with `BlobDecode` by the
/// `check_fixed_width` gate before the Vec is materialized.
#[test]
fn blob_gate_platform_addrs_load_all_rejects_wrong_width_address() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    // 21-byte address: wrong width, within size cap.
    let bad_addr = vec![0x42u8; 21];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO platform_addresses \
            (wallet_id, account_index, address_index, address, balance, nonce) \
         VALUES (?1, 0, 0, ?2, 0, 0)",
        params![w.as_slice(), bad_addr.as_slice()],
    )
    .expect("insert wrong-width address row");

    // load_all drives all_address_rows which has the check_fixed_width gate.
    use platform_wallet_storage::sqlite::schema::platform_addrs;
    let err =
        platform_addrs::load_all(&conn).expect_err("load_all must reject a wrong-width address");
    assert!(
        matches!(
            err,
            WalletStorageError::BlobDecode { .. } | WalletStorageError::BlobTooLarge { .. }
        ),
        "expected BlobDecode or BlobTooLarge for wrong-width address, got {err:?}"
    );
}

/// A `platform_addresses` row whose `address` column exceeds the 16 MiB cap
/// is rejected with `BlobTooLarge` by the `check_fixed_width` gate.
#[test]
fn blob_gate_platform_addrs_load_all_rejects_oversize_address() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);

    let oversize_addr = oversize_blob();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO platform_addresses \
            (wallet_id, account_index, address_index, address, balance, nonce) \
         VALUES (?1, 0, 0, ?2, 0, 0)",
        params![w.as_slice(), oversize_addr.as_slice()],
    )
    .expect("insert oversize address row");

    use platform_wallet_storage::sqlite::schema::platform_addrs;
    let err =
        platform_addrs::load_all(&conn).expect_err("load_all must reject an oversize address blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge for oversize address, got {err:?}"
    );
}

// ── identity_keys — bounded inner public_key_bincode decode ──────────────────

/// A `public_key_blob` that is small enough to pass the outer size gate but
/// contains a crafted `public_key_bincode` whose content causes the inner
/// `blob::bounded_config()` decode to fail deterministically, without
/// OOM-allocating. Proves the inner nested decode is end-to-end capped.
#[test]
fn blob_gate_identity_keys_bounded_inner_public_key_bincode() {
    // Build an outer entry blob (tiny, within the 16 MiB gate) that wraps
    // a public_key_bincode containing a huge-length varint. The test helper
    // in identity_keys builds this without going through the bounded encode.
    let crafted_blob = identity_keys::crafted_entry_blob_with_bad_pk_bincode_for_test();

    assert!(
        crafted_blob.len() < platform_wallet_storage::SIZE_LIMIT_BYTES,
        "test blob must fit within the outer gate to exercise the inner path"
    );

    // decode_entry: outer blob::decode succeeds (small blob, valid serde wire);
    // into_entry's inner decode fails on the crafted pk_bincode.
    let err = identity_keys::decode_entry(&crafted_blob)
        .expect_err("inner decode must fail on crafted public_key_bincode");
    assert!(
        matches!(
            err,
            WalletStorageError::BincodeDecode { .. } | WalletStorageError::BlobTooLarge { .. }
        ),
        "expected bounded decode error, got {err:?}"
    );
}

// ── accounts::load_state — account_xpub_bytes ────────────────────────────────

/// An `account_registrations` row whose `account_xpub_bytes` blob exceeds the
/// 16 MiB cap is rejected by `accounts::load_state` with `BlobTooLarge`
/// **before** the Vec is allocated (the `length(account_xpub_bytes)` gate).
#[test]
fn blob_gate_accounts_load_state_rejects_oversize_xpub_bytes() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA1);
    ensure_wallet_meta(&persister, &w);

    let blob = oversize_blob();
    let conn = persister.lock_conn_for_test();
    // Plant the oversize blob directly; `zero_id` (32-byte all-zero) is the
    // default sentinel for `user_identity_id` / `friend_identity_id`.
    let zero_id = [0u8; 32];
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, key_class, \
             user_identity_id, friend_identity_id, account_xpub_bytes) \
         VALUES (?1, 'platform_payment', 0, 0, ?2, ?3, ?4)",
        params![w.as_slice(), &zero_id[..], &zero_id[..], blob.as_slice()],
    )
    .expect("insert oversize xpub_bytes row");

    let err = accounts::load_state(&conn, &w)
        .expect_err("load_state must reject an oversize account_xpub_bytes blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

#[test]
fn blob_gate_accounts_bulk_platform_payment_rejects_oversize_wallet_id() {
    let (persister, _tmp, _path) = fresh_persister();
    let oversize_id = oversize_blob();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![oversize_id.as_slice()],
    )
    .expect("insert wallet with oversize id");
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, 'platform_payment', 0, ?2)",
        params![oversize_id.as_slice(), &[0x00u8]],
    )
    .expect("insert platform payment row with oversize wallet id");

    let err = platform_wallet_storage::sqlite::schema::platform_addrs::load_all(&conn)
        .expect_err("bulk account load must reject an oversize wallet id");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

#[test]
fn blob_gate_accounts_ecdsa_reader_rejects_oversize_user_identity_id() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA2);
    ensure_wallet_meta(&persister, &w);

    let oversize_id = oversize_blob();
    let zero_id = [0u8; 32];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, user_identity_id, \
             friend_identity_id, account_xpub_bytes) \
         VALUES (?1, 'dashpay_receiving', 0, ?2, ?3, ?4)",
        params![
            w.as_slice(),
            oversize_id.as_slice(),
            &zero_id[..],
            &[0x00u8]
        ],
    )
    .expect("insert account row with oversize user identity id");

    let err = accounts::load_state(&conn, &w)
        .expect_err("ECDSA account load must reject an oversize user identity id");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

#[test]
fn blob_gate_accounts_provider_reader_rejects_oversize_friend_identity_id() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA3);
    ensure_wallet_meta(&persister, &w);

    let oversize_id = oversize_blob();
    let zero_id = [0u8; 32];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, user_identity_id, \
             friend_identity_id, account_xpub_bytes) \
         VALUES (?1, 'provider_operator', 0, ?2, ?3, ?4)",
        params![
            w.as_slice(),
            &zero_id[..],
            oversize_id.as_slice(),
            &[0x00u8]
        ],
    )
    .expect("insert provider row with oversize friend identity id");

    let err = accounts::load_state(&conn, &w)
        .expect_err("provider account load must reject an oversize friend identity id");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

// ── identity_keys::load_state — public_key_blob ──────────────────────────────

/// An `identity_keys` row whose `public_key_blob` exceeds the 16 MiB cap is
/// rejected by `identity_keys::load_state` with `BlobTooLarge` before the Vec
/// is materialised (the `length(public_key_blob)` gate).
#[test]
fn blob_gate_identity_keys_load_state_rejects_oversize_public_key_blob() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xB1);
    ensure_wallet_meta(&persister, &w);
    let identity_id = [0xCCu8; 32];

    let blob = oversize_blob();
    let conn = persister.lock_conn_for_test();
    // `identity_keys` has a FK to `identities(identity_id)`; plant the stub.
    identities::ensure_exists(&conn, &w, &identity_id).expect("ensure identity stub");
    let zero_hash = [0u8; 20];
    conn.execute(
        "INSERT INTO identity_keys \
            (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
         VALUES (?1, ?2, 0, ?3, ?4, NULL)",
        params![
            w.as_slice(),
            &identity_id[..],
            blob.as_slice(),
            &zero_hash[..]
        ],
    )
    .expect("insert oversize public_key_blob row");

    let err = identity_keys::load_state(&conn, &w)
        .expect_err("load_state must reject an oversize public_key_blob");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}
