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
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .unwrap();
    let identity_id = [7u8; 32];
    conn.execute(
        "INSERT INTO identities (wallet_id, identity_index, identity_id, entry_blob, tombstoned) \
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
            "INSERT INTO account_registrations (wallet_id, account_type, account_index, account_xpub_bytes) VALUES (?1, 'standard_bip44', 0, X'00')",
            &[&wallet_id.as_slice()],
        ),
        (
            "core_transactions",
            "INSERT INTO core_transactions (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) VALUES (?1, ?2, NULL, NULL, NULL, 0, X'00')",
            &[&wallet_id.as_slice(), &txid],
        ),
        (
            "core_utxos",
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) VALUES (?1, ?2, 0, X'00', 0)",
            &[&wallet_id.as_slice(), &outpoint],
        ),
        (
            "core_instant_locks",
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, X'00')",
            &[&wallet_id.as_slice(), &txid],
        ),
        (
            "core_sync_state",
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) VALUES (?1, NULL, NULL)",
            &[&wallet_id.as_slice()],
        ),
        (
            "identity_keys",
            // identity_keys is keyed by (wallet_id, identity_id, key_id);
            // the wallet_id FK targets wallets and the
            // identity_id FK targets identities(identity_id).
            "INSERT INTO identity_keys (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) VALUES (?1, ?2, 0, X'00', X'00', NULL)",
            &[&wallet_id.as_slice(), &identity_id.as_slice()],
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

    // `identity_keys` is counted above via the identity join, but it also
    // carries its OWN `wallet_id` column (the direct per-wallet read scope);
    // verify the smoke row is countable that way too.
    let direct: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identity_keys WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        direct >= 1,
        "identity_keys must be countable by its direct wallet_id column"
    );
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
/// Asserts intra-run stability and a non-empty list — not content
/// pinning. The fingerprint is content-blind (hashes `(version, name)`
/// only), so this guards the migration set's identity, not its DDL.
#[test]
fn tc029_migration_fingerprint_stable() {
    let a = mig::embedded_migrations_fingerprint();
    let b = mig::embedded_migrations_fingerprint();
    assert_eq!(a, b);
    assert!(!mig::embedded_migrations().is_empty());
}

/// `core_utxos` stores only fields used by production persistence.
#[test]
fn tc030_core_utxos_dead_metadata_columns_removed() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn.prepare("PRAGMA table_info(core_utxos)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!columns.iter().any(|column| column == "account_index"));
    assert!(!columns.iter().any(|column| column == "spent_in_txid"));
}

/// Confirmation height is single-sourced in nullable `core_transactions` rows.
#[test]
fn tc031_confirmation_height_is_single_sourced_in_core_transactions() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let mut utxo_stmt = conn.prepare("PRAGMA table_info(core_utxos)").unwrap();
    let utxo_columns: Vec<String> = utxo_stmt
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!utxo_columns.iter().any(|column| column == "height"));

    let mut transaction_stmt = conn
        .prepare("PRAGMA table_info(core_transactions)")
        .unwrap();
    let transaction_columns: Vec<(String, bool)> = transaction_stmt
        .query_map([], |row| Ok((row.get(1)?, row.get::<_, i64>(3)? == 0)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(transaction_columns
        .iter()
        .any(|(column, _nullable)| column == "height"));
    assert!(transaction_columns
        .iter()
        .any(|(column, nullable)| column == "record_blob" && *nullable));
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

    // 2. Populate it the way a live wallet would have. The table is still
    //    `wallet_metadata` here — V007 is what renames it to `wallets`.
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

    // 3b. Plant a legacy orphan row the way an old connection with FK
    //     enforcement off could have: its wallet row is gone, so copying
    //     it into the FK-declared twin would abort the rebuild. V004's
    //     explicit orphan policy must drop it instead.
    conn.pragma_update(None, "foreign_keys", false)
        .expect("disable foreign keys");
    let ghost_wallet = [9u8; 32];
    conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, \
         amount_duffs, lifecycle_blob) VALUES (?1, X'04', 'built', 0, 0, 1, X'04')",
        params![ghost_wallet.as_slice()],
    )
    .expect("insert orphan row with FK enforcement off");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("re-enable foreign keys");

    // 4. Upgrade to the latest schema (applies V004's table rebuild).
    mig::run(&mut conn).expect("migrate to latest despite the orphan row");

    // 4b. The orphan is gone (same outcome the declared cascade would
    //     have produced), the real row below is untouched.
    let orphans: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM asset_locks WHERE wallet_id = ?1",
            params![ghost_wallet.as_slice()],
            |row| row.get(0),
        )
        .expect("count orphans");
    assert_eq!(orphans, 0, "V004 must drop legacy orphan rows, not abort");

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
        "DELETE FROM wallets WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )
    .expect("delete wallet");
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM asset_locks", [], |row| row.get(0))
        .expect("count");
    assert_eq!(remaining, 0, "ON DELETE CASCADE must survive the rebuild");
}

/// V013 → V014 upgrade path: a database created at the prior release
/// schema (through V013) carrying a legacy empty-script spent row becomes
/// loadable again.
///
/// The poisoned row is what the producer wrote before it reconstructed a
/// spent output's script from its typed address: `spent = 1, script = X''`.
/// `load_used_addresses` decodes every stored script with no load-policy
/// escape hatch, so that one row rejects the whole file — this test drives
/// exactly the sequence an existing install experiences, asserting the read
/// fails before the purge and recovers after it.
#[test]
fn tc046_v014_purges_legacy_empty_script_spent_utxos() {
    use platform_wallet_storage::sqlite::schema::core_state;
    use platform_wallet_storage::WalletStorageError;
    use rusqlite::params;

    let mut conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    // 1. Stand the database up at the PRIOR release schema (V013).
    let to_v013 = mig::runner().set_target(refinery::Target::Version(13));
    to_v013.run(&mut conn).expect("migrate to V013");

    // 2. Two wallets: the poisoned one, and one holding the unspent
    //    empty-script edge case the predicate must NOT reach.
    let poisoned = [42u8; 32];
    let untouched = [43u8; 32];
    for wallet_id in [poisoned, untouched] {
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet_id.as_slice()],
        )
        .expect("insert wallet");
    }

    // P2PKH: OP_DUP OP_HASH160 <20-byte hash> OP_EQUALVERIFY OP_CHECKSIG.
    let mut real_script = vec![0x76, 0xa9, 0x14];
    real_script.extend_from_slice(&[0x11u8; 20]);
    real_script.extend_from_slice(&[0x88, 0xac]);

    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 1000, X'', 1)",
        params![poisoned.as_slice(), [1u8; 36].as_slice()],
    )
    .expect("insert poisoned row");
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 2000, ?3, 1)",
        params![
            poisoned.as_slice(),
            [2u8; 36].as_slice(),
            real_script.as_slice()
        ],
    )
    .expect("insert legitimate spent row");
    // Same degenerate script, but `spent = 0`: balance state, out of the
    // predicate's reach whatever the script holds.
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 3000, X'', 0)",
        params![untouched.as_slice(), [3u8; 36].as_slice()],
    )
    .expect("insert unspent empty-script row");

    // 3. The damage V012 exists to repair: at V011 the single poisoned row
    //    rejects the used-set read for the whole wallet.
    let err = core_state::load_used_addresses_with_ctx(
        &conn,
        &poisoned,
        dashcore::Network::Testnet,
        &platform_wallet_storage::LoadCtx::strict(),
    )
    .expect_err("the poisoned row must reject the used-set read before V012");
    assert!(
        matches!(err, WalletStorageError::AddressDecode { .. }),
        "expected AddressDecode from the empty script, got {err:?}"
    );

    // 4. Upgrade to the latest schema (applies V012's purge).
    mig::run(&mut conn).expect("migrate to latest");

    // 5. The poisoned row is gone...
    let poisoned_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE spent = 1 AND length(script) = 0",
            [],
            |row| row.get(0),
        )
        .expect("count poisoned rows");
    assert_eq!(
        poisoned_rows, 0,
        "V012 must purge legacy empty-script spent rows"
    );

    // 6. ...the legitimate spent row beside it survived byte-identical...
    let (value, script): (i64, Vec<u8>) = conn
        .query_row(
            "SELECT value, script FROM core_utxos WHERE wallet_id = ?1",
            params![poisoned.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the legitimate spent row survives");
    assert_eq!((value, script.as_slice()), (2000, real_script.as_slice()));

    // 7. ...so the read the poisoned row was rejecting now succeeds, and
    //    the real address still guards against reuse.
    let used = core_state::load_used_addresses_with_ctx(
        &conn,
        &poisoned,
        dashcore::Network::Testnet,
        &platform_wallet_storage::LoadCtx::strict(),
    )
    .expect("the used-set read recovers after the purge");
    let used_scripts: Vec<Vec<u8>> = used
        .iter()
        .map(|(addr, _owner)| addr.script_pubkey().to_bytes())
        .collect();
    assert_eq!(
        used_scripts,
        vec![real_script],
        "the real address must stay in the reuse guard"
    );

    // 8. The unspent empty-script row is untouched: the predicate is scoped
    //    to `spent = 1`, not "any empty script".
    let unspent_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1 AND spent = 0 \
             AND length(script) = 0",
            params![untouched.as_slice()],
            |row| row.get(0),
        )
        .expect("count unspent rows");
    assert_eq!(
        unspent_rows, 1,
        "an unspent row is balance state and must survive any script content"
    );
}

/// V013 rebuilds `core_transactions` into an FK-declaring twin and backfills
/// height-only rows from `core_utxos`. Both sources can hold rows whose wallet
/// was deleted while FK enforcement happened to be off — third-party SQLite
/// tooling defaults `foreign_keys` OFF, and this database sits on an end
/// user's own device. Copying such a row under `PRAGMA foreign_keys = ON`
/// aborts the migration, and since `open` migrates on every open the database
/// then never opens again.
///
/// Drives exactly that: one orphan in each source table, plus live rows that
/// must survive untouched.
#[test]
fn tc047_v013_drops_orphans_instead_of_aborting_the_rebuild() {
    use rusqlite::params;

    let mut conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    // 1. Stand the database up at the PRIOR release schema (V012).
    let to_v012 = mig::runner().set_target(refinery::Target::Version(12));
    to_v012.run(&mut conn).expect("migrate to V012");

    // 2. A live wallet with one real transaction and one real UTXO.
    let wallet_id = [42u8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .expect("insert wallet");
    let live_txid = [7u8; 32];
    conn.execute(
        "INSERT INTO core_transactions (wallet_id, txid, height, block_hash, block_time, \
         finalized, record_blob) VALUES (?1, ?2, 100, NULL, NULL, 1, X'AA')",
        params![wallet_id.as_slice(), live_txid.as_slice()],
    )
    .expect("insert live transaction");
    let live_outpoint = [0x20u8; 37];
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, height, spent) \
         VALUES (?1, ?2, 5000, X'BB', 100, 0)",
        params![wallet_id.as_slice(), live_outpoint.as_slice()],
    )
    .expect("insert live utxo");

    // 3. Plant one orphan in each source table, the way an old connection
    //    with FK enforcement off could have left them.
    conn.pragma_update(None, "foreign_keys", false)
        .expect("disable foreign keys");
    let ghost_wallet = [9u8; 32];
    conn.execute(
        "INSERT INTO core_transactions (wallet_id, txid, height, block_hash, block_time, \
         finalized, record_blob) VALUES (?1, X'01', 5, NULL, NULL, 0, X'CC')",
        params![ghost_wallet.as_slice()],
    )
    .expect("insert orphan transaction with FK enforcement off");
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, height, spent) \
         VALUES (?1, X'02', 1, X'DD', 9, 0)",
        params![ghost_wallet.as_slice()],
    )
    .expect("insert orphan utxo with FK enforcement off");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("re-enable foreign keys");

    // 4. The rebuild must complete rather than abort on the orphans.
    mig::run(&mut conn).expect("migrate to latest despite the orphan rows");

    // 5. Both orphans are gone — the same outcome the declared cascade would
    //    have produced had enforcement been on when the wallet was deleted.
    let ghost_rows: i64 = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1) \
                  + (SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1)",
            params![ghost_wallet.as_slice()],
            |row| row.get(0),
        )
        .expect("count orphan rows");
    assert_eq!(ghost_rows, 0, "V013 must drop orphans, not abort");

    // 6. The live transaction survived the rebuild with its record intact.
    let (height, finalized, blob): (i64, i64, Vec<u8>) = conn
        .query_row(
            "SELECT height, finalized, record_blob FROM core_transactions WHERE txid = ?1",
            params![live_txid.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("live transaction survives");
    assert_eq!((height, finalized, blob), (100, 1, vec![0xAA]));

    // 7. The live UTXO survived, and its confirmation height was backfilled
    //    onto a height-only `core_transactions` row.
    let live_utxos: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("count live utxos");
    assert_eq!(live_utxos, 1, "the live UTXO must survive the orphan sweep");
}

/// V007 rebuilds `identity_keys` with its own `wallet_id` scope, backfilled by
/// joining `identities`. A key whose identity is gone cannot be carried across
/// — the re-declared FK would abort the migration — so it is swept, and a key
/// belonging to a live identity must land under that identity's wallet.
#[test]
fn tc048_v007_backfills_identity_key_wallet_scope() {
    use rusqlite::params;

    let mut conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    // The published v4.2-dev schema, before the reshape.
    let to_v006 = mig::runner().set_target(refinery::Target::Version(6));
    to_v006.run(&mut conn).expect("migrate to V006");

    let wallet_id = [0x51u8; 32];
    let owned_identity = [0x61u8; 32];
    let ghost_identity = [0x62u8; 32];
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .expect("insert wallet");
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, 0, X'00', 0)",
        params![owned_identity.as_slice(), wallet_id.as_slice()],
    )
    .expect("insert identity");
    conn.execute(
        "INSERT INTO identity_keys (identity_id, key_id, public_key_blob, public_key_hash) \
         VALUES (?1, 0, X'00', X'00')",
        params![owned_identity.as_slice()],
    )
    .expect("insert key for the live identity");

    // A key whose identity never existed, plantable only with enforcement off.
    conn.pragma_update(None, "foreign_keys", false)
        .expect("disable foreign keys");
    conn.execute(
        "INSERT INTO identity_keys (identity_id, key_id, public_key_blob, public_key_hash) \
         VALUES (?1, 0, X'00', X'00')",
        params![ghost_identity.as_slice()],
    )
    .expect("insert orphan key with FK enforcement off");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("re-enable foreign keys");

    mig::run(&mut conn).expect("migrate to latest despite the orphan key");

    let scope: Vec<u8> = conn
        .query_row(
            "SELECT wallet_id FROM identity_keys WHERE identity_id = ?1",
            params![owned_identity.as_slice()],
            |row| row.get(0),
        )
        .expect("live key survives the rebuild");
    assert_eq!(
        scope,
        wallet_id.to_vec(),
        "the rebuilt key must be scoped to the wallet that owns its identity"
    );
    let ghosts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
            params![ghost_identity.as_slice()],
            |row| row.get(0),
        )
        .expect("count orphan keys");
    assert_eq!(
        ghosts, 0,
        "a key naming no identity must be swept, not copied"
    );
}

/// A pre-split `standard` row whose blob says BIP32 must still load cleanly
/// under the default strict policy.
///
/// `v4.2-dev` wrote `standard` for both standard variants, so which one a row
/// is lives only in `account_xpub_bytes`. V007 therefore admits the legacy
/// label instead of rewriting it: a rewrite would have to guess, and guessing
/// BIP44 for this row would make the reader's cross-check fail and take the
/// whole wallet down under `LoadPolicy::Strict` -- a row that loads today
/// turned into a hard failure by the migration meant to carry it forward.
#[test]
fn tc049_legacy_standard_row_with_bip32_blob_still_loads() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::bip32::ExtendedPubKey;
    use platform_wallet::changeset::{AccountRegistrationEntry, PlatformWalletPersistence};
    use platform_wallet_storage::sqlite::schema::blob;
    use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
    use rusqlite::params;

    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("legacy-standard.db");
    let wallet_id = [0x53u8; 32];

    let xpub = ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap();
    // The blob says BIP32; the column will say the pre-split `standard`.
    let entry = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP32Account,
        },
        account_xpub: xpub,
    };
    let entry_blob = blob::encode(&entry).expect("encode registration");

    {
        let mut conn = rusqlite::Connection::open(&path).expect("open db");
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        mig::runner()
            .set_target(refinery::Target::Version(6))
            .run(&mut conn)
            .expect("migrate to the v4.2-dev schema");
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            params![wallet_id.as_slice()],
        )
        .expect("insert wallet");
        conn.execute(
            "INSERT INTO account_registrations \
                 (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'standard', 0, ?2)",
            params![wallet_id.as_slice(), entry_blob],
        )
        .expect("insert pre-split standard registration");
    }

    // Default config is LoadPolicy::Strict: a cross-check mismatch is fatal.
    let persister =
        SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("v4.2-dev store opens");
    let state = persister
        .load()
        .expect("strict load must not reject the legacy row");

    let label: String = {
        let conn = persister.lock_conn_for_test();
        conn.query_row(
            "SELECT account_type FROM account_registrations WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("registration survives the rebuild")
    };
    assert_eq!(
        label, "standard",
        "the pre-split label is admitted, never rewritten to a guess"
    );
    assert!(
        state.wallets.contains_key(&wallet_id),
        "the wallet carrying the legacy row must reconstruct"
    );
}

/// A surviving legacy `standard` row is NOT rewritten by a later save of the
/// same account: the upsert's conflict target includes `account_type`, so the
/// writer's precise label is a different primary key and inserts a sibling row.
///
/// Pinned because "the next write heals it" is the obvious wrong assumption to
/// make here, and because the reconciliation that makes the surviving row
/// harmless lives in the READER: `accounts::load_state` collapses the pair, so
/// the manifest carries the account once even though the table carries it
/// twice. A destructive `DELETE` in a wallet's account table to tidy a label
/// would be the wrong trade. Both halves are asserted below, because the
/// reader's guarantee is only worth anything while the writer's fork is real.
#[test]
fn tc050_legacy_standard_row_is_reconciled_by_the_reader_not_the_writer() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::bip32::ExtendedPubKey;
    use platform_wallet::changeset::{
        AccountRegistrationEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use platform_wallet_storage::sqlite::schema::blob;
    use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
    use rusqlite::params;

    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("legacy-sibling.db");
    let wallet_id = [0x54u8; 32];
    let xpub = ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap();
    let entry = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        account_xpub: xpub,
    };
    let entry_blob = blob::encode(&entry).unwrap();

    {
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        mig::runner()
            .set_target(refinery::Target::Version(6))
            .run(&mut conn)
            .expect("migrate to the v4.2-dev schema");
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            params![wallet_id.as_slice()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_registrations \
                 (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'standard', 0, ?2)",
            params![wallet_id.as_slice(), entry_blob],
        )
        .unwrap();
    }

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let mut cs = PlatformWalletChangeSet::default();
    cs.account_registrations = vec![entry];
    persister.store(wallet_id, cs).unwrap();
    persister.flush(wallet_id).unwrap();

    let labels: Vec<String> = {
        let conn = persister.lock_conn_for_test();
        let mut stmt = conn
            .prepare(
                "SELECT account_type FROM account_registrations \
                 WHERE wallet_id = ?1 ORDER BY account_type",
            )
            .unwrap();
        let rows = stmt
            .query_map(params![wallet_id.as_slice()], |r| r.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        rows
    };
    assert_eq!(
        labels,
        vec!["standard".to_string(), "standard_bip44".to_string()],
        "the legacy row survives beside the writer's precise label"
    );
    let manifest = {
        let conn = persister.lock_conn_for_test();
        platform_wallet_storage::sqlite::schema::accounts::load_state(
            &conn,
            &wallet_id,
            &platform_wallet_storage::LoadCtx::strict(),
        )
        .expect("the forked pair must not break a strict load")
    };
    assert_eq!(
        manifest.ecdsa.len(),
        1,
        "two rows, one account: the reader must return it once, got {:?}",
        manifest.ecdsa
    );
}
