#![allow(clippy::field_reassign_with_default)]

//! TC-076, TC-077, TC-078 — compile-time assertions.
//! TC-P1-003 — every writer call site uses `prepare_cached`.
//! TC-P4-011 — `ClientStartState` keeps the base public shape.

mod common;

use std::sync::Arc;

use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use static_assertions::assert_impl_all;

assert_impl_all!(SqlitePersister: Send, Sync, PlatformWalletPersistence);

/// TC-078: SqlitePersister fits behind Arc<dyn PlatformWalletPersistence>.
#[test]
fn tc078_object_safety() {
    fn accepts(_: Arc<dyn PlatformWalletPersistence>) {}
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path);
    let p = SqlitePersister::open(cfg).unwrap();
    let arc: Arc<dyn PlatformWalletPersistence> = Arc::new(p);
    accepts(arc);
}

/// Read-only SELECT call sites where `prepare(` is allowed (per FR-P1-1).
/// Every other writer in `schema/` MUST use `prepare_cached`. Match key
/// is the line content (substring) — line numbers shift, contents
/// rarely do.
const READ_ONLY_PREPARE_ALLOWED: &[(&str, &str)] = &[
    (
        "wallets.rs",
        "SELECT wallet_id FROM wallets ORDER BY wallet_id",
    ),
    (
        "wallets.rs",
        "SELECT network, birth_height FROM wallets WHERE wallet_id",
    ),
    // asset_locks readers (load_active / load_unconsumed / list_active) —
    // pre-read length() gates on outpoint and lifecycle_blob.
    (
        "asset_locks.rs",
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status",
    ),
    ("platform_addrs.rs", "SELECT account_index, address_index"),
    // Grouped bulk readers driving `load()` — fixed scans over the whole
    // table, not per-wallet fan-out.
    (
        "platform_addrs.rs",
        "SELECT wallet_id, sync_height, sync_timestamp, last_known_recent_block",
    ),
    (
        "platform_addrs.rs",
        "SELECT wallet_id, account_index, address_index, length(address), address, balance, nonce",
    ),
    // Pre-read `length()` gates added by PR #3968 review — substrings updated
    // to reflect the new `length(<col>)` column in each SELECT.
    (
        "accounts.rs",
        "SELECT account_index, key_class, length(account_xpub_bytes), account_xpub_bytes",
    ),
    (
        "accounts.rs",
        "SELECT length(wallet_id), wallet_id, account_index, key_class,",
    ),
    // Provider platform-node-key reader: pre-read length() gates on both
    // fixed-width key columns.
    (
        "accounts.rs",
        "SELECT key_index, length(public_key), public_key, length(node_id), node_id",
    ),
    // list_unspent_utxos (test-helper reader, ungated — global SQLITE_LIMIT_LENGTH covers it).
    ("core_state.rs", "SELECT outpoint, value, script, account_index"),
    // load_state unspent-UTXO reader: pre-read length() gates on outpoint and script.
    (
        "core_state.rs",
        "SELECT length(outpoint), outpoint, value, length(script), script",
    ),
    ("core_state.rs", "SELECT DISTINCT script FROM core_utxos"),
    // Pool reader: verbatim used-set with owner columns, a one-shot read-only
    // scan per wallet.
    (
        "core_pool.rs",
        "SELECT script, account_type, account_index,",
    ),
    // Typed-pool reader: one-shot read-only scan of pre-derived platform-node
    // keys per wallet, during `load()` rehydration.
    (
        "core_pool.rs",
        "SELECT address_index, length(script), script, length(public_key), public_key",
    ),
    // Full-rehydration readers — one-shot SELECTs in `load_state`.
    (
        "accounts.rs",
        "SELECT account_type, account_index, key_class,",
    ),
    (
        "core_state.rs",
        "SELECT length(record_blob), record_blob FROM core_transactions",
    ),
    (
        "core_state.rs",
        "SELECT length(txid), txid, length(islock_blob), islock_blob",
    ),
    (
        "core_state.rs",
        "SELECT last_processed_height, synced_height,",
    ),
    (
        "identity_keys.rs",
        "SELECT identity_id, key_id, length(public_key_blob), public_key_blob",
    ),
    // P4 readers — `load_state` per area uses one-shot SELECTs.
    // Substring covers both `fetch` (`SELECT length(entry_blob)…`) and
    // `load_state` (`SELECT identity_id, length(entry_blob)…`).
    (
        "identities.rs",
        "length(entry_blob), entry_blob, tombstoned",
    ),
    // load_tombstoned_ids supplies the merge's positive tombstone signal.
    (
        "identities.rs",
        "SELECT identity_id FROM identities WHERE",
    ),
    ("contacts.rs", "SELECT owner_id, contact_id, state"),
    (
        "contacts.rs",
        "SELECT owner_id, sender_id FROM ignored_senders",
    ),
    (
        "pending_contact_crypto.rs",
        "SELECT wallet_id, payload FROM pending_contact_crypto",
    ),
    (
        "invitations.rs",
        "SELECT outpoint, status, funding_index, amount_duffs",
    ),
];

/// TC-P1-003: writer paths in `src/sqlite/schema/*.rs` must not call
/// `prepare(`. Read-only SELECTs explicitly listed in
/// `READ_ONLY_PREPARE_ALLOWED` (per FR-P1-1) are exempt; every other
/// call site must use `prepare_cached`.
#[test]
fn tc_p1_003_prepare_cached_in_writers() {
    let schema_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sqlite")
        .join("schema");
    let mut offenders: Vec<(String, usize, String)> = Vec::new();
    for entry in std::fs::read_dir(&schema_dir).expect("read schema dir") {
        let entry = entry.expect("schema dir entry");
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".rs") {
            continue;
        }
        if file_name == "mod.rs" || file_name == "blob.rs" {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("read schema file");
        let lines: Vec<&str> = body.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if !line.contains(".prepare(") {
                continue;
            }
            // SQL may be on this line or the following two — concat
            // and probe each allow-list substring.
            let probe: String = lines
                .iter()
                .skip(idx)
                .take(3)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            let allowed = READ_ONLY_PREPARE_ALLOWED
                .iter()
                .any(|(f, sql)| *f == file_name && probe.contains(sql));
            if allowed {
                continue;
            }
            offenders.push((file_name.to_string(), idx + 1, (*line).to_string()));
        }
    }
    assert!(
        offenders.is_empty(),
        "writer paths must use `prepare_cached`; offenders: {:#?}",
        offenders
    );
}

/// TC-P4-011: `ClientStartState` keeps the base public shape — plain
/// (NOT `#[non_exhaustive]`) and carrying exactly the two wired-up
/// slots `platform_addresses` + `wallets`. The persister populates
/// only `platform_addresses`; any reintroduction of `#[non_exhaustive]`
/// or extra slots is a breaking-API regression for downstream callers
/// that destructure the struct exhaustively.
#[test]
fn tc_p4_011_client_start_state_base_shape() {
    let upstream = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("packages/")
        .join("rs-platform-wallet/src/changeset/client_start_state.rs");
    let body = std::fs::read_to_string(&upstream).expect("read client_start_state.rs");

    let mut prev_non_exhaustive = false;
    let mut found = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[non_exhaustive]") {
            prev_non_exhaustive = true;
            continue;
        }
        if trimmed.starts_with("pub struct ClientStartState") {
            found = true;
            assert!(
                !prev_non_exhaustive,
                "`ClientStartState` must NOT be `#[non_exhaustive]` — the \
                 base public shape was restored (PR #3643 thread #7)"
            );
            break;
        }
        if !trimmed.is_empty()
            && !trimmed.starts_with("///")
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("#[derive")
        {
            prev_non_exhaustive = false;
        }
    }
    assert!(
        found,
        "did not encounter `pub struct ClientStartState` declaration"
    );

    for field in ["platform_addresses:", "wallets:"] {
        assert!(
            body.contains(field),
            "base `ClientStartState` must keep the `{field}` slot"
        );
    }
    for removed in ["identities:", "contacts:", "asset_locks:"] {
        assert!(
            !body.contains(removed),
            "`ClientStartState` must not carry the reverted `{removed}` slot"
        );
    }
}
