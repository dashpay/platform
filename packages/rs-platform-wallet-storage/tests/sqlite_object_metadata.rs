//! Integration tests for the per-object-type KV metadata facility on
//! `SqlitePersister`.
//!
//! Covers the public [`KvStore`] surface (`get` / `put` / `delete` /
//! `list_keys`) across all six [`ObjectId`] scopes, the referential
//! integrity contract (parent-absent `put` → `ObjectNotFound`), the
//! native per-object and per-wallet cascades, the `delete_wallet` report
//! wiring, key/value bounds, prefix escaping, and scope isolation.
//! (TC-MD-001..025.)

#![cfg(feature = "kv")]

mod common;

use common::{
    ensure_contact_established, ensure_identity, ensure_platform_address, ensure_token_balance,
    ensure_wallet_meta, fresh_persister, wid,
};

use platform_wallet_storage::kv::{KvError, ObjectKind, MAX_KEY_LEN, MAX_VALUE_LEN};
use platform_wallet_storage::{KvStore, ObjectId};

fn id32(byte: u8) -> [u8; 32] {
    [byte; 32]
}

// ---------------------------------------------------------------------
// TC-MD-001..006 — per-scope roundtrip (get→None, put, get, overwrite,
// delete, get→None). Parent rows seeded first.
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// TC-MD-007..011 — `put` with parent row ABSENT → ObjectNotFound for the
// five FK scopes. TC-MD-012 — Global put on empty DB → Ok.
// ---------------------------------------------------------------------

/// Assert the `put` failed with `ObjectNotFound` carrying exactly
/// `expected_kind` — a kind-swap bug in `classify_put_error`/`ScopeSql`
/// must fail here, not just any FK violation.
fn assert_object_not_found(res: Result<(), KvError>, expected_kind: ObjectKind) {
    match res {
        Err(KvError::ObjectNotFound { kind }) => assert_eq!(
            kind, expected_kind,
            "ObjectNotFound carried {kind:?}, expected {expected_kind:?}"
        ),
        other => panic!("expected ObjectNotFound {{ {expected_kind:?} }}, got {other:?}"),
    }
}

#[test]
fn tc_md_007_put_wallet_absent_parent() {
    let (p, _tmp, _path) = fresh_persister();
    assert_object_not_found(
        p.put(&ObjectId::Wallet(wid(0xAB)), "k", b"v"),
        ObjectKind::Wallet,
    );
}

#[test]
fn tc_md_008_put_identity_absent_parent() {
    let (p, _tmp, _path) = fresh_persister();
    assert_object_not_found(
        p.put(&ObjectId::Identity(id32(0xAC)), "k", b"v"),
        ObjectKind::Identity,
    );
}

#[test]
fn tc_md_009_put_token_absent_parent() {
    let (p, _tmp, _path) = fresh_persister();
    assert_object_not_found(
        p.put(
            &ObjectId::Token {
                identity_id: id32(0xAD),
                token_id: id32(0xAE),
            },
            "k",
            b"v",
        ),
        ObjectKind::Token,
    );
}

#[test]
fn tc_md_010_put_contact_absent_parent() {
    let (p, _tmp, _path) = fresh_persister();
    assert_object_not_found(
        p.put(
            &ObjectId::Contact {
                wallet_id: wid(0xAF),
                owner_id: id32(0xB0),
                contact_id: id32(0xB1),
            },
            "k",
            b"v",
        ),
        ObjectKind::Contact,
    );
}

#[test]
fn tc_md_011_put_platform_address_absent_parent() {
    let (p, _tmp, _path) = fresh_persister();
    assert_object_not_found(
        p.put(
            &ObjectId::PlatformAddress {
                wallet_id: wid(0xB2),
                address: vec![0x01, 0x02, 0x03],
            },
            "k",
            b"v",
        ),
        ObjectKind::PlatformAddress,
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

// ---------------------------------------------------------------------
// QA-002 — delete of a never-existing key is idempotent (returns Ok),
// for both a no-FK scope and an FK scope.
// ---------------------------------------------------------------------

#[test]
fn delete_missing_key_is_idempotent() {
    let (p, _tmp, _path) = fresh_persister();
    p.delete(&ObjectId::Global, "never-existed").unwrap();
    let w = wid(0x90);
    ensure_wallet_meta(&p, &w);
    p.delete(&ObjectId::Wallet(w), "never-existed").unwrap();
}

// ---------------------------------------------------------------------
// QA-003 — list_keys returns keys in ascending order regardless of
// insertion order.
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// TC-MD-013..016 — native per-object cascade: seed+put, DELETE FROM the
// direct parent table, assert the meta row is gone.
// ---------------------------------------------------------------------

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
            "DELETE FROM contacts_established \
             WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3",
            params![w.as_slice(), &owner[..], &contact[..]],
        )
        .expect("delete contact_established");
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

// ---------------------------------------------------------------------
// TC-MD-017 / 017b — wallet cascade (direct + transitive via identities).
// ---------------------------------------------------------------------

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
            "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .expect("delete wallet_metadata");
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
            "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .expect("delete wallet_metadata");
    }
    // wallet → identities → meta_identity cascade chain.
    assert_eq!(p.get(&scope, "k").unwrap(), None);
}

// ---------------------------------------------------------------------
// TC-MD-018 / 019 — delete_wallet purges every meta_* for the wallet;
// Global + other wallet's meta_wallet survive; report wiring.
// ---------------------------------------------------------------------

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

    let report = p.delete_wallet(a).expect("delete_wallet");
    let counts = &report.rows_removed_per_table;
    for table in [
        "meta_wallet",
        "meta_identity",
        "meta_token",
        "meta_contact",
        "meta_platform_address",
    ] {
        assert_eq!(
            counts.get(table).copied(),
            Some(1),
            "{table} should report one removed meta row"
        );
    }
    assert!(
        !counts.contains_key("meta_global"),
        "meta_global must not appear in the per-wallet delete report"
    );
}

// ---------------------------------------------------------------------
// TC-MD-020..022 — key bounds.
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// TC-MD-023 — oversized value planted directly is rejected on `get`
// before materialisation, across every meta_* table.
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// TC-MD-024 — list_keys prefix with literal `%`/`_`/`\` (not wildcards).
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// TC-MD-025 — scope isolation: same key string across Wallet(A)/Wallet(B)
// and Global/Wallet(A) stays independent.
// ---------------------------------------------------------------------

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
