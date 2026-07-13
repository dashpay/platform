//! V004 (`provider_platform_node_keys`) migration tests — dashpay/platform#4113.
//!
//! Covers TC-PKA-006 (the migration is additive: a new file, no DDL edit to an
//! applied one) and TC-PKA-007 (a store pinned at V003 carries its rows across
//! the upgrade untouched).

mod common;

use common::fresh_persister;
use platform_wallet_storage::sqlite::migrations as mig;
use rusqlite::Connection;

/// V004 is the newest migration; the ceiling derives from the embedded list.
#[test]
fn max_supported_version_is_four() {
    assert_eq!(
        mig::max_supported_version(),
        4,
        "V004 must raise max_supported_version to 4"
    );
}

/// TC-PKA-006 — the node-key table arrives in its own migration file, and a
/// fresh store applies it.
#[test]
fn tc_pka_006_v004_is_a_new_additive_migration() {
    let embedded = mig::embedded_migrations();
    assert!(
        embedded
            .iter()
            .any(|(v, name)| *v == 4 && name == "provider_key_accounts"),
        "V004__provider_key_accounts must be embedded, got {embedded:?}"
    );

    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM refinery_schema_history WHERE version = 4",
            [],
            |r| r.get(0),
        )
        .expect("query schema history");
    assert_eq!(applied, 1, "a fresh store must apply V004");
    assert!(
        table_exists(&conn, "provider_platform_node_keys"),
        "V004 must create provider_platform_node_keys"
    );

    // Additive means the earlier migrations' DDL is untouched: V004 creates
    // exactly one table and alters nothing.
    let v004_sql = mig::embedded_migrations_sql()
        .into_iter()
        .next_back()
        .expect("at least one migration");
    assert!(
        !v004_sql.to_uppercase().contains("ALTER TABLE")
            && !v004_sql.to_uppercase().contains("DROP "),
        "V004 must be create-only; got: {v004_sql}"
    );
}

/// TC-PKA-007 — rows written against the pre-V004 schema survive the upgrade
/// byte-identical, and the tables they live in are not rewritten.
#[test]
fn tc_pka_007_pre_v004_rows_survive_the_upgrade() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let mut conn = Connection::open(&path).expect("open");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enable fks");

    // Stop at V003 — the schema as it stood before this change.
    mig::runner()
        .set_target(refinery::Target::Version(3))
        .run(&mut conn)
        .expect("migrate to V003");

    let wallet_id = [0xE1u8; 32];
    let xpub_bytes = vec![0xAAu8; 42];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 7)",
        rusqlite::params![&wallet_id[..]],
    )
    .expect("insert wallet");
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, 'standard_bip44', 0, ?2)",
        rusqlite::params![&wallet_id[..], xpub_bytes],
    )
    .expect("insert registration");

    // Upgrade to current.
    mig::run(&mut conn).expect("migrate to current");
    assert_eq!(
        mig::max_supported_version(),
        4,
        "the upgrade must land on V004"
    );

    let (network, birth_height): (String, i64) = conn
        .query_row(
            "SELECT network, birth_height FROM wallets WHERE wallet_id = ?1",
            rusqlite::params![&wallet_id[..]],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("wallet row survives");
    assert_eq!((network.as_str(), birth_height), ("testnet", 7));

    let (account_type, stored_xpub): (String, Vec<u8>) = conn
        .query_row(
            "SELECT account_type, account_xpub_bytes FROM account_registrations \
             WHERE wallet_id = ?1",
            rusqlite::params![&wallet_id[..]],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("registration row survives");
    assert_eq!(account_type, "standard_bip44");
    assert_eq!(stored_xpub, vec![0xAAu8; 42], "blob must be byte-identical");

    let node_keys: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_platform_node_keys",
            [],
            |r| r.get(0),
        )
        .expect("new table is queryable");
    assert_eq!(node_keys, 0, "the upgrade invents no rows");
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        rusqlite::params![table],
        |_| Ok(()),
    )
    .is_ok()
}
