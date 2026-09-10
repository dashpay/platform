//! Integration tests for the per-object-type KV metadata facility on
//! `SqlitePersister`.
//!
//! Covers the public [`KvStore`] surface (`get` / `put` / `delete` /
//! `list_keys`) across all six [`ObjectId`] scopes, the soft-cascade
//! contract (parentless `put` succeeds; an `AFTER DELETE` trigger cleans
//! the metadata up when the parent is deleted, including via FK cascade),
//! the `delete_wallet` cascade wiring, key/value bounds, prefix
//! escaping, and scope isolation.

#![cfg(feature = "kv")]

mod common;

use common::{
    ensure_contact_established, ensure_contact_received, ensure_contact_sent, ensure_identity,
    ensure_platform_address, ensure_token_balance, ensure_wallet_meta, fresh_persister, wid,
};

use platform_wallet_storage::kv::{KvError, MAX_KEY_LEN, MAX_VALUE_LEN};
use platform_wallet_storage::{KvStore, ObjectId};

fn id32(byte: u8) -> [u8; 32] {
    [byte; 32]
}

/// Full per-scope roundtrip: get→None, put, get, overwrite, delete,
/// get→None. Parent rows must be seeded first by the caller.
fn roundtrip(p: &impl KvStore, scope: &ObjectId) {
    assert_eq!(p.get(scope, "k").unwrap(), None);
    p.put(scope, "k", b"v1").unwrap();
    assert_eq!(p.get(scope, "k").unwrap().as_deref(), Some(&b"v1"[..]));
    p.put(scope, "k", b"v2").unwrap();
    assert_eq!(p.get(scope, "k").unwrap().as_deref(), Some(&b"v2"[..]));
    p.delete(scope, "k").unwrap();
    assert_eq!(p.get(scope, "k").unwrap(), None);
}

#[test]
fn tc_md_001_roundtrip_global() {
    let (p, _tmp, _path) = fresh_persister();
    roundtrip(&p, &ObjectId::Global);
}

#[test]
fn tc_md_002_roundtrip_wallet() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(1);
    ensure_wallet_meta(&p, &w);
    roundtrip(&p, &ObjectId::Wallet(w));
}

#[test]
fn tc_md_003_roundtrip_identity() {
    let (p, _tmp, _path) = fresh_persister();
    let idy = id32(2);
    ensure_identity(&p, &idy, None);
    roundtrip(&p, &ObjectId::Identity(idy));
}

#[test]
fn tc_md_004_roundtrip_token() {
    let (p, _tmp, _path) = fresh_persister();
    let idy = id32(3);
    let token = id32(0x30);
    ensure_identity(&p, &idy, None);
    ensure_token_balance(&p, &idy, &token);
    roundtrip(
        &p,
        &ObjectId::Token {
            identity_id: idy,
            token_id: token,
        },
    );
}

#[test]
fn tc_md_005_roundtrip_contact() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(4);
    let owner = id32(0x40);
    let contact = id32(0x41);
    ensure_wallet_meta(&p, &w);
    ensure_contact_established(&p, &w, &owner, &contact);
    roundtrip(
        &p,
        &ObjectId::Contact {
            wallet_id: w,
            owner_id: owner,
            contact_id: contact,
        },
    );
}

#[test]
fn tc_md_006_roundtrip_platform_address() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(5);
    let address = vec![0xAA, 0xBB, 0xCC, 0xDD];
    ensure_wallet_meta(&p, &w);
    ensure_platform_address(&p, &w, &address);
    roundtrip(
        &p,
        &ObjectId::PlatformAddress {
            wallet_id: w,
            address,
        },
    );
}

/// Put `(scope, "k") = b"v"` with NO parent row present, then read it
/// back. Asserts the soft-cascade model: writes don't require a parent.
fn assert_parentless_put_roundtrips(p: &impl KvStore, scope: &ObjectId) {
    p.put(scope, "k", b"v").unwrap();
    assert_eq!(p.get(scope, "k").unwrap().as_deref(), Some(&b"v"[..]));
}

#[test]
fn parentless_put_succeeds_wallet() {
    let (p, _tmp, _path) = fresh_persister();
    assert_parentless_put_roundtrips(&p, &ObjectId::Wallet(wid(0xAB)));
}

#[test]
fn parentless_put_succeeds_identity() {
    let (p, _tmp, _path) = fresh_persister();
    assert_parentless_put_roundtrips(&p, &ObjectId::Identity(id32(0xAC)));
}

#[test]
fn parentless_put_succeeds_token() {
    let (p, _tmp, _path) = fresh_persister();
    assert_parentless_put_roundtrips(
        &p,
        &ObjectId::Token {
            identity_id: id32(0xAD),
            token_id: id32(0xAE),
        },
    );
}

#[test]
fn parentless_put_succeeds_contact() {
    let (p, _tmp, _path) = fresh_persister();
    assert_parentless_put_roundtrips(
        &p,
        &ObjectId::Contact {
            wallet_id: wid(0xAF),
            owner_id: id32(0xB0),
            contact_id: id32(0xB1),
        },
    );
}

#[test]
fn parentless_put_succeeds_platform_address() {
    let (p, _tmp, _path) = fresh_persister();
    assert_parentless_put_roundtrips(
        &p,
        &ObjectId::PlatformAddress {
            wallet_id: wid(0xB2),
            address: vec![0x01, 0x02, 0x03],
        },
    );
}

#[test]
fn tc_md_012_put_global_on_empty_db_is_ok() {
    let (p, _tmp, _path) = fresh_persister();
    p.put(&ObjectId::Global, "k", b"v").unwrap();
    assert_eq!(
        p.get(&ObjectId::Global, "k").unwrap().as_deref(),
        Some(&b"v"[..])
    );
}

/// delete of a never-existing key is idempotent for both the Global
/// scope and a typed scope.
#[test]
fn delete_missing_key_is_idempotent() {
    let (p, _tmp, _path) = fresh_persister();
    p.delete(&ObjectId::Global, "never-existed").unwrap();
    let w = wid(0x90);
    ensure_wallet_meta(&p, &w);
    p.delete(&ObjectId::Wallet(w), "never-existed").unwrap();
}

#[test]
fn list_keys_is_ascending_regardless_of_insert_order() {
    let (p, _tmp, _path) = fresh_persister();
    for k in ["c", "a", "b"] {
        p.put(&ObjectId::Global, k, b"v").unwrap();
    }
    assert_eq!(
        p.list_keys(&ObjectId::Global, None).unwrap(),
        vec!["a", "b", "c"]
    );
}

// Soft cascade via AFTER DELETE trigger: seed+put, DELETE FROM the
// direct parent table, assert the meta row is gone.

#[test]
fn tc_md_013_cascade_identity() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let idy = id32(0x13);
    ensure_identity(&p, &idy, None);
    let scope = ObjectId::Identity(idy);
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM identities WHERE identity_id = ?1",
            params![&idy[..]],
        )
        .expect("delete identity");
    }
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

#[test]
fn tc_md_014_cascade_token() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let idy = id32(0x14);
    let token = id32(0x15);
    ensure_identity(&p, &idy, None);
    ensure_token_balance(&p, &idy, &token);
    let scope = ObjectId::Token {
        identity_id: idy,
        token_id: token,
    };
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            params![&idy[..], &token[..]],
        )
        .expect("delete token_balance");
    }
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

#[test]
fn tc_md_015_cascade_contact() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x16);
    let owner = id32(0x17);
    let contact = id32(0x18);
    ensure_wallet_meta(&p, &w);
    ensure_contact_established(&p, &w, &owner, &contact);
    let scope = ObjectId::Contact {
        wallet_id: w,
        owner_id: owner,
        contact_id: contact,
    };
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM contacts \
             WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3",
            params![w.as_slice(), &owner[..], &contact[..]],
        )
        .expect("delete established contact");
    }
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

#[test]
fn tc_md_016_cascade_platform_address() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x19);
    let address = vec![0xDE, 0xAD, 0xBE, 0xEF];
    ensure_wallet_meta(&p, &w);
    ensure_platform_address(&p, &w, &address);
    let scope = ObjectId::PlatformAddress {
        wallet_id: w,
        address: address.clone(),
    };
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM platform_addresses WHERE wallet_id = ?1 AND address = ?2",
            params![w.as_slice(), address.as_slice()],
        )
        .expect("delete platform_address");
    }
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

// Wallet cascade: direct, plus transitive via identities.

#[test]
fn tc_md_017_cascade_wallet() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x1A);
    ensure_wallet_meta(&p, &w);
    let scope = ObjectId::Wallet(w);
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .expect("delete wallets");
    }
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

#[test]
fn tc_md_017b_cascade_identity_via_wallet() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x1B);
    let idy = id32(0x1C);
    ensure_wallet_meta(&p, &w);
    ensure_identity(&p, &idy, Some(&w));
    let scope = ObjectId::Identity(idy);
    p.put(&scope, "k", b"v").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .expect("delete wallets");
    }
    // wallets delete → identities FK cascade → meta_identity
    // trigger (SQLite fires it for FK-cascade-deleted rows natively).
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

// delete_wallet purges every meta_* for the wallet; Global and another
// wallet's meta_wallet survive.

#[test]
fn tc_md_018_delete_wallet_purges_all_meta_for_wallet() {
    let (p, _tmp, _path) = fresh_persister();
    let a = wid(0x20);
    let b = wid(0x21);
    ensure_wallet_meta(&p, &a);
    ensure_wallet_meta(&p, &b);

    // Wallet-A objects across every per-wallet meta table.
    let idy_a = id32(0x22);
    let token_a = id32(0x23);
    let owner_a = id32(0x24);
    let contact_a = id32(0x25);
    let addr_a = vec![0x10, 0x11];
    ensure_identity(&p, &idy_a, Some(&a));
    ensure_token_balance(&p, &idy_a, &token_a);
    ensure_contact_established(&p, &a, &owner_a, &contact_a);
    ensure_platform_address(&p, &a, &addr_a);

    let wallet_a = ObjectId::Wallet(a);
    let identity_a = ObjectId::Identity(idy_a);
    let token_scope_a = ObjectId::Token {
        identity_id: idy_a,
        token_id: token_a,
    };
    let contact_scope_a = ObjectId::Contact {
        wallet_id: a,
        owner_id: owner_a,
        contact_id: contact_a,
    };
    let addr_scope_a = ObjectId::PlatformAddress {
        wallet_id: a,
        address: addr_a.clone(),
    };
    for scope in [
        &wallet_a,
        &identity_a,
        &token_scope_a,
        &contact_scope_a,
        &addr_scope_a,
    ] {
        p.put(scope, "k", b"v").unwrap();
    }

    // Survivors: a global slot and wallet-B's meta_wallet.
    p.put(&ObjectId::Global, "g", b"keep").unwrap();
    let wallet_b = ObjectId::Wallet(b);
    p.put(&wallet_b, "k", b"keep").unwrap();

    p.delete_wallet(a).expect("delete_wallet A");

    for scope in [
        &wallet_a,
        &identity_a,
        &token_scope_a,
        &contact_scope_a,
        &addr_scope_a,
    ] {
        assert_eq!(
            p.get(scope, "k").unwrap(),
            None,
            "scope {scope:?} should be purged"
        );
    }
    assert_eq!(
        p.get(&ObjectId::Global, "g").unwrap().as_deref(),
        Some(&b"keep"[..])
    );
    assert_eq!(
        p.get(&wallet_b, "k").unwrap().as_deref(),
        Some(&b"keep"[..])
    );
}

#[test]
fn tc_md_019_delete_wallet_report_counts_meta_tables() {
    let (p, _tmp, _path) = fresh_persister();
    let a = wid(0x26);
    ensure_wallet_meta(&p, &a);

    let idy = id32(0x27);
    let token = id32(0x28);
    let owner = id32(0x29);
    let contact = id32(0x2A);
    let addr = vec![0x30, 0x31];
    ensure_identity(&p, &idy, Some(&a));
    ensure_token_balance(&p, &idy, &token);
    ensure_contact_established(&p, &a, &owner, &contact);
    ensure_platform_address(&p, &a, &addr);

    p.put(&ObjectId::Wallet(a), "k", b"v").unwrap();
    p.put(&ObjectId::Identity(idy), "k", b"v").unwrap();
    p.put(
        &ObjectId::Token {
            identity_id: idy,
            token_id: token,
        },
        "k",
        b"v",
    )
    .unwrap();
    p.put(
        &ObjectId::Contact {
            wallet_id: a,
            owner_id: owner,
            contact_id: contact,
        },
        "k",
        b"v",
    )
    .unwrap();
    p.put(
        &ObjectId::PlatformAddress {
            wallet_id: a,
            address: addr,
        },
        "k",
        b"v",
    )
    .unwrap();

    // Also seed a global row that must outlive the delete.
    p.put(&ObjectId::Global, "g", b"keep").unwrap();

    p.delete_wallet(a).expect("delete_wallet");

    // The cascade brooms every wallet/identity-scoped meta table; assert
    // directly that no row survives in any of them and meta_global stays.
    let conn = p.lock_conn_for_test();
    for table in [
        "meta_wallet",
        "meta_identity",
        "meta_token",
        "meta_contact",
        "meta_platform_address",
    ] {
        let n: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(n, 0, "{table} should hold no rows after delete_wallet");
    }
    let global: i64 = conn
        .query_row("SELECT COUNT(*) FROM meta_global", [], |row| row.get(0))
        .unwrap();
    assert_eq!(global, 1, "meta_global must survive the per-wallet delete");
}

/// Write metadata before the parent exists, read it back, create the
/// parent (metadata persists), then delete it (the AFTER DELETE trigger
/// removes the metadata).
#[test]
fn det_write_before_parent_then_create_then_delete() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let idy = id32(0x80);
    let scope = ObjectId::Identity(idy);

    // 1. Write metadata with no parent identity row — succeeds.
    p.put(&scope, "alias", b"satoshi").unwrap();
    assert_eq!(
        p.get(&scope, "alias").unwrap().as_deref(),
        Some(&b"satoshi"[..])
    );

    // 2. The parent identity is synced in later — metadata persists.
    ensure_identity(&p, &idy, None);
    assert_eq!(
        p.get(&scope, "alias").unwrap().as_deref(),
        Some(&b"satoshi"[..])
    );

    // 3. The parent is deleted — the trigger removes the metadata.
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM identities WHERE identity_id = ?1",
            params![&idy[..]],
        )
        .expect("delete identity");
    }
    assert_eq!(p.get(&scope, "alias").unwrap(), None);
}

/// Wallet deletion cascades through core transactions and UTXOs alongside
/// the metadata cleanup triggers.
#[test]
fn delete_wallet_with_core_tx_and_utxo_stays_consistent() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x82);
    ensure_wallet_meta(&p, &w);

    let txid = vec![0xABu8; 32];
    let outpoint = vec![0xCDu8; 36];
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_transactions \
                (wallet_id, txid, finalized, record_blob) \
             VALUES (?1, ?2, 1, X'00')",
            params![w.as_slice(), txid.as_slice()],
        )
        .expect("seed core_transactions");
        conn.execute(
            "INSERT INTO core_utxos \
                (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 1000, X'00', 1)",
            params![w.as_slice(), outpoint.as_slice()],
        )
        .expect("seed core_utxos");
    }

    p.put(&ObjectId::Wallet(w), "k", b"v").unwrap();

    p.delete_wallet(w).expect("delete_wallet must succeed");

    {
        let conn = p.lock_conn_for_test();
        let tx_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1",
                params![w.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        let utxo_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
                params![w.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tx_rows, 0, "core_transactions must be cascaded away");
        assert_eq!(utxo_rows, 0, "core_utxos must be cascaded away");
    }
    assert_eq!(p.get(&ObjectId::Wallet(w), "k").unwrap(), None);
}

/// SQLite fires an AFTER DELETE trigger for a row removed by FK ON DELETE
/// CASCADE natively — `recursive_triggers` (off by default) does not gate
/// this. On a raw connection at defaults, the one-hop chain wallets
/// delete → identities FK cascade → meta_identity trigger cleans up.
#[test]
fn meta_identity_cleanup_fires_on_wallet_cascade() {
    use rusqlite::{params, Connection};

    let tmp = common::secure_tempdir().expect("tempdir");
    let path = tmp.path().join("raw.db");
    let mut conn = Connection::open(&path).expect("open raw conn");
    platform_wallet_storage::sqlite::migrations::run(&mut conn).expect("apply migration");

    // FK cascade on; recursive_triggers left at SQLite's default (OFF).
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert_eq!(
        conn.query_row("SELECT * FROM pragma_recursive_triggers", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0,
        "precondition: recursive_triggers is OFF by default"
    );

    let w = [0x90u8; 32];
    let idy = [0x91u8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![&w[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob) \
         VALUES (?1, ?2, NULL, X'00')",
        params![&idy[..], &w[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO meta_identity (identity_id, key, value) VALUES (?1, 'alias', X'00')",
        params![&idy[..]],
    )
    .unwrap();

    // wallets delete → identities FK cascade → meta_identity trigger.
    conn.execute("DELETE FROM wallets WHERE wallet_id = ?1", params![&w[..]])
        .unwrap();

    let identity_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            params![&idy[..]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(identity_rows, 0, "FK cascade must remove the identity");

    let meta_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meta_identity WHERE identity_id = ?1",
            params![&idy[..]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        meta_rows, 0,
        "the meta_identity trigger fires for the FK-cascade-deleted parent"
    );
}

/// The meta_token chain spans two FK cascades: wallets delete →
/// identities → token_balances → meta_token trigger, firing natively
/// without recursive_triggers.
#[test]
fn meta_token_cleanup_fires_on_wallet_cascade_two_hops() {
    use rusqlite::{params, Connection};

    let tmp = common::secure_tempdir().expect("tempdir");
    let path = tmp.path().join("raw.db");
    let mut conn = Connection::open(&path).expect("open raw conn");
    platform_wallet_storage::sqlite::migrations::run(&mut conn).expect("apply migration");

    // FK cascade on; recursive_triggers left at SQLite's default (OFF).
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert_eq!(
        conn.query_row("SELECT * FROM pragma_recursive_triggers", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0,
        "precondition: recursive_triggers is OFF by default"
    );

    let w = [0xA0u8; 32];
    let idy = [0xA1u8; 32];
    let token = [0xA2u8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![&w[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob) \
         VALUES (?1, ?2, NULL, X'00')",
        params![&idy[..], &w[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, 0, 0)",
        params![&idy[..], &token[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO meta_token (identity_id, token_id, key, value) \
         VALUES (?1, ?2, 'alias', X'00')",
        params![&idy[..], &token[..]],
    )
    .unwrap();

    // Two-hop cascade: wallet → identities → token_balances → trigger.
    conn.execute("DELETE FROM wallets WHERE wallet_id = ?1", params![&w[..]])
        .unwrap();

    let token_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            params![&idy[..], &token[..]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        token_rows, 0,
        "two-hop FK cascade must remove the token balance"
    );

    let meta_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meta_token WHERE identity_id = ?1 AND token_id = ?2",
            params![&idy[..], &token[..]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        meta_rows, 0,
        "the meta_token trigger fires across the two-hop FK cascade"
    );
}

#[test]
fn tc_md_020_empty_key_rejected() {
    let (p, _tmp, _path) = fresh_persister();
    assert!(matches!(
        p.get(&ObjectId::Global, ""),
        Err(KvError::KeyEmpty)
    ));
    assert!(matches!(
        p.put(&ObjectId::Global, "", b"v"),
        Err(KvError::KeyEmpty)
    ));
    assert!(matches!(
        p.delete(&ObjectId::Global, ""),
        Err(KvError::KeyEmpty)
    ));
}

#[test]
fn tc_md_021_too_long_key_rejected() {
    let (p, _tmp, _path) = fresh_persister();
    let too_long = "a".repeat(MAX_KEY_LEN + 1);
    match p.put(&ObjectId::Global, &too_long, b"v") {
        Err(KvError::KeyTooLong { len }) => assert_eq!(len, MAX_KEY_LEN + 1),
        other => panic!("expected KeyTooLong, got {other:?}"),
    }
    match p.get(&ObjectId::Global, &too_long) {
        Err(KvError::KeyTooLong { len }) => assert_eq!(len, MAX_KEY_LEN + 1),
        other => panic!("expected KeyTooLong on get, got {other:?}"),
    }
}

#[test]
fn tc_md_022_max_length_key_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let max_key = "a".repeat(MAX_KEY_LEN);
    p.put(&ObjectId::Global, &max_key, b"v").unwrap();
    assert_eq!(
        p.get(&ObjectId::Global, &max_key).unwrap().as_deref(),
        Some(&b"v"[..])
    );
}

/// An oversized value planted directly is rejected on `get` before
/// materialisation, across every meta_* table.
#[test]
fn tc_md_023_oversized_value_rejected_before_materialising() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0x50);
    let idy = id32(0x51);
    let token = id32(0x52);
    let owner = id32(0x53);
    let contact = id32(0x54);
    let addr = vec![0x60, 0x61];
    ensure_wallet_meta(&p, &w);
    ensure_identity(&p, &idy, Some(&w));
    ensure_token_balance(&p, &idy, &token);
    ensure_contact_established(&p, &w, &owner, &contact);
    ensure_platform_address(&p, &w, &addr);

    let oversize = vec![0u8; MAX_VALUE_LEN + 1];

    // (table, insert SQL with the planted oversized value, get scope).
    type Planter<'a> = (&'a str, Box<dyn Fn(&rusqlite::Connection) + 'a>, ObjectId);
    let planters: Vec<Planter<'_>> = vec![
        (
            "meta_global",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_global (key, value) VALUES ('huge', ?1)",
                    params![oversize.as_slice()],
                )
                .expect("plant meta_global");
            }),
            ObjectId::Global,
        ),
        (
            "meta_wallet",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_wallet (wallet_id, key, value) VALUES (?1, 'huge', ?2)",
                    params![w.as_slice(), oversize.as_slice()],
                )
                .expect("plant meta_wallet");
            }),
            ObjectId::Wallet(w),
        ),
        (
            "meta_identity",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_identity (identity_id, key, value) VALUES (?1, 'huge', ?2)",
                    params![&idy[..], oversize.as_slice()],
                )
                .expect("plant meta_identity");
            }),
            ObjectId::Identity(idy),
        ),
        (
            "meta_token",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_token (identity_id, token_id, key, value) \
                     VALUES (?1, ?2, 'huge', ?3)",
                    params![&idy[..], &token[..], oversize.as_slice()],
                )
                .expect("plant meta_token");
            }),
            ObjectId::Token {
                identity_id: idy,
                token_id: token,
            },
        ),
        (
            "meta_contact",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_contact (wallet_id, owner_id, contact_id, key, value) \
                     VALUES (?1, ?2, ?3, 'huge', ?4)",
                    params![w.as_slice(), &owner[..], &contact[..], oversize.as_slice()],
                )
                .expect("plant meta_contact");
            }),
            ObjectId::Contact {
                wallet_id: w,
                owner_id: owner,
                contact_id: contact,
            },
        ),
        (
            "meta_platform_address",
            Box::new(|c: &rusqlite::Connection| {
                c.execute(
                    "INSERT INTO meta_platform_address (wallet_id, address, key, value) \
                     VALUES (?1, ?2, 'huge', ?3)",
                    params![w.as_slice(), addr.as_slice(), oversize.as_slice()],
                )
                .expect("plant meta_platform_address");
            }),
            ObjectId::PlatformAddress {
                wallet_id: w,
                address: addr.clone(),
            },
        ),
    ];

    for (table, plant, scope) in &planters {
        {
            let conn = p.lock_conn_for_test();
            plant(&conn);
        }
        match p.get(scope, "huge") {
            Err(KvError::ValueTooLarge { found, max }) => {
                assert_eq!(found, MAX_VALUE_LEN + 1, "{table} found mismatch");
                assert_eq!(max, MAX_VALUE_LEN, "{table} max mismatch");
            }
            other => panic!("expected ValueTooLarge for {table}, got {other:?}"),
        }
    }
}

/// list_keys treats `%`/`_`/`\` in the prefix as literals, not LIKE
/// wildcards.
#[test]
fn tc_md_024_list_keys_escapes_like_metacharacters() {
    let (p, _tmp, _path) = fresh_persister();
    p.put(&ObjectId::Global, "100%cotton", b"v").unwrap();
    p.put(&ObjectId::Global, "a_b", b"v").unwrap();
    p.put(&ObjectId::Global, "c\\d", b"v").unwrap();
    p.put(&ObjectId::Global, "axb", b"v").unwrap();
    p.put(&ObjectId::Global, "plain", b"v").unwrap();

    // Literal `%`: matches only the key containing it.
    assert_eq!(
        p.list_keys(&ObjectId::Global, Some("100%")).unwrap(),
        vec!["100%cotton"]
    );
    // `1%` (literal `1` + literal `%`) must NOT wildcard-match `100%cotton`.
    assert!(p
        .list_keys(&ObjectId::Global, Some("1%"))
        .unwrap()
        .is_empty());
    // Literal `_`: `a_` matches `a_b` but not `axb`.
    assert_eq!(
        p.list_keys(&ObjectId::Global, Some("a_")).unwrap(),
        vec!["a_b"]
    );
    // Literal backslash.
    assert_eq!(
        p.list_keys(&ObjectId::Global, Some("c\\")).unwrap(),
        vec!["c\\d"]
    );
}

/// The same key string across Wallet(A)/Wallet(B) and Global/Wallet(A)
/// stays scope-independent.
#[test]
fn tc_md_025_scope_isolation() {
    let (p, _tmp, _path) = fresh_persister();
    let a = wid(0x70);
    let b = wid(0x71);
    ensure_wallet_meta(&p, &a);
    ensure_wallet_meta(&p, &b);

    p.put(&ObjectId::Global, "shared", b"global").unwrap();
    p.put(&ObjectId::Wallet(a), "shared", b"wallet_a").unwrap();
    p.put(&ObjectId::Wallet(b), "shared", b"wallet_b").unwrap();

    assert_eq!(
        p.get(&ObjectId::Global, "shared").unwrap().as_deref(),
        Some(&b"global"[..])
    );
    assert_eq!(
        p.get(&ObjectId::Wallet(a), "shared").unwrap().as_deref(),
        Some(&b"wallet_a"[..])
    );
    assert_eq!(
        p.get(&ObjectId::Wallet(b), "shared").unwrap().as_deref(),
        Some(&b"wallet_b"[..])
    );

    // list_keys per scope sees only its own key.
    assert_eq!(
        p.list_keys(&ObjectId::Global, None).unwrap(),
        vec!["shared"]
    );
    assert_eq!(
        p.list_keys(&ObjectId::Wallet(a), None).unwrap(),
        vec!["shared"]
    );

    // Deleting one scope's key leaves the others untouched.
    p.delete(&ObjectId::Wallet(a), "shared").unwrap();
    assert_eq!(p.get(&ObjectId::Wallet(a), "shared").unwrap(), None);
    assert_eq!(
        p.get(&ObjectId::Global, "shared").unwrap().as_deref(),
        Some(&b"global"[..])
    );
    assert_eq!(
        p.get(&ObjectId::Wallet(b), "shared").unwrap().as_deref(),
        Some(&b"wallet_b"[..])
    );
}

/// Cascade-completeness AND scope: deleting one wallet leaves ZERO rows
/// for that wallet in every per-wallet table AND every
/// wallet/identity-scoped `meta_*` table — including parentless metadata
/// whose typed parent never existed and metadata on non-established
/// contacts — while a SECOND wallet's rows and `meta_global` survive.
///
/// Counts are scoped by `wallet_id` / `identity_id` rather than taken
/// over the whole table, so the test catches both an incomplete cascade
/// (a's rows linger) and an over-broad one (b's rows get swept too).
#[test]
fn delete_wallet_leaves_no_surviving_rows() {
    use rusqlite::params;

    let (p, _tmp, _path) = fresh_persister();
    let a = wid(0x70);
    ensure_wallet_meta(&p, &a);

    let idy = id32(0x71);
    ensure_identity(&p, &idy, Some(&a));

    let token = id32(0x72);
    ensure_token_balance(&p, &idy, &token);

    // Three contacts in distinct lifecycle states.
    let owner = id32(0x73);
    let est_contact = id32(0x74);
    let sent_contact = id32(0x75);
    let recv_contact = id32(0x76);
    ensure_contact_established(&p, &a, &owner, &est_contact);
    ensure_contact_sent(&p, &a, &owner, &sent_contact);
    ensure_contact_received(&p, &a, &owner, &recv_contact);

    let addr = vec![0xAAu8; 20];
    ensure_platform_address(&p, &a, &addr);

    // One typed row in every remaining FK-bearing per-wallet table so
    // the cascade has something to remove in each.
    {
        let conn = p.lock_conn_for_test();
        let txid = vec![0x01u8; 32];
        let outpoint = vec![0x02u8; 36];
        let stmts: &[(&str, &[&dyn rusqlite::ToSql])] = &[
            ("INSERT INTO account_registrations (wallet_id, account_type, account_index, account_xpub_bytes) VALUES (?1, 'standard_bip44', 0, X'00')", &[&a.as_slice()]),
            ("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, X'00')", &[&a.as_slice(), &txid]),
            ("INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) VALUES (?1, ?2, 0, X'00', 0)", &[&a.as_slice(), &outpoint]),
            ("INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, X'00')", &[&a.as_slice(), &txid]),
            ("INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) VALUES (?1, 1, 1)", &[&a.as_slice()]),
            ("INSERT INTO identity_keys (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) VALUES (?1, ?2, 0, X'00', X'00', NULL)", &[&a.as_slice(), &idy.as_slice()]),
            ("INSERT INTO platform_address_sync (wallet_id, sync_height, sync_timestamp, last_known_recent_block) VALUES (?1, 0, 0, 0)", &[&a.as_slice()]),
            ("INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) VALUES (?1, ?2, 'built', 0, 0, 0, X'00')", &[&a.as_slice(), &outpoint]),
            ("INSERT INTO dashpay_profiles (identity_id, profile_blob) VALUES (?1, X'00')", &[&idy.as_slice()]),
            ("INSERT INTO dashpay_payments_overlay (identity_id, payment_id, overlay_blob) VALUES (?1, 'pay1', X'00')", &[&idy.as_slice()]),
        ];
        for (sql, prms) in stmts {
            conn.execute(sql, *prms)
                .unwrap_or_else(|e| panic!("seed `{sql}`: {e}"));
        }
    }

    // Metadata in every scope, including PARENTLESS rows whose typed
    // parent object was never created, plus a global row that must
    // survive the wallet delete.
    p.put(&ObjectId::Global, "k", b"survives").unwrap();
    p.put(&ObjectId::Wallet(a), "k", b"v").unwrap();
    p.put(&ObjectId::Identity(idy), "k", b"v").unwrap();
    // Parented token metadata.
    p.put(
        &ObjectId::Token {
            identity_id: idy,
            token_id: token,
        },
        "k",
        b"v",
    )
    .unwrap();
    // Parentless token metadata — no token_balances row for this pair.
    p.put(
        &ObjectId::Token {
            identity_id: idy,
            token_id: id32(0x7A),
        },
        "k",
        b"v",
    )
    .unwrap();
    // Contact metadata across all three lifecycle states.
    p.put(
        &ObjectId::Contact {
            wallet_id: a,
            owner_id: owner,
            contact_id: est_contact,
        },
        "k",
        b"v",
    )
    .unwrap();
    p.put(
        &ObjectId::Contact {
            wallet_id: a,
            owner_id: owner,
            contact_id: sent_contact,
        },
        "k",
        b"v",
    )
    .unwrap();
    p.put(
        &ObjectId::Contact {
            wallet_id: a,
            owner_id: owner,
            contact_id: recv_contact,
        },
        "k",
        b"v",
    )
    .unwrap();
    // Parentless contact metadata — no contacts row for this pair.
    p.put(
        &ObjectId::Contact {
            wallet_id: a,
            owner_id: owner,
            contact_id: id32(0x7B),
        },
        "k",
        b"v",
    )
    .unwrap();
    // Parented + parentless platform-address metadata.
    p.put(
        &ObjectId::PlatformAddress {
            wallet_id: a,
            address: addr.clone(),
        },
        "k",
        b"v",
    )
    .unwrap();
    p.put(
        &ObjectId::PlatformAddress {
            wallet_id: a,
            address: vec![0xCCu8; 20],
        },
        "k",
        b"v",
    )
    .unwrap();

    // A SECOND wallet `b` with its own per-wallet row, identity-owned
    // rows, a contact, a platform address, and meta in every scope. None
    // of this belongs to `a`, so deleting `a` must leave it untouched —
    // this is what distinguishes a wallet-scoped cascade from an
    // over-broad one that drops every row.
    let b = wid(0x80);
    ensure_wallet_meta(&p, &b);
    let b_idy = id32(0x81);
    ensure_identity(&p, &b_idy, Some(&b));
    let b_token = id32(0x82);
    ensure_token_balance(&p, &b_idy, &b_token);
    let b_owner = id32(0x83);
    let b_contact = id32(0x84);
    ensure_contact_established(&p, &b, &b_owner, &b_contact);
    let b_addr = vec![0xBBu8; 20];
    ensure_platform_address(&p, &b, &b_addr);
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) VALUES (?1, 1, 1)",
            params![b.as_slice()],
        )
        .expect("seed b core_sync_state");
    }
    p.put(&ObjectId::Wallet(b), "k", b"b").unwrap();
    p.put(&ObjectId::Identity(b_idy), "k", b"b").unwrap();
    p.put(
        &ObjectId::Token {
            identity_id: b_idy,
            token_id: b_token,
        },
        "k",
        b"b",
    )
    .unwrap();
    p.put(
        &ObjectId::Contact {
            wallet_id: b,
            owner_id: b_owner,
            contact_id: b_contact,
        },
        "k",
        b"b",
    )
    .unwrap();
    p.put(
        &ObjectId::PlatformAddress {
            wallet_id: b,
            address: b_addr.clone(),
        },
        "k",
        b"b",
    )
    .unwrap();

    p.delete_wallet(a).expect("delete_wallet");

    let conn = p.lock_conn_for_test();

    // `a`'s rows are gone from every wallet-scoped table; `b`'s rows
    // survive. Scoping each count by `wallet_id` catches an over-broad
    // cascade that an unscoped whole-table COUNT(*) would miss.
    let wallet_scoped = [
        "wallets",
        "account_registrations",
        "core_transactions",
        "core_utxos",
        "core_instant_locks",
        "core_sync_state",
        "identities",
        "contacts",
        "platform_addresses",
        "platform_address_sync",
        "asset_locks",
        "meta_wallet",
        "meta_contact",
        "meta_platform_address",
    ];
    for table in wallet_scoped {
        let gone: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                params![a.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(gone, 0, "{table} must drop wallet a's rows after delete");
    }

    // Identity-owned tables (and identity-scoped meta) are keyed by
    // `identity_id`, not `wallet_id`; check `a`'s identity is gone.
    let identity_owned = [
        "identity_keys",
        "token_balances",
        "dashpay_profiles",
        "dashpay_payments_overlay",
        "meta_identity",
        "meta_token",
    ];
    for table in identity_owned {
        let gone: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE identity_id = ?1"),
                params![idy.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(gone, 0, "{table} must drop identity a's rows after delete");
    }

    // Wallet b survives — every table we seeded for it still holds its
    // rows. (b is seeded in a representative subset of the scoped tables,
    // not all of them, so we check exactly the tables it was given.)
    let b_wallet_scoped = [
        "wallets",
        "core_sync_state",
        "identities",
        "contacts",
        "platform_addresses",
        "meta_wallet",
        "meta_contact",
        "meta_platform_address",
    ];
    for table in b_wallet_scoped {
        let kept: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                params![b.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            kept >= 1,
            "{table} must keep wallet b's rows when only a is deleted"
        );
    }
    let b_identity_owned = ["token_balances", "meta_identity", "meta_token"];
    for table in b_identity_owned {
        let kept: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE identity_id = ?1"),
                params![b_idy.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            kept >= 1,
            "{table} must keep identity b's rows when only a is deleted"
        );
    }

    // meta_global is not wallet-scoped and must survive.
    let global: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meta_global WHERE key = ?1",
            params!["k"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(global, 1, "meta_global must survive a per-wallet delete");
}
