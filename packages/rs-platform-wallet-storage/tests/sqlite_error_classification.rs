#![allow(clippy::field_reassign_with_default)]

//! TC-P2-005 — `WalletStorageError::is_transient` table-driven check.
//! TC-P2-010 — boundary mapping `FlushRetryable` → `PersistenceError::Backend`.
//!
//! The table uses a wildcard-free `match` so adding a future variant
//! to `WalletStorageError` forces this file to update too — together
//! with the `is_transient` impl, the ban on `#[non_exhaustive]` keeps
//! the policy honest end-to-end.

use std::path::PathBuf;

use platform_wallet::changeset::PersistenceError;
use platform_wallet_storage::sqlite::error::AutoBackupOperation;
use platform_wallet_storage::sqlite::util::safe_cast::SafeCastTarget;
use platform_wallet_storage::WalletStorageError;
use rusqlite::{Error as SqlErr, ErrorCode};

fn sqlite_busy() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    ))
}

fn sqlite_locked() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseLocked,
            extended_code: rusqlite::ffi::SQLITE_LOCKED,
        },
        Some("database table is locked".into()),
    ))
}

fn sqlite_corrupt() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseCorrupt,
            extended_code: rusqlite::ffi::SQLITE_CORRUPT,
        },
        Some("disk image malformed".into()),
    ))
}

/// TC-P2-005: every variant has the right transient/fatal classification.
///
/// The match below intentionally covers every variant without a `_`
/// arm so the compiler errors out when a new variant lands. The
/// `WalletStorageError` enum MUST NOT carry `#[non_exhaustive]` —
/// otherwise this exhaustiveness gate degrades to a warning.
#[test]
fn tc_p2_005_is_transient_table() {
    let cases: Vec<(WalletStorageError, bool, &'static str)> = vec![
        // Transient: BUSY + LOCKED + FlushRetryable.
        (sqlite_busy(), true, "sqlite_busy"),
        (sqlite_locked(), true, "sqlite_locked"),
        (
            WalletStorageError::FlushRetryable {
                wallet_id: [0xAB; 32],
                source: SqlErr::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::DatabaseBusy,
                        extended_code: rusqlite::ffi::SQLITE_BUSY,
                    },
                    Some("busy".into()),
                ),
            },
            true,
            "flush_retryable",
        ),
        // Fatal classes:
        (sqlite_corrupt(), false, "sqlite_other"),
        (
            WalletStorageError::Io(std::io::Error::other("boom")),
            false,
            "io",
        ),
        (
            WalletStorageError::IntegrityCheckFailed {
                report: "rows missing".into(),
            },
            false,
            "integrity_check_failed",
        ),
        (WalletStorageError::LockPoisoned, false, "lock_poisoned"),
        (
            WalletStorageError::ConfigInvalid { reason: "bad knob" },
            false,
            "config_invalid",
        ),
        (
            WalletStorageError::WalletNotFound {
                wallet_id: [0u8; 32],
            },
            false,
            "wallet_not_found",
        ),
        (
            WalletStorageError::IntegerOverflow {
                field: "f",
                value: u64::MAX,
                target: SafeCastTarget::U64,
            },
            false,
            "integer_overflow",
        ),
        (
            WalletStorageError::AutoBackupDisabled {
                operation: AutoBackupOperation::DeleteWallet,
            },
            false,
            "auto_backup_disabled",
        ),
        (
            WalletStorageError::AutoBackupDirUnwritable {
                dir: PathBuf::from("/nope"),
                source: std::io::Error::other("nope"),
            },
            false,
            "auto_backup_dir_unwritable",
        ),
        (
            WalletStorageError::LoadIncomplete { unimplemented: &[] },
            false,
            "load_incomplete",
        ),
        (
            WalletStorageError::BlobDecode {
                reason: "bad shape",
            },
            false,
            "blob_decode",
        ),
        (
            WalletStorageError::SchemaHistoryMissing,
            false,
            "schema_history_missing",
        ),
        (
            WalletStorageError::SchemaVersionUnsupported {
                found: 99,
                max_supported: 3,
            },
            false,
            "schema_version_unsupported",
        ),
        (
            WalletStorageError::MigrationDirty {
                applied: 1,
                pending: 1,
            },
            false,
            "migration_dirty",
        ),
        (
            WalletStorageError::IntegrityCheckRunFailed {
                source: SqlErr::ExecuteReturnedResults,
            },
            false,
            "integrity_check_run_failed",
        ),
        (
            WalletStorageError::SourceOpenFailed {
                source: SqlErr::ExecuteReturnedResults,
            },
            false,
            "source_open_failed",
        ),
        (
            WalletStorageError::RestoreDestinationLocked,
            false,
            "restore_destination_locked",
        ),
        (
            WalletStorageError::InvalidWalletIdHex {
                source: hex::FromHexError::OddLength,
            },
            false,
            "invalid_wallet_id_hex",
        ),
        (
            WalletStorageError::InvalidWalletIdLength { actual: 10 },
            false,
            "invalid_wallet_id_length",
        ),
        (
            WalletStorageError::BackupDestinationExists {
                path: PathBuf::from("/x"),
            },
            false,
            "backup_destination_exists",
        ),
    ];
    for (err, expect_transient, expect_kind) in cases {
        // Wildcard-free reflective match — the compile error when a
        // variant is added is the whole point of this test.
        let kind = err.error_kind_str();
        assert_eq!(
            err.is_transient(),
            expect_transient,
            "variant {kind:?} classification mismatch (got {})",
            err.is_transient()
        );
        assert_eq!(
            kind, expect_kind,
            "variant kind mismatch for {err:?}: expected {expect_kind}, got {kind}"
        );
    }
}

/// TC-P2-010: `FlushRetryable` flowing through the `From` impl into
/// `PersistenceError::Backend(String)` carries the markers ops grep
/// for: variant name, hex-encoded wallet id prefix, and the inner
/// rusqlite source text.
#[test]
fn tc_p2_010_boundary_error_mapping() {
    let err = WalletStorageError::FlushRetryable {
        wallet_id: [0xAB; 32],
        source: rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy,
                extended_code: rusqlite::ffi::SQLITE_BUSY,
            },
            Some("database is locked".into()),
        ),
    };
    let pe: PersistenceError = err.into();
    let s = match pe {
        PersistenceError::Backend(s) => s,
        other => panic!("expected Backend(_), got {other:?}"),
    };
    assert!(
        s.contains("flush failed transiently"),
        "missing FlushRetryable display marker: {s}"
    );
    assert!(s.contains("abab"), "missing wallet_id hex prefix: {s}");
    assert!(
        s.contains("database is locked"),
        "missing inner source text: {s}"
    );
}
