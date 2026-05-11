#![allow(clippy::field_reassign_with_default)]

//! TC-005, TC-013, TC-079, TC-080, TC-081 — config + scalar round-trips.
//!
//! The bulk of the per-sub-changeset round-trip tests in Marvin's spec
//! (TC-001..TC-014) require constructing upstream changeset values
//! whose payload types do not derive `serde` or `bincode`. The schema
//! captures every typed scalar column those tests verify; the blob
//! columns store a custom self-describing layout (see
//! `src/schema/blob.rs`) that round-trips the wallet-id key tuple but
//! not the upstream payloads.
//!
//! TC-001 is exercised in `buffer_semantics.rs::tc001_get_core_tx_record_roundtrip`.
//! TC-015 is exercised in `buffer_semantics.rs::tc015_two_wallets_in_one_db`.
//! TC-005 / TC-013 are below.
//!
//! TC-002, TC-006..TC-012, TC-014 are tracked as follow-up work once
//! upstream gains `serde`/`bincode` derives on the changeset payload
//! types; the persistence machinery is in place to receive them.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::Network;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{
    SqlitePersister, SqlitePersisterConfig, SqlitePersisterError, Synchronous,
};

/// TC-005: sync heights round-trip with monotonic-max merge.
#[test]
fn tc005_sync_heights_roundtrip() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF0);
    ensure_wallet_meta(&persister, &w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        last_processed_height: Some(100),
        synced_height: Some(95),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        last_processed_height: Some(120),
        synced_height: Some(100),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    let conn = persister.lock_conn_for_test();
    let (lp, sy): (i64, i64) = conn
        .query_row(
            "SELECT last_processed_height, synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(lp, 120);
    assert_eq!(sy, 100);
}

/// TC-013: wallet_metadata round-trip.
#[test]
fn tc013_wallet_metadata_roundtrip() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&persister, &w);
    let cs = PlatformWalletChangeSet {
        wallet_metadata: Some(WalletMetadataEntry {
            network: Network::Testnet,
            birth_height: 12345,
        }),
        ..Default::default()
    };
    persister.store(w, cs).unwrap();
    let conn = persister.lock_conn_for_test();
    let (network, birth_height): (String, i64) = conn
        .query_row(
            "SELECT network, birth_height FROM wallet_metadata WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(network, "testnet");
    assert_eq!(birth_height, 12345);
}

/// TC-079: synchronous=Off is rejected at open with a typed error.
#[test]
fn tc079_synchronous_off_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let mut cfg = SqlitePersisterConfig::new(&path);
    cfg.synchronous = Synchronous::Off;
    let err = SqlitePersister::open(cfg);
    let matched = matches!(err.as_ref(), Err(SqlitePersisterError::ConfigInvalid(_)));
    assert!(
        matched,
        "expected ConfigInvalid, got error = {:?}",
        err.as_ref().err()
    );
    assert!(
        !path.exists(),
        "DB should not be created when config is invalid"
    );
}

/// TC-080: SqlitePersisterConfig::new yields sensible defaults.
#[test]
fn tc080_config_defaults() {
    let cfg = SqlitePersisterConfig::new("/tmp/some.db");
    assert!(matches!(
        cfg.flush_mode,
        platform_wallet_storage::FlushMode::Immediate
    ));
    assert_eq!(cfg.busy_timeout, std::time::Duration::from_secs(5));
    assert!(matches!(
        cfg.journal_mode,
        platform_wallet_storage::JournalMode::Wal
    ));
    assert!(matches!(cfg.synchronous, Synchronous::Normal));
    assert!(cfg.auto_backup_dir.is_some());
}

/// TC-081: LockPoisoned round-trips into PersistenceError::LockPoisoned.
#[test]
fn tc081_lock_poisoned_mapping() {
    use platform_wallet::changeset::PersistenceError;
    let err = SqlitePersisterError::LockPoisoned;
    let mapped: PersistenceError = err.into();
    assert!(matches!(mapped, PersistenceError::LockPoisoned));
}

/// TC-082 (lint): grep for `Box<dyn Error>` in the crate's sources.
#[test]
fn tc082_no_box_dyn_error_in_src() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    visit(&root, &mut offenders);
    assert!(
        offenders.is_empty(),
        "Box<dyn Error> found in: {offenders:?}"
    );

    fn visit(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                visit(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let s = std::fs::read_to_string(&p).unwrap();
                if s.contains("Box<dyn Error") || s.contains("Box<dyn std::error::Error") {
                    out.push(p.display().to_string());
                }
            }
        }
    }
}
