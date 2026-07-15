#![allow(clippy::field_reassign_with_default)]

//! V004 migration (#4133): a pre-fix `Some(proof)` asset-lock row, written in
//! the old on-disk serde format that is unrecoverable by construction, is
//! dropped so a previously-bricked wallet loads cleanly.
//!
//! Seeds a DB at the pre-fix schema (V003 only) with an `is_locked` row whose
//! `lifecycle_blob` is an old-format `AssetLockEntry { proof: Some(..) }` — the
//! shape that hard-fails rehydration today — then runs migrations and confirms
//! the row is gone and `load_unconsumed` succeeds.

mod common;

use common::wid;

use dashcore::hashes::Hash;
use dashcore::{OutPoint, Transaction, Txid};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::AssetLockEntry;
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::{migrations as mig, schema::asset_locks};
use rusqlite::{params, Connection};

/// An `is_locked` asset-lock entry carrying a chain proof — the exact shape
/// that was written undecodably before the wire-type fix.
fn poison_entry(op: OutPoint) -> AssetLockEntry {
    AssetLockEntry {
        out_point: op,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 0,
        funding_type: AssetLockFundingType::IdentityRegistration,
        identity_index: 0,
        amount_duffs: 1000,
        status: AssetLockStatus::InstantSendLocked,
        proof: Some(AssetLockProof::Chain(ChainAssetLockProof::new(
            42,
            [0x07u8; 36],
        ))),
    }
}

/// Old-format on-disk bytes: exactly what the pre-fix `blob::encode` produced
/// — `bincode::serde::encode_to_vec` straight over the `AssetLockEntry`, which
/// routes the internally-tagged proof enum through the serde bridge. The limit
/// only bounds decode allocation, not the byte layout, so the plain standard
/// config reproduces the on-disk bytes byte-for-byte.
fn old_format_blob(entry: &AssetLockEntry) -> Vec<u8> {
    bincode::serde::encode_to_vec(entry, bincode::config::standard())
        .expect("old-format serde encode must succeed (it always did — only decode failed)")
}

fn asset_lock_count(conn: &Connection, wallet: &[u8; 32]) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM asset_locks WHERE wallet_id = ?1",
        params![&wallet[..]],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn v004_drops_undecodable_proof_bearing_asset_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let mut conn = Connection::open(&path).unwrap();

    // Seed at the pre-fix schema: migrate to V003 only, so V004 has not run.
    mig::runner()
        .set_target(refinery::Target::Version(3))
        .run(&mut conn)
        .expect("migrate to V003");

    let w = wid(0xA4);
    let op = OutPoint {
        txid: Txid::from_byte_array([0x11u8; 32]),
        vout: 0,
    };
    let blob = old_format_blob(&poison_entry(op));

    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .unwrap();
    // Insert the poison row directly (bypassing the writer, which now uses the
    // wire encoder) so the row carries genuine old-format bytes.
    conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'is_locked', 0, 0, 1000, ?3)",
        params![w.as_slice(), &[0x22u8; 36][..], blob],
    )
    .unwrap();

    // Pre-V004: the undecodable row hard-fails the rehydration feed — the
    // bug this migration remediates.
    assert_eq!(asset_lock_count(&conn, &w), 1, "poison row seeded");
    assert!(
        asset_locks::load_unconsumed(&conn, &w).is_err(),
        "pre-V004: the old-format proof row must fail to decode"
    );

    // Run the remaining migrations (applies V004).
    mig::run(&mut conn).expect("apply V004");

    // Post-V004: the row is gone and the feed loads cleanly.
    assert_eq!(
        asset_lock_count(&conn, &w),
        0,
        "V004 must delete the is_locked row"
    );
    let feed = asset_locks::load_unconsumed(&conn, &w).expect("load must succeed after V004");
    assert!(feed.is_empty(), "no asset locks remain after the cleanup");
}

/// V004 keys on the `status` column, so a terminal `consumed` proof-bearing
/// row is intentionally left untouched (it is already SQL-filtered out of
/// rehydration and only test-only readers decode it).
#[test]
fn v004_leaves_consumed_rows_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let mut conn = Connection::open(&path).unwrap();
    mig::runner()
        .set_target(refinery::Target::Version(3))
        .run(&mut conn)
        .expect("migrate to V003");

    let w = wid(0xC0);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'consumed', 0, 0, 1000, ?3)",
        params![w.as_slice(), &[0x33u8; 36][..], &[0xDEu8, 0xAD][..]],
    )
    .unwrap();

    mig::run(&mut conn).expect("apply V004");
    assert_eq!(
        asset_lock_count(&conn, &w),
        1,
        "consumed rows are outside V004's scope and must survive"
    );
}
