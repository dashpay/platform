#![allow(clippy::field_reassign_with_default)]

//! Cross-wallet isolation + delete cascade for the new V003 tables
//! (`core_address_pool`, `meta_data_versions`) — TC-B-006. Two wallets with
//! fully-overlapping keys must not collide, must not leak across wallets, and
//! deleting one must leave the other's V003 rows intact.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::wallet::platform_wallet::WalletId;

fn pool_count(conn: &rusqlite::Connection, w: &WalletId) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
        |r| r.get(0),
    )
    .unwrap()
}

fn versions_count(conn: &rusqlite::Connection, w: &WalletId) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM meta_data_versions WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
        |r| r.get(0),
    )
    .unwrap()
}

/// TC-B-006 — overlapping keys across two wallets coexist without PK
/// collision, and deleting wallet A cascades away only A's V003 rows while
/// wallet B's survive intact.
#[test]
fn tc_b_006_v003_tables_isolate_and_cascade_per_wallet() {
    let (persister, _tmp, _path) = fresh_persister();
    let a: WalletId = wid(0x0A);
    let b: WalletId = wid(0x0B);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);

    // Identical (account_type, account_index, key_class, pool_type,
    // address_index, domain) for both wallets — only wallet_id differs.
    {
        let conn = persister.lock_conn_for_test();
        for w in [&a, &b] {
            conn.execute(
                "INSERT INTO core_address_pool \
                    (wallet_id, account_type, account_index, key_class, pool_type, \
                     address_index, script, used) \
                 VALUES (?1, 'standard_bip44', 0, 0, 0, 0, ?2, 1)",
                rusqlite::params![w.as_slice(), &[0xEEu8; 25][..]],
            )
            .expect("overlapping-key pool rows must not collide across wallets");
            conn.execute(
                "INSERT INTO meta_data_versions (wallet_id, domain, seq) \
                 VALUES (?1, 'core', 3)",
                rusqlite::params![w.as_slice()],
            )
            .expect("overlapping-domain version rows must not collide across wallets");
        }

        // No cross-wallet read leakage: each wallet sees exactly its own row.
        assert_eq!(pool_count(&conn, &a), 1);
        assert_eq!(pool_count(&conn, &b), 1);
        assert_eq!(versions_count(&conn, &a), 1);
        assert_eq!(versions_count(&conn, &b), 1);
    }

    // Delete wallet A — FK ON DELETE CASCADE (core_address_pool) and the
    // meta_data_versions soft-cascade trigger must reap only A's rows.
    persister.delete_wallet_skip_backup(a).expect("delete A");

    let conn = persister.lock_conn_for_test();
    assert_eq!(pool_count(&conn, &a), 0, "A's pool rows cascade-deleted");
    assert_eq!(
        versions_count(&conn, &a),
        0,
        "A's version rows removed by the delete trigger"
    );
    assert_eq!(pool_count(&conn, &b), 1, "B's pool rows survive A's delete");
    assert_eq!(
        versions_count(&conn, &b),
        1,
        "B's version rows survive A's delete"
    );
}
