//! Smoke tests for the enum-domain `CHECK` constraints. The schema has
//! four such TEXT columns across four domains: `wallets.network`,
//! `account_registrations.account_type`, `asset_locks.status`, and the
//! synthetic `contacts.state`. These tests exercise each directly.
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
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(1).as_slice(), "not-a-network", 0i64],
    );
    assert_constraint_check(res, "wallets.network");
}

#[test]
fn check_rejects_bad_account_type_on_registrations() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    // First seed a valid parent row so we don't trip the FK.
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(2).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallets");
    let res = conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, ?2, ?3, ?4)",
        params![wid(2).as_slice(), "bogus_account_type", 0i64, &[0u8; 4][..]],
    );
    assert_constraint_check(res, "account_registrations.account_type");
}

#[test]
fn check_rejects_bad_asset_lock_status() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(4).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallets");
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
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
            params![wid_bytes.as_slice(), *label, 0i64],
        )
        .unwrap_or_else(|e| panic!("network={label} should be accepted: {e}"));
    }
}

#[test]
fn check_rejects_bad_contact_state() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    // Seed a valid parent wallet so the insert trips the state CHECK, not the FK.
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(7).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallets");
    let res = conn.execute(
        "INSERT INTO contacts (wallet_id, owner_id, contact_id, state) \
         VALUES (?1, ?2, ?3, ?4)",
        params![
            wid(7).as_slice(),
            &[0xAAu8; 32][..],
            &[0xBBu8; 32][..],
            "not_a_contact_state"
        ],
    );
    assert_constraint_check(res, "contacts.state");
}

#[test]
fn check_accepts_every_known_contact_state_label() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, ?3)",
        params![wid(8).as_slice(), "testnet", 0i64],
    )
    .expect("seed wallets");
    // Mirrors `sqlite::schema::contacts::CONTACT_STATE_LABELS`; hardcoded
    // because that const is `pub(crate)` and unreachable from this separate
    // integration-test crate (same constraint as the network test above).
    // The per-module `contact_state_labels_match_enum` unit test guards the
    // const itself against drift, so a label added there without updating
    // this list surfaces in that test, not as a silent gap here.
    for (i, label) in ["sent", "received", "established"].iter().enumerate() {
        // Same wallet+owner, distinct contact_id per label to keep the
        // composite PK (wallet_id, owner_id, contact_id) unique.
        conn.execute(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                wid(8).as_slice(),
                &[0xC0u8; 32][..],
                &[i as u8; 32][..],
                *label
            ],
        )
        .unwrap_or_else(|e| panic!("contact state={label} should be accepted: {e}"));
    }
}
