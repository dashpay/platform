#![allow(clippy::field_reassign_with_default)]

//! V003 unified-migration schema tests. Numbered V003, not V002: PR #4019
//! (ADDR-09) independently claimed version 2 (`V002__address_height_pin.rs`)
//! and landed first, so the unified migration sequences after it.
//!
//! Covers TC-B-030 (fresh store migrates clean to the new target version),
//! TC-B-003 (`meta_data_versions` shape + PK), the schema half of TC-B-001
//! (`core_address_pool` shape + PK), and the store-generation seed.

mod common;

use std::collections::BTreeMap;

use common::fresh_persister;
use platform_wallet_storage::sqlite::migrations as mig;
use rusqlite::Connection;

/// Column metadata from `PRAGMA table_info`: name → (type, notnull, pk_pos).
fn table_columns(conn: &Connection, table: &str) -> BTreeMap<String, (String, bool, i64)> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("prepare table_info");
    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let ty: String = row.get(2)?;
            let notnull: i64 = row.get(3)?;
            let pk: i64 = row.get(5)?;
            Ok((name, (ty, notnull != 0, pk)))
        })
        .expect("query table_info");
    rows.map(|r| r.expect("row")).collect()
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        rusqlite::params![table],
        |_| Ok(()),
    )
    .optional_exists()
}

trait OptionalExists {
    fn optional_exists(self) -> bool;
}
impl OptionalExists for rusqlite::Result<()> {
    fn optional_exists(self) -> bool {
        matches!(self, Ok(()))
    }
}

/// The unified migration is embedded and supported. The exact ceiling moves
/// with the newest migration and is pinned in that migration's own test file.
#[test]
fn v003_is_embedded_and_supported() {
    assert!(
        mig::embedded_migrations().iter().any(|(v, _)| *v == 3),
        "V003 must be in the embedded migration set"
    );
    assert!(mig::max_supported_version() >= 3, "V003 must be applicable");
}

/// TC-B-030 — a fresh store applies V003 and migrates clean through to the
/// newest embedded migration (e.g. V004's DIP-13 invitations table), and
/// every V003 table exists.
#[test]
fn tc_b_030_fresh_store_migrates_to_version_three() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM refinery_schema_history WHERE version = 3",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(applied, 1, "a fresh store must apply V003");
    let max: i64 = conn
        .query_row("SELECT MAX(version) FROM refinery_schema_history", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        max,
        mig::max_supported_version(),
        "fresh store must land at the newest embedded schema version"
    );
    for table in [
        "core_address_pool",
        "meta_data_versions",
        "meta_store_generation",
    ] {
        assert!(table_exists(&conn, table), "missing table {table}");
    }
}

/// Schema half of TC-B-001 — `core_address_pool` carries per-index rows
/// scoped by `(wallet_id, account_type, account_index, key_class,
/// user_identity_id, friend_identity_id, pool_type, address_index)`, a
/// stored `script`, and a `used` flag. The DashPay identity pair is in the PK
/// (mirroring `account_registrations`) so distinct contacts, which otherwise
/// collapse to the same `(dashpay_receiving, 0)` sentinel, never overwrite
/// each other's pool rows (T5).
#[test]
fn tc_b_001_core_address_pool_shape() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let cols = table_columns(&conn, "core_address_pool");

    for (name, ty) in [
        ("wallet_id", "BLOB"),
        ("account_type", "TEXT"),
        ("account_index", "INTEGER"),
        ("key_class", "INTEGER"),
        ("user_identity_id", "BLOB"),
        ("friend_identity_id", "BLOB"),
        ("pool_type", "INTEGER"),
        ("address_index", "INTEGER"),
        ("script", "BLOB"),
        ("used", "INTEGER"),
    ] {
        let col = cols
            .get(name)
            .unwrap_or_else(|| panic!("core_address_pool missing column {name}"));
        assert_eq!(col.0, ty, "column {name} has unexpected type");
    }

    // Composite PK includes account_type so accounts collapsing to the same
    // (account_index, key_class) sentinel never overwrite each other, the
    // DashPay identity pair so distinct contacts never overwrite each other,
    // and pool_type so External/Internal pools never collide at one
    // address_index.
    let pk: BTreeMap<i64, String> = cols
        .iter()
        .filter(|(_, (_, _, pk))| *pk > 0)
        .map(|(name, (_, _, pk))| (*pk, name.clone()))
        .collect();
    let pk_order: Vec<&str> = pk.values().map(String::as_str).collect();
    assert_eq!(
        pk_order,
        vec![
            "wallet_id",
            "account_type",
            "account_index",
            "key_class",
            "user_identity_id",
            "friend_identity_id",
            "pool_type",
            "address_index"
        ],
        "core_address_pool PK must be (wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id, pool_type, address_index)"
    );
}

/// TC-B-003 — `meta_data_versions` is `(wallet_id BLOB, domain TEXT, seq
/// INTEGER)` with composite PK `(wallet_id, domain)`; `seq` defaults to 0.
#[test]
fn tc_b_003_meta_data_versions_shape() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let cols = table_columns(&conn, "meta_data_versions");

    assert_eq!(cols["wallet_id"].0, "BLOB");
    assert_eq!(cols["domain"].0, "TEXT");
    assert_eq!(cols["seq"].0, "INTEGER");

    let pk: BTreeMap<i64, String> = cols
        .iter()
        .filter(|(_, (_, _, pk))| *pk > 0)
        .map(|(name, (_, _, pk))| (*pk, name.clone()))
        .collect();
    let pk_order: Vec<&str> = pk.values().map(String::as_str).collect();
    assert_eq!(
        pk_order,
        vec!["wallet_id", "domain"],
        "meta_data_versions PK must be (wallet_id, domain)"
    );

    // A domain with no writes yet has seq default 0.
    let w = [0x01u8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        rusqlite::params![w.as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO meta_data_versions (wallet_id, domain) VALUES (?1, 'core_pool')",
        rusqlite::params![w.as_slice()],
    )
    .unwrap();
    let seq: i64 = conn
        .query_row(
            "SELECT seq FROM meta_data_versions WHERE wallet_id = ?1 AND domain = 'core_pool'",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(seq, 0, "seq must default to 0 for a fresh domain");
}

/// The store-generation token is seeded on migration as a non-empty
/// 16-byte blob in the single-row `meta_store_generation` table.
#[test]
fn store_generation_seeded_16_bytes() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let gen: Vec<u8> = conn
        .query_row(
            "SELECT generation FROM meta_store_generation WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .expect("store generation row must exist");
    assert_eq!(gen.len(), 16, "store generation must be 16 bytes");
    assert!(
        gen.iter().any(|b| *b != 0),
        "generation must not be all-zero"
    );
}
