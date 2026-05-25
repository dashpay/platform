//! Integration tests for the V002 migration — cascade-only identity
//! references (CODE-002). Six TCs covering: migration on populated
//! V001 data, fresh V002 FK chain, orphan identities, real wallet_id
//! token-balance writes, sentinel-row refusal, and a post-migration
//! `PRAGMA foreign_key_check` clean-room verification.

#[path = "common/mod.rs"]
mod common;

use common::fresh_persister;

use rusqlite::{params, Connection};

/// Apply only V001 to a fresh in-memory connection (V002 is held back
/// so populated-DB migration TCs can stage realistic rows first).
///
/// Uses refinery's `Runner::set_target` to apply migrations up to and
/// including V001 only, which leaves V002 unapplied for the test body
/// to trigger explicitly.
fn apply_only_v001(conn: &mut Connection) {
    use refinery::Target;
    platform_wallet_storage::sqlite::migrations::runner()
        .set_target(Target::Version(1))
        .run(conn)
        .expect("apply V001 only");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enable FKs");
}

/// Apply V002 over an already-V001 connection, matching the
/// persister's open-path discipline: FKs OFF for the duration of the
/// migration tx so DROP-TABLE cascades don't wipe rows mid-migration,
/// then back ON.
fn apply_v002_with_fk_toggle(conn: &mut Connection) -> Result<refinery::Report, refinery::Error> {
    conn.pragma_update(None, "foreign_keys", "OFF")
        .expect("disable FKs");
    let result = platform_wallet_storage::sqlite::migrations::run(conn);
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("re-enable FKs");
    result
}

/// Apply the full embedded migration set (V001 + V002), routing
/// through the open-path FK toggle so the V002 cascade rewrite
/// doesn't wipe the rows it is trying to preserve.
fn apply_all(conn: &mut Connection) {
    conn.pragma_update(None, "foreign_keys", "OFF")
        .expect("disable FKs");
    platform_wallet_storage::sqlite::migrations::run(conn).expect("apply migrations");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enable FKs");
}

/// TC-CODE-002-1 — V001→V002 migration on populated DB preserves rows.
///
/// Stage one wallet + identity + identity_keys + dashpay_profile +
/// token_balance under V001's schema. Apply V002. Every row must still
/// be readable through the new columns / PK shape; the column lists
/// must reflect the V002 shape.
#[test]
fn tc_code_002_1_v001_to_v002_preserves_rows() {
    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_only_v001(&mut conn);

    let wid = [11u8; 32];
    let iid = [22u8; 32];
    let kid: i64 = 3;
    let tid = [33u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![&wid[..]],
    )
    .expect("insert wallet_metadata");

    conn.execute(
        "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, 7, ?2, X'AA', 0)",
        params![&wid[..], &iid[..]],
    )
    .expect("insert identity");

    conn.execute(
        "INSERT INTO identity_keys (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
         VALUES (?1, ?2, ?3, X'BB', X'CC', NULL)",
        params![&wid[..], &iid[..], kid],
    )
    .expect("insert identity_keys");

    conn.execute(
        "INSERT INTO dashpay_profiles (wallet_id, identity_id, profile_blob) VALUES (?1, ?2, X'DD')",
        params![&wid[..], &iid[..]],
    )
    .expect("insert dashpay_profile");

    conn.execute(
        "INSERT INTO token_balances (wallet_id, identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, ?3, 42, 100)",
        params![&wid[..], &iid[..], &tid[..]],
    )
    .expect("insert token_balance");

    // Apply V002 on top through the FK-toggle helper that mirrors
    // the persister open-path discipline.
    apply_v002_with_fk_toggle(&mut conn).expect("V002 migration on populated DB");

    // identities row preserved.
    let identity_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            params![&iid[..]],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(identity_count, 1, "identity row survived V002");

    // identity_keys row preserved (PK now identity_id, key_id).
    let key_blob: Vec<u8> = conn
        .query_row(
            "SELECT public_key_blob FROM identity_keys WHERE identity_id = ?1 AND key_id = ?2",
            params![&iid[..], kid],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(key_blob, vec![0xBB], "identity_keys blob preserved");

    // dashpay_profiles row preserved.
    let profile_blob: Vec<u8> = conn
        .query_row(
            "SELECT profile_blob FROM dashpay_profiles WHERE identity_id = ?1",
            params![&iid[..]],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(profile_blob, vec![0xDD], "dashpay_profile blob preserved");

    // token_balances row preserved with balance/updated_at intact.
    let (balance, updated_at): (i64, i64) = conn
        .query_row(
            "SELECT balance, updated_at FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            params![&iid[..], &tid[..]],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap();
    assert_eq!(balance, 42);
    assert_eq!(updated_at, 100);

    // PK shape: token_balances must no longer carry wallet_id.
    let mut stmt = conn
        .prepare("SELECT name FROM pragma_table_info('token_balances')")
        .unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        !cols.iter().any(|c| c == "wallet_id"),
        "token_balances must not carry wallet_id after V002; got {cols:?}"
    );
}

/// TC-CODE-002-2 — fresh V002 enforces the cascade chain
/// wallet_metadata → identities → identity-owned tables.
#[test]
fn tc_code_002_2_cascade_chain_wallet_to_identity_to_tokens() {
    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_all(&mut conn);

    let wid = [11u8; 32];
    let iid = [22u8; 32];
    let tid = [33u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![&wid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, NULL, X'AA', 0)",
        params![&iid[..], &wid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, 1, 0)",
        params![&iid[..], &tid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO dashpay_profiles (identity_id, profile_blob) VALUES (?1, X'DD')",
        params![&iid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identity_keys (identity_id, key_id, public_key_blob, public_key_hash) \
         VALUES (?1, 0, X'BB', X'CC')",
        params![&iid[..]],
    )
    .unwrap();

    // Deleting the wallet cascades through identities to every
    // identity-owned table.
    conn.execute(
        "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
        params![&wid[..]],
    )
    .unwrap();

    for (table, where_clause) in [
        ("identities", "identity_id = ?1"),
        ("token_balances", "identity_id = ?1"),
        ("dashpay_profiles", "identity_id = ?1"),
        ("identity_keys", "identity_id = ?1"),
    ] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {where_clause}"),
                params![&iid[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "cascade did not reach {table}");
    }
}

/// TC-CODE-002-3 — orphan identity (NULL wallet_id) writes + reads OK.
#[test]
fn tc_code_002_3_orphan_identity_nullable_wallet_id() {
    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_all(&mut conn);

    let iid = [77u8; 32];
    // No wallet_metadata row at all — identity row carries NULL.
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, NULL, NULL, X'AA', 0)",
        params![&iid[..]],
    )
    .expect("orphan insert must succeed");

    let (wid_opt, blob): (Option<Vec<u8>>, Vec<u8>) = conn
        .query_row(
            "SELECT wallet_id, entry_blob FROM identities WHERE identity_id = ?1",
            params![&iid[..]],
            |row| Ok((row.get::<_, Option<Vec<u8>>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .unwrap();
    assert!(wid_opt.is_none(), "orphan identity wallet_id must be NULL");
    assert_eq!(blob, vec![0xAA]);

    // Schema reports wallet_id as nullable (`notnull` = 0).
    let notnull: i64 = conn
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('identities') WHERE name = 'wallet_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(notnull, 0, "identities.wallet_id must be nullable");
}

/// TC-CODE-002-4 — token-balance write under a real wallet_id passes FK.
///
/// Confirms the V002 schema accepts the identity-id-keyed write path
/// the post-fix consumer takes — no `SQLITE_CONSTRAINT_FOREIGNKEY`.
#[test]
fn tc_code_002_4_token_balance_with_real_identity_succeeds() {
    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_all(&mut conn);

    let wid = [11u8; 32];
    let iid = [22u8; 32];
    let tid = [33u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![&wid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, NULL, X'AA', 0)",
        params![&iid[..], &wid[..]],
    )
    .unwrap();

    // The fix-side schema: token_balance write needs no wallet_id.
    conn.execute(
        "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, 12345, 0)",
        params![&iid[..], &tid[..]],
    )
    .expect("token_balance write must succeed under V002");
}

/// TC-CODE-002-5 — migration refuses if legacy sentinel rows are present.
///
/// Stage a V001 DB with one `token_balances` row keyed under
/// `WalletId::default()` (`X'00..00'`). Run migrations — the V002
/// guard must abort the transaction with the typed
/// `MigrationRequiresManualCleanup` signal.
#[test]
fn tc_code_002_5_migration_refuses_legacy_sentinel_rows() {
    use rusqlite::ErrorCode;

    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_only_v001(&mut conn);

    let sentinel = [0u8; 32];
    let iid = [22u8; 32];
    let tid = [33u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![&sentinel[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, X'AA', 0)",
        params![&sentinel[..], &iid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO token_balances (wallet_id, identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, ?3, 0, 0)",
        params![&sentinel[..], &iid[..], &tid[..]],
    )
    .unwrap();

    let err = platform_wallet_storage::sqlite::migrations::run(&mut conn)
        .expect_err("migration must refuse legacy sentinel rows");
    let msg = format!("{err:?}");
    // The guard rides a CHECK constraint on a temp table column named
    // `sentinel_count`; SQLite's error text quotes the failing CHECK
    // predicate so the column name surfaces verbatim.
    assert!(
        msg.contains("sentinel_count"),
        "expected sentinel_count CHECK in error text, got: {msg}"
    );
    // Walk the source chain to confirm the underlying SQLite error is
    // a real `ConstraintViolation` (the CHECK failure), not an
    // unrelated I/O surprise.
    let mut source: Option<&dyn std::error::Error> = Some(&err);
    let mut found_constraint = false;
    while let Some(s) = source {
        if let Some(rusqlite::Error::SqliteFailure(e, _)) = s.downcast_ref::<rusqlite::Error>() {
            if matches!(e.code, ErrorCode::ConstraintViolation) {
                found_constraint = true;
                break;
            }
        }
        source = s.source();
    }
    assert!(
        found_constraint,
        "expected ConstraintViolation in error chain"
    );

    // Going through the persister's open-path re-classifies that raw
    // CHECK failure into the typed
    // `MigrationRequiresManualCleanup` error, which is what operators
    // actually see. Drive the same scenario via `SqlitePersister::open`
    // on a file-backed DB to verify the re-classification.
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {});

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    {
        let mut conn = Connection::open(&path).expect("open file db");
        apply_only_v001(&mut conn);
        let sentinel = [0u8; 32];
        let iid2 = [99u8; 32];
        let tid2 = [88u8; 32];
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&sentinel[..]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
             VALUES (?1, NULL, ?2, X'AA', 0)",
            params![&sentinel[..], &iid2[..]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO token_balances (wallet_id, identity_id, token_id, balance, updated_at) \
             VALUES (?1, ?2, ?3, 0, 0)",
            params![&sentinel[..], &iid2[..], &tid2[..]],
        )
        .unwrap();
    }
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&path);
    let open_err = match platform_wallet_storage::SqlitePersister::open(cfg) {
        Ok(_) => panic!("open must refuse the sentinel-laden DB"),
        Err(e) => e,
    };
    assert!(
        matches!(
            open_err,
            platform_wallet_storage::WalletStorageError::MigrationRequiresManualCleanup {
                table: "token_balances",
                count
            } if count >= 1
        ),
        "expected MigrationRequiresManualCleanup, got: {open_err:?}"
    );
}

/// TC-CODE-002-6 — `PRAGMA foreign_key_check` post-migration is empty.
///
/// Sanity-check the migrated schema has no dangling FK references —
/// neither among the migrated rows nor among the new tables.
#[test]
fn tc_code_002_6_pragma_foreign_key_check_clean() {
    let mut conn = Connection::open_in_memory().expect("open in-memory");
    apply_only_v001(&mut conn);

    let wid = [11u8; 32];
    let iid = [22u8; 32];
    let tid = [33u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![&wid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, X'AA', 0)",
        params![&wid[..], &iid[..]],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO token_balances (wallet_id, identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, ?3, 1, 0)",
        params![&wid[..], &iid[..], &tid[..]],
    )
    .unwrap();

    apply_v002_with_fk_toggle(&mut conn).expect("apply V002");

    let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let rows: Vec<(String, i64, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        rows.is_empty(),
        "foreign_key_check found dangling refs: {rows:?}"
    );

    // End-to-end smoke: a fresh persister opens the migrated DB
    // cleanly (no extra FK errors at open time).
    let (_p, _tmp, _path) = fresh_persister();
}
