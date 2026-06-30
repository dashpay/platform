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

use platform_wallet_storage::sqlite::schema::{accounts, identities, identity_keys};
use platform_wallet_storage::WalletStorageError;

/// Blob larger than the 16 MiB cap: one byte over the limit is enough to
/// trigger the pre-read gate without wasting more memory than necessary.
fn oversize_blob() -> Vec<u8> {
    vec![0u8; platform_wallet_storage::SIZE_LIMIT_BYTES + 1]
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
