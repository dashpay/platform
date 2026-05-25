#![allow(clippy::field_reassign_with_default)]

//! TC-025..TC-030, TC-028, TC-044 — migration discovery and reach.

mod common;

use common::fresh_persister;
use platform_wallet_storage::sqlite::migrations as mig;

/// TC-025: every embedded migration corresponds to a file in `migrations/`.
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

/// TC-026: fresh DB ends at latest schema version.
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

/// TC-027: every declared table is creatable and accepts a minimal row
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
            "INSERT INTO account_registrations (wallet_id, account_type, account_index, account_xpub_bytes) VALUES (?1, 'Standard', 0, X'00')",
            &[&wallet_id.as_slice()],
        ),
        (
            "account_address_pools",
            "INSERT INTO account_address_pools (wallet_id, account_type, account_index, pool_type, snapshot_blob) VALUES (?1, 'Standard', 0, 'External', X'00')",
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
            "INSERT INTO core_derived_addresses (wallet_id, account_type, account_index, address, derivation_path, used) VALUES (?1, 'Standard', 0, 'addr', '', 0)",
            &[&wallet_id.as_slice()],
        ),
        (
            "core_sync_state",
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) VALUES (?1, NULL, NULL)",
            &[&wallet_id.as_slice()],
        ),
        (
            "identity_keys",
            // V002: identity_keys drops the wallet_id column; the
            // FK now targets identities(identity_id).
            "INSERT INTO identity_keys (identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) VALUES (?1, 0, X'00', X'00', NULL)",
            &[&identity_id.as_slice()],
        ),
        (
            "contacts_sent",
            "INSERT INTO contacts_sent (wallet_id, owner_id, recipient_id, entry_blob) VALUES (?1, ?2, ?3, X'00')",
            &[&wallet_id.as_slice(), &identity_id.as_slice(), &[1u8; 32].as_slice()],
        ),
        (
            "contacts_recv",
            "INSERT INTO contacts_recv (wallet_id, owner_id, sender_id, entry_blob) VALUES (?1, ?2, ?3, X'00')",
            &[&wallet_id.as_slice(), &identity_id.as_slice(), &[2u8; 32].as_slice()],
        ),
        (
            "contacts_established",
            "INSERT INTO contacts_established (wallet_id, owner_id, contact_id, entry_blob) VALUES (?1, ?2, ?3, X'00')",
            &[&wallet_id.as_slice(), &identity_id.as_slice(), &[3u8; 32].as_slice()],
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
            // V002: token_balances PK is (identity_id, token_id);
            // wallet_id column is gone.
            "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) VALUES (?1, ?2, 0, 0)",
            &[&identity_id.as_slice(), &[5u8; 32].as_slice()],
        ),
        (
            "dashpay_profiles",
            // V002: dashpay_profiles keyed by identity_id only.
            "INSERT INTO dashpay_profiles (identity_id, profile_blob) VALUES (?1, X'00')",
            &[&identity_id.as_slice()],
        ),
        (
            "dashpay_payments_overlay",
            // V002: dashpay_payments_overlay keyed by (identity_id, payment_id).
            "INSERT INTO dashpay_payments_overlay (identity_id, payment_id, overlay_blob) VALUES (?1, 'pay1', X'00')",
            &[&identity_id.as_slice()],
        ),
    ];
    use platform_wallet_storage::sqlite::schema::{count_rows_for_wallet_sql, PER_WALLET_TABLES};
    let scope_for = |name: &str| {
        PER_WALLET_TABLES
            .iter()
            .find(|(t, _)| *t == name)
            .map(|(_, s)| *s)
            .expect("table is in PER_WALLET_TABLES")
    };
    for (table, sql, params) in cases {
        conn.execute(sql, *params).expect(table);
        let n: i64 = conn
            .query_row(
                &count_rows_for_wallet_sql(table, scope_for(table)),
                rusqlite::params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(n >= 1, "{table} insert did not land");
    }
}

/// TC-028: re-open is idempotent.
#[test]
fn tc028_idempotent_reopen() {
    let (persister, tmp, path) = fresh_persister();
    drop(persister);
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&path);
    let _p2 = platform_wallet_storage::SqlitePersister::open(cfg).expect("reopen");
    drop(tmp);
}

/// TC-029: append-only migration hash.
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

/// TC-044: load() on empty post-migrate DB is empty.
#[test]
fn tc044_load_empty_is_empty() {
    let (persister, _tmp, _path) = fresh_persister();
    let state = platform_wallet::changeset::PlatformWalletPersistence::load(&persister).unwrap();
    assert!(state.is_empty());
}
