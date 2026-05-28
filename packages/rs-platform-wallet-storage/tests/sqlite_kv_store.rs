//! Integration tests for the generic KV store on `SqlitePersister`.
//!
//! Covers the public [`KvStore`] surface (`get` / `put` / `delete` /
//! `list_keys`) for both global (`wallet_id = None`) and per-wallet
//! (`wallet_id = Some(id)`) scopes, plus the boundary behaviours:
//! coexistence of the two scopes under the same key string, upsert
//! semantics, idempotent delete, cascade on wallet delete, key length
//! validation, and the typed `WalletNotFound` path for FK violations.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use platform_wallet_storage::kv::MAX_KEY_LEN;
use platform_wallet_storage::{KvError, KvStore};

#[test]
fn kv_round_trip_global() {
    let (p, _tmp, _path) = fresh_persister();
    assert_eq!(p.get(None, "app.config").unwrap(), None);
    p.put(None, "app.config", b"{}").unwrap();
    assert_eq!(
        p.get(None, "app.config").unwrap().as_deref(),
        Some(&b"{}"[..])
    );
    p.delete(None, "app.config").unwrap();
    assert_eq!(p.get(None, "app.config").unwrap(), None);
}

#[test]
fn kv_round_trip_per_wallet() {
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(1);
    ensure_wallet_meta(&p, &id);
    assert_eq!(p.get(Some(&id), "ui.theme").unwrap(), None);
    p.put(Some(&id), "ui.theme", b"dark").unwrap();
    assert_eq!(
        p.get(Some(&id), "ui.theme").unwrap().as_deref(),
        Some(&b"dark"[..])
    );
    p.delete(Some(&id), "ui.theme").unwrap();
    assert_eq!(p.get(Some(&id), "ui.theme").unwrap(), None);
}

#[test]
fn kv_global_and_wallet_keys_coexist() {
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(2);
    ensure_wallet_meta(&p, &id);
    p.put(None, "same", b"global").unwrap();
    p.put(Some(&id), "same", b"scoped").unwrap();
    assert_eq!(
        p.get(None, "same").unwrap().as_deref(),
        Some(&b"global"[..])
    );
    assert_eq!(
        p.get(Some(&id), "same").unwrap().as_deref(),
        Some(&b"scoped"[..])
    );
}

#[test]
fn kv_list_keys_with_prefix() {
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(3);
    ensure_wallet_meta(&p, &id);
    for k in ["app.x", "app.y", "app.z", "ui.a", "ui.b"] {
        p.put(Some(&id), k, b"v").unwrap();
    }
    let keys = p.list_keys(Some(&id), Some("app.")).unwrap();
    assert_eq!(keys, vec!["app.x", "app.y", "app.z"]);
    let keys = p.list_keys(Some(&id), Some("ui.")).unwrap();
    assert_eq!(keys, vec!["ui.a", "ui.b"]);
    // Unmatched prefix returns empty without erroring.
    assert!(p
        .list_keys(Some(&id), Some("no_match."))
        .unwrap()
        .is_empty());
}

#[test]
fn kv_list_keys_no_prefix() {
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(4);
    ensure_wallet_meta(&p, &id);
    for k in ["b", "a", "c"] {
        p.put(Some(&id), k, b"v").unwrap();
    }
    let keys = p.list_keys(Some(&id), None).unwrap();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn kv_list_keys_scope_isolation() {
    // Per-wallet listing must NOT include global keys, and vice versa.
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(5);
    ensure_wallet_meta(&p, &id);
    p.put(None, "g1", b"v").unwrap();
    p.put(None, "g2", b"v").unwrap();
    p.put(Some(&id), "w1", b"v").unwrap();
    let global = p.list_keys(None, None).unwrap();
    let scoped = p.list_keys(Some(&id), None).unwrap();
    assert_eq!(global, vec!["g1", "g2"]);
    assert_eq!(scoped, vec!["w1"]);
}

#[test]
fn kv_list_keys_escapes_like_metacharacters() {
    // A `%` or `_` in a stored key must NOT be treated as a wildcard
    // when it appears literally in the user-supplied prefix.
    let (p, _tmp, _path) = fresh_persister();
    p.put(None, "100%cotton", b"v").unwrap();
    p.put(None, "abc", b"v").unwrap();
    let keys = p.list_keys(None, Some("100%")).unwrap();
    assert_eq!(keys, vec!["100%cotton"]);
    // Prefix `1%` (literal `1` + literal `%`) must NOT match `100%cotton`
    // — without escaping, the `%` would be treated as a wildcard.
    let keys = p.list_keys(None, Some("1%")).unwrap();
    assert!(
        keys.is_empty(),
        "wildcard `%` must be escaped in the prefix; got {keys:?}"
    );
}

#[test]
fn kv_put_overwrites_existing() {
    let (p, _tmp, _path) = fresh_persister();
    p.put(None, "k", b"v1").unwrap();
    p.put(None, "k", b"v2").unwrap();
    p.put(None, "k", b"v3").unwrap();
    assert_eq!(p.get(None, "k").unwrap().as_deref(), Some(&b"v3"[..]));
    // Exactly one row survives despite three upserts.
    let n = p.list_keys(None, None).unwrap().len();
    assert_eq!(n, 1);
}

#[test]
fn kv_delete_idempotent() {
    let (p, _tmp, _path) = fresh_persister();
    // Delete on absent key is Ok(())
    p.delete(None, "missing").unwrap();
    let id = wid(6);
    ensure_wallet_meta(&p, &id);
    p.delete(Some(&id), "missing").unwrap();
}

#[test]
fn kv_cascades_on_wallet_delete() {
    use rusqlite::params;
    let (p, _tmp, _path) = fresh_persister();
    let id = wid(9);
    ensure_wallet_meta(&p, &id);
    p.put(None, "global_survivor", b"keep").unwrap();
    p.put(Some(&id), "wallet_local_1", b"goes").unwrap();
    p.put(Some(&id), "wallet_local_2", b"too").unwrap();
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
            params![id.as_slice()],
        )
        .expect("delete wallet");
    }
    assert!(p.list_keys(Some(&id), None).unwrap().is_empty());
    assert_eq!(
        p.get(None, "global_survivor").unwrap().as_deref(),
        Some(&b"keep"[..])
    );
}

#[test]
fn kv_rejects_too_long_key() {
    let (p, _tmp, _path) = fresh_persister();
    let too_long = "a".repeat(MAX_KEY_LEN + 1);
    match p.put(None, &too_long, b"v") {
        Err(KvError::KeyTooLong { len }) => assert_eq!(len, MAX_KEY_LEN + 1),
        other => panic!("expected KeyTooLong, got {other:?}"),
    }
    // get / delete / list — same validation
    match p.get(None, &too_long) {
        Err(KvError::KeyTooLong { .. }) => {}
        other => panic!("expected KeyTooLong on get, got {other:?}"),
    }
    match p.delete(None, &too_long) {
        Err(KvError::KeyTooLong { .. }) => {}
        other => panic!("expected KeyTooLong on delete, got {other:?}"),
    }
}

#[test]
fn kv_rejects_empty_key() {
    let (p, _tmp, _path) = fresh_persister();
    match p.put(None, "", b"v") {
        Err(KvError::KeyEmpty) => {}
        other => panic!("expected KeyEmpty, got {other:?}"),
    }
    match p.get(None, "") {
        Err(KvError::KeyEmpty) => {}
        other => panic!("expected KeyEmpty on get, got {other:?}"),
    }
}

#[test]
fn kv_accepts_max_length_key() {
    let (p, _tmp, _path) = fresh_persister();
    let max_key = "a".repeat(MAX_KEY_LEN);
    p.put(None, &max_key, b"v").unwrap();
    assert_eq!(p.get(None, &max_key).unwrap().as_deref(), Some(&b"v"[..]));
}

#[test]
fn kv_put_for_nonexistent_wallet_errors() {
    let (p, _tmp, _path) = fresh_persister();
    let ghost = wid(0xAB); // never seeded into wallet_metadata
    match p.put(Some(&ghost), "k", b"v") {
        Err(KvError::WalletNotFound { wallet_id }) => assert_eq!(wallet_id, ghost),
        other => panic!("expected WalletNotFound, got {other:?}"),
    }
}

#[test]
fn kv_empty_value_is_allowed() {
    // An empty `Vec<u8>` is a legitimate BLOB. Round-trip must
    // distinguish "present and empty" from "absent".
    let (p, _tmp, _path) = fresh_persister();
    p.put(None, "k", &[]).unwrap();
    assert_eq!(p.get(None, "k").unwrap().as_deref(), Some(&[][..]));
}
