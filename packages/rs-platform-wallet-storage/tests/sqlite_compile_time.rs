#![allow(clippy::field_reassign_with_default)]

//! TC-076, TC-077, TC-078 — compile-time assertions.
//! TC-P1-003 — every writer call site uses `prepare_cached`.
//! TC-P4-011 — `ClientStartState` carries `#[non_exhaustive]`.

use std::sync::Arc;

use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use static_assertions::assert_impl_all;

assert_impl_all!(SqlitePersister: Send, Sync, PlatformWalletPersistence);

/// TC-078: SqlitePersister fits behind Arc<dyn PlatformWalletPersistence>.
#[test]
fn tc078_object_safety() {
    fn accepts(_: Arc<dyn PlatformWalletPersistence>) {}
    let tmp = tempfile::tempdir().unwrap();
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
        "wallet_meta.rs",
        "SELECT wallet_id FROM wallet_metadata ORDER BY wallet_id",
    ),
    (
        "wallet_meta.rs",
        "SELECT network, birth_height FROM wallet_metadata WHERE wallet_id",
    ),
    ("asset_locks.rs", "SELECT outpoint, account_index"),
    ("platform_addrs.rs", "SELECT account_index, address_index"),
    ("core_state.rs", "SELECT outpoint, value, script, height"),
    // P4 readers — `load_state` per area uses one-shot SELECTs.
    (
        "identities.rs",
        "SELECT identity_id, entry_blob, tombstoned",
    ),
    (
        "contacts.rs",
        "SELECT owner_id, recipient_id, entry_blob FROM contacts_sent",
    ),
    (
        "contacts.rs",
        "SELECT owner_id, sender_id, entry_blob FROM contacts_recv",
    ),
    (
        "contacts.rs",
        "SELECT owner_id, contact_id, entry_blob FROM contacts_established",
    ),
    // Bulk `load_all` readers — single scan per table for `load()`,
    // grouped in memory by wallet_id (FR-P4-6). Read-only by design.
    (
        "platform_addrs.rs",
        "SELECT wallet_id, sync_height, sync_timestamp, last_known_recent_block",
    ),
    (
        "platform_addrs.rs",
        "SELECT wallet_id FROM platform_addresses",
    ),
    (
        "identities.rs",
        "SELECT wallet_id, identity_id, entry_blob, tombstoned",
    ),
    (
        "contacts.rs",
        "SELECT wallet_id, owner_id, recipient_id, entry_blob",
    ),
    (
        "contacts.rs",
        "SELECT wallet_id, owner_id, sender_id, entry_blob",
    ),
    (
        "contacts.rs",
        "SELECT wallet_id, owner_id, contact_id, entry_blob",
    ),
    (
        "asset_locks.rs",
        "SELECT wallet_id, outpoint, account_index, lifecycle_blob",
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

/// TC-P4-011: `ClientStartState` is `#[non_exhaustive]` so future
/// slots can be added without a breaking-change wave for callers that
/// destructure exhaustively.
#[test]
fn tc_p4_011_client_start_state_non_exhaustive() {
    // Source-level grep — the attribute is per-decl, not exposed via
    // reflection. Locate the upstream file relative to this crate.
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
                prev_non_exhaustive,
                "`pub struct ClientStartState` must be preceded by `#[non_exhaustive]`"
            );
            break;
        }
        // Reset only if we see another item attribute or a non-trivial
        // declaration line — derive-only lines preserve the marker.
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
}
