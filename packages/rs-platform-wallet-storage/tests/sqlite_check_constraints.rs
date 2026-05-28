//! Smoke tests for the enum-domain `CHECK` constraints on the five
//! enum-shaped TEXT columns (`wallet_metadata.network`,
//! `account_registrations.account_type`,
//! `account_address_pools.account_type`/`pool_type`,
//! `core_derived_addresses.account_type`, and `asset_locks.status`).
//!
//! The per-module parity unit tests in `src/sqlite/schema/*` cover the
//! Rust↔const-array equality. These tests cover the runtime half: a
//! row carrying a label outside the const array is rejected by SQLite
//! with `SqliteFailure(ConstraintCheck, _)`.

mod common;

use common::{fresh_persister, wid};

use rusqlite::{params, ErrorCode};

/// Helper: assert that `res` is a `SqliteFailure` carrying
/// `ConstraintCheck`. Any other shape is a test failure.
fn assert_constraint_check(res: rusqlite::Result<usize>, ctx: &str) {
    let err = res.expect_err(&format!("{ctx}: insert should have been rejected"));
    match err {
        rusqlite::Error::SqliteFailure(ffi_err, _msg) => {
            assert_eq!(
                ffi_err.code,
                ErrorCode::ConstraintViolation,
                "{ctx}: expected ConstraintViolation, got {:?}",
                ffi_err
            );
            assert_eq!(
                ffi_err.extended_code,
                rusqlite::ffi::SQLITE_CONSTRAINT_CHECK,
                "{ctx}: expected SQLITE_CONSTRAINT_CHECK, got extended_code={}",
                ffi_err.extended_code
            );
        }
        other => panic!("{ctx}: expected SqliteFailure(ConstraintCheck), got {other:?}"),
    }
}

#[test]
fn check_rejects_bad_network_label() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let res = conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(1).as_slice(), "not-a-network", 0i64],
    );
    assert_constraint_check(res, "wallet_metadata.network");
}

#[test]
fn check_rejects_bad_account_type_on_registrations() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    // First seed a valid parent row so we don't trip the FK.
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(2).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallet_metadata");
    let res = conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, ?2, ?3, ?4)",
        params![wid(2).as_slice(), "bogus_account_type", 0i64, &[0u8; 4][..]],
    );
    assert_constraint_check(res, "account_registrations.account_type");
}

#[test]
fn check_rejects_bad_pool_type() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(3).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallet_metadata");
    let res = conn.execute(
        "INSERT INTO account_address_pools \
            (wallet_id, account_type, account_index, pool_type, snapshot_blob) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            wid(3).as_slice(),
            "standard",
            0i64,
            "not_a_pool",
            &[0u8; 4][..]
        ],
    );
    assert_constraint_check(res, "account_address_pools.pool_type");
}

#[test]
fn check_rejects_bad_asset_lock_status() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(4).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallet_metadata");
    let res = conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            wid(4).as_slice(),
            &[0u8; 36][..],
            "halfbuilt",
            0i64,
            0i64,
            0i64,
            &[0u8; 4][..],
        ],
    );
    assert_constraint_check(res, "asset_locks.status");
}

#[test]
fn check_accepts_every_known_label_network() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    for (i, label) in ["mainnet", "testnet", "devnet", "regtest"]
        .iter()
        .enumerate()
    {
        let wid_bytes = [i as u8 + 10; 32];
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
            params![wid_bytes.as_slice(), *label, 0i64],
        )
        .unwrap_or_else(|e| panic!("network={label} should be accepted: {e}"));
    }
}
