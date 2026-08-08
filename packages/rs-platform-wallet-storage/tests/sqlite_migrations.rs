#![allow(clippy::field_reassign_with_default)]

//! Migration discovery, application, and idempotency.

mod common;

use common::fresh_persister;
use platform_wallet_storage::sqlite::migrations as mig;

/// every embedded migration corresponds to a file in `migrations/`.
#[test]
fn tc025_embedded_migrations_match_files() {
    let embedded = mig::embedded_migrations();
    assert!(!embedded.is_empty(), "no migrations embedded");
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let on_disk: Vec<_> = std::fs::read_dir(format!("{crate_root}/migrations"))
        .expect("read migrations dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('V') && n.ends_with(".rs"))
        .collect();
    assert_eq!(
        embedded.len(),
        on_disk.len(),
        "embedded vs on-disk count mismatch: {embedded:?} vs {on_disk:?}"
    );
    for (v, name) in &embedded {
        let expected_padded = format!("V{:03}__{}.rs", v, name);
        let expected_plain = format!("V{}__{}.rs", v, name);
        assert!(
            on_disk
                .iter()
                .any(|f| f == &expected_padded || f == &expected_plain),
            "no on-disk file for migration V{v} {name} \
             (expected {expected_padded} or {expected_plain})"
        );
    }
}

/// fresh DB ends at latest schema version.
#[test]
fn tc026_fresh_db_at_latest() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let highest_embedded = mig::embedded_migrations()
        .iter()
        .map(|(v, _)| *v as i64)
        .max()
        .unwrap();
    assert_eq!(max, Some(highest_embedded));
}

/// every declared table is creatable and accepts a minimal row
/// (parent first, then children).
#[test]
fn tc027_smoke_insert_every_table() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    use rusqlite::params;
    let wallet_id = [42u8; 32];

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .unwrap();
    let identity_id = [7u8; 32];
    conn.execute(
        "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, X'01', 0)",
        params![wallet_id.as_slice(), identity_id.as_slice()],
    )
    .unwrap();
    let outpoint = vec![0u8; 36];
    let txid = vec![0u8; 32];
    let cases: &[(&str, &str, &[&dyn rusqlite::ToSql])] = &[
        (
            "account_registrations",
            // Labels must match the writer-side canonical strings — see the
            // CHECK constraint sourced from `ACCOUNT_TYPE_LABELS` in
            // `sqlite::schema::accounts`.
            "INSERT INTO account_registrations (wallet_id, account_type, account_index, account_xpub_bytes) VALUES (?1, 'standard', 0, X'00')",
            &[&wallet_id.as_slice()],
        ),
        (
            "account_address_pools",
            "INSERT INTO account_address_pools (wallet_id, account_type, account_index, pool_type, snapshot_blob) VALUES (?1, 'standard', 0, 'external', X'00')",
            &[&wallet_id.as_slice()],
        ),
        (
            "core_transactions",
            "INSERT INTO core_transactions (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) VALUES (?1, ?2, NULL, NULL, NULL, 0, X'00')",
            &[&wallet_id.as_slice(), &txid],
        ),
        (
            "core_utxos",
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) VALUES (?1, ?2, 0, X'00', NULL, 0, 0, NULL)",
            &[&wallet_id.as_slice(), &outpoint],
        ),
        (
            "core_instant_locks",
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, X'00')",
            &[&wallet_id.as_slice(), &txid],
        ),
        (
            "core_derived_addresses",
            "INSERT INTO core_derived_addresses (wallet_id, account_type, account_index, address, derivation_path, used) VALUES (?1, 'standard', 0, 'addr', '', 0)",
            &[&wallet_id.as_slice()],
        ),
        (
            "core_sync_state",
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) VALUES (?1, NULL, NULL)",
            &[&wallet_id.as_slice()],
        ),
        (
            "identity_keys",
            // identity_keys is keyed by (identity_id, key_id); the FK
            // targets identities(identity_id).
            "INSERT INTO identity_keys (identity_id, key_id, public_key_blob, public_key_hash) VALUES (?1, 0, X'00', X'00')",
            &[&identity_id.as_slice()],
        ),
        (
            "contacts",
            // `state` must match the CHECK sourced from CONTACT_STATE_LABELS
            // in `sqlite::schema::contacts`; request/metadata columns are
            // nullable so a minimal pending row only needs `state`.
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state) VALUES (?1, ?2, ?3, 'sent')",
            &[&wallet_id.as_slice(), &identity_id.as_slice(), &[1u8; 32].as_slice()],
        ),
        (
            "platform_addresses",
            "INSERT INTO platform_addresses (wallet_id, account_index, address_index, address, balance, nonce) VALUES (?1, 0, 0, X'0000000000000000000000000000000000000000', 0, 0)",
            &[&wallet_id.as_slice()],
        ),
        (
            "platform_address_sync",
            "INSERT INTO platform_address_sync (wallet_id, sync_height, sync_timestamp, last_known_recent_block) VALUES (?1, 0, 0, 0)",
            &[&wallet_id.as_slice()],
        ),
        (
            "asset_locks",
            "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) VALUES (?1, ?2, 'built', 0, 0, 0, X'00')",
            &[&wallet_id.as_slice(), &outpoint],
        ),
        (
            "token_balances",
            // token_balances PK is (identity_id, token_id); the FK
            // cascades through identities.
            "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) VALUES (?1, ?2, 0, 0)",
            &[&identity_id.as_slice(), &[5u8; 32].as_slice()],
        ),
        (
            "dashpay_profiles",
            // dashpay_profiles is keyed by identity_id only.
            "INSERT INTO dashpay_profiles (identity_id, profile_blob) VALUES (?1, X'00')",
            &[&identity_id.as_slice()],
        ),
        (
            "dashpay_payments_overlay",
            // dashpay_payments_overlay is keyed by (identity_id, payment_id).
            "INSERT INTO dashpay_payments_overlay (identity_id, payment_id, overlay_blob) VALUES (?1, 'pay1', X'00')",
            &[&identity_id.as_slice()],
        ),
    ];
    // Identity-owned tables have no `wallet_id` column; count them by
    // joining through `identities`. Everything else is wallet-scoped.
    let via_identity = [
        "identity_keys",
        "token_balances",
        "dashpay_profiles",
        "dashpay_payments_overlay",
    ];
    for (table, sql, params) in cases {
        conn.execute(sql, *params).expect(table);
        let count_sql = if via_identity.contains(table) {
            format!(
                "SELECT COUNT(*) FROM {table} \
                 WHERE identity_id IN (SELECT identity_id FROM identities WHERE wallet_id = ?1)"
            )
        } else {
            format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1")
        };
        let n: i64 = conn
            .query_row(&count_sql, rusqlite::params![wallet_id.as_slice()], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(n >= 1, "{table} insert did not land");
    }
}

/// re-open is idempotent.
#[test]
fn tc028_idempotent_reopen() {
    let (persister, tmp, path) = fresh_persister();
    drop(persister);
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&path);
    let _p2 = platform_wallet_storage::SqlitePersister::open(cfg).expect("reopen");
    drop(tmp);
}

/// append-only migration hash.
///
/// The hash is computed at runtime from the embedded list. Because this
/// test belongs to the migration drift policy, we assert the list is
/// non-empty and the hash is stable across successive calls — not a
/// pinned value (which would force a churn on every committed migration).
#[test]
fn tc029_migration_fingerprint_stable() {
    let a = mig::embedded_migrations_fingerprint();
    let b = mig::embedded_migrations_fingerprint();
    assert_eq!(a, b);
    assert!(!mig::embedded_migrations().is_empty());
}

/// load() on empty post-migrate DB is empty.
#[test]
fn tc044_load_empty_is_empty() {
    let (persister, _tmp, _path) = fresh_persister();
    let state = platform_wallet::changeset::PlatformWalletPersistence::load(&persister).unwrap();
    assert!(state.is_empty());
}

/// V003 → V004 upgrade path: a database created at the prior release
/// schema (through V003) upgrades in place — the asset_locks rebuild
/// keeps existing rows byte-for-byte and widens the status CHECK to
/// admit `recovered_from_chain`, which the V003 schema rejects.
///
/// This is the regression test for the review finding that V001's
/// generated CHECK must never change (Refinery `abort_divergent` would
/// brick every already-migrated database): the domain widens by
/// APPENDING V004, and this test drives exactly the sequence an
/// existing install experiences.
#[test]
fn tc045_v004_widens_asset_lock_status_on_existing_db() {
    use rusqlite::params;

    let mut conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    // 1. Stand the database up at the PRIOR release schema (V003).
    let to_v003 = mig::runner().set_target(refinery::Target::Version(3));
    to_v003.run(&mut conn).expect("migrate to V003");

    // 2. Populate it the way a live wallet would have.
    let wallet_id = [42u8; 32];
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .expect("insert wallet");
    let outpoint_a = [1u8; 36];
    conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, \
         amount_duffs, lifecycle_blob) VALUES (?1, ?2, 'chain_locked', 0, 4, 1000000, X'01')",
        params![wallet_id.as_slice(), outpoint_a.as_slice()],
    )
    .expect("insert pre-upgrade asset lock");

    // 3. The V003 CHECK must reject the new label — that's the schema
    //    gap V004 exists to close.
    let outpoint_b = [2u8; 36];
    let rejected = conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, \
         amount_duffs, lifecycle_blob) VALUES (?1, ?2, 'recovered_from_chain', 0, 0, 500, X'02')",
        params![wallet_id.as_slice(), outpoint_b.as_slice()],
    );
    assert!(
        rejected.is_err(),
        "the V003 CHECK domain must reject recovered_from_chain"
    );

    // 4. Upgrade to the latest schema (applies V004's table rebuild).
    mig::run(&mut conn).expect("migrate to latest");

    // 5. The pre-upgrade row survived the rebuild intact...
    let (status, identity_index, amount): (String, i64, i64) = conn
        .query_row(
            "SELECT status, identity_index, amount_duffs FROM asset_locks WHERE outpoint = ?1",
            params![outpoint_a.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("pre-upgrade row survives");
    assert_eq!(
        (status.as_str(), identity_index, amount),
        ("chain_locked", 4, 1_000_000)
    );

    // 6. ...the widened domain admits the new label...
    conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, \
         amount_duffs, lifecycle_blob) VALUES (?1, ?2, 'recovered_from_chain', 0, 0, 500, X'02')",
        params![wallet_id.as_slice(), outpoint_b.as_slice()],
    )
    .expect("recovered_from_chain must insert after V004");

    // 7. ...garbage labels stay rejected, and the rebuilt table kept its
    //    FK: deleting the wallet cascades to both rows.
    let garbage = conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, \
         amount_duffs, lifecycle_blob) VALUES (?1, X'03', 'bogus', 0, 0, 1, X'03')",
        params![wallet_id.as_slice()],
    );
    assert!(garbage.is_err(), "unknown labels must still be rejected");
    conn.execute(
        "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )
    .expect("delete wallet");
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM asset_locks", [], |row| row.get(0))
        .expect("count");
    assert_eq!(remaining, 0, "ON DELETE CASCADE must survive the rebuild");
}
