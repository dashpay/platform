#![allow(clippy::field_reassign_with_default)]

//! `WalletStorageError::is_transient` + `error_kind_str` exhaustiveness,
//! plus the boundary mapping of `FlushRetryable` into
//! `PersistenceError::Backend`.
//!
//! The check is a wildcard-free `match` with one arm per variant (no
//! `_`), so a new `WalletStorageError` variant fails to compile here
//! until it is classified — mirroring the matches in `error::is_transient`
//! / `error::error_kind_str`.

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

fn sqlite_disk_full() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DiskFull,
            extended_code: rusqlite::ffi::SQLITE_FULL,
        },
        Some("database or disk is full".into()),
    ))
}

fn sqlite_io_failure() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::SystemIoFailure,
            extended_code: rusqlite::ffi::SQLITE_IOERR,
        },
        Some("disk I/O error".into()),
    ))
}

fn sqlite_oom() -> WalletStorageError {
    WalletStorageError::Sqlite(SqlErr::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::OutOfMemory,
            extended_code: rusqlite::ffi::SQLITE_NOMEM,
        },
        Some("out of memory".into()),
    ))
}

/// One representative sample per `WalletStorageError` variant.
///
/// The samples are passed through a wildcard-free `match` below; the
/// compiler enforces every variant is named. `Sqlite(_)` and the
/// `FlushRetryable` retry path are split into their classified
/// sub-cases (busy / locked / non-retryable) inside the body.
fn samples() -> Vec<WalletStorageError> {
    vec![
        WalletStorageError::Io(std::io::Error::other("boom")),
        sqlite_busy(),
        sqlite_locked(),
        sqlite_corrupt(),
        sqlite_disk_full(),
        sqlite_io_failure(),
        sqlite_oom(),
        // Migration wraps a refinery error with no public constructor, so
        // it can't be synthesised here. It's omitted from the samples but
        // the `Migration(_)` arm below still keeps the match exhaustive.
        WalletStorageError::IntegrityCheckFailed {
            report: "rows missing".into(),
        },
        WalletStorageError::IntegrityCheckRunFailed {
            source: SqlErr::ExecuteReturnedResults,
        },
        WalletStorageError::SourceOpenFailed {
            source: SqlErr::ExecuteReturnedResults,
        },
        WalletStorageError::SchemaHistoryMissing,
        WalletStorageError::SchemaVersionUnsupported {
            found: 99,
            max_supported: 3,
        },
        WalletStorageError::AutoBackupDisabled {
            operation: AutoBackupOperation::DeleteWallet,
        },
        WalletStorageError::AutoBackupDirUnwritable {
            dir: PathBuf::from("/nope"),
            source: std::io::Error::other("nope"),
        },
        WalletStorageError::InsecureParentDir { mode: 0o777 },
        WalletStorageError::WalletNotFound {
            wallet_id: [0u8; 32],
        },
        WalletStorageError::WalletIdMismatch {
            expected: [1u8; 32],
            found: [2u8; 32],
        },
        WalletStorageError::IdentityKeyEntryMismatch,
        WalletStorageError::AssetLockEntryMismatch {
            typed_outpoint: "txid:0".into(),
            blob_outpoint: "txid:1".into(),
            typed_account_index: 5,
            blob_account_index: 9,
        },
        WalletStorageError::BlobTooLarge {
            len_bytes: 32 * 1024 * 1024,
            limit_bytes: 16 * 1024 * 1024,
        },
        WalletStorageError::ForeignKeysNotEnforced,
        WalletStorageError::JournalModeNotApplied {
            requested: "WAL",
            actual: "delete".into(),
        },
        WalletStorageError::SchemaHistoryMalformed {
            reason: "bad applied_on",
        },
        WalletStorageError::NotAWalletDb {
            expected: 0x504C_5754,
            found: 0,
        },
        WalletStorageError::AlreadyOpen {
            path: PathBuf::from("/x/w.db"),
        },
        WalletStorageError::LockPoisoned,
        WalletStorageError::RestoreDestinationLocked,
        WalletStorageError::InvalidWalletIdHex {
            source: hex::FromHexError::OddLength,
        },
        WalletStorageError::InvalidWalletIdLength {
            column: "wallets.wallet_id",
            actual: 10,
        },
        WalletStorageError::ConfigInvalid { reason: "bad knob" },
        WalletStorageError::IdentityEntryIdMismatch,
        WalletStorageError::OrphanedIdentityEntry { owner: [0x0E; 32] },
        WalletStorageError::AddressDecode {
            source: dashcore::address::Error::UnrecognizedScript,
        },
        WalletStorageError::AccountRegistrationEntryMismatch,
        WalletStorageError::ProviderKeyAccountEntryMismatch,
        WalletStorageError::ProviderKeyAccountConflict {
            account_type: "provider_platform",
        },
        WalletStorageError::TypedPoolKeyConflict {
            account_type: "provider_platform",
            address_index: 0,
        },
        // BincodeEncode / BincodeDecode / HashDecode / ConsensusCodec
        // need real upstream errors; omitted but covered by their arms.
        WalletStorageError::BlobDecode {
            reason: "bad shape",
        },
        WalletStorageError::BackupDestinationExists {
            path: PathBuf::from("/x"),
        },
        WalletStorageError::IntegerOverflow {
            field: "f",
            value: u64::MAX,
            target: SafeCastTarget::U64,
        },
        WalletStorageError::MissingAccount {
            wallet_id: [3u8; 32],
        },
        WalletStorageError::AccountRecordInvalid {
            e: key_wallet::error::Error::WatchOnly,
        },
        WalletStorageError::AccountRejected {
            cause: "unknown account_type".into(),
        },
        WalletStorageError::RehydrationPoolMismatch {
            expected: 2,
            found: 1,
        },
        WalletStorageError::RehydrationPoolTypeMismatch {
            position: 0,
            expected: key_wallet::managed_account::address_pool::AddressPoolType::External,
            found: key_wallet::managed_account::address_pool::AddressPoolType::Internal,
        },
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
    ]
}

/// Wildcard-free exhaustiveness gate: each variant's expected
/// `(is_transient, error_kind_str)` pair is asserted via a `match` with
/// no `_` arm.
#[test]
fn tc_p2_005_is_transient_table() {
    fn classify(err: &WalletStorageError) -> (bool, &'static str) {
        // Every arm asserts the expected (transient, kind_str) pair
        // and returns it for the outer assertion. A new variant
        // landing in WalletStorageError makes this match fail to
        // compile until classified.
        match err {
            // SQLite path discriminates by inner ErrorCode — split
            // into busy / locked / other to mirror error_kind_str.
            WalletStorageError::Sqlite(SqlErr::SqliteFailure(e, _)) => match e.code {
                ErrorCode::DatabaseBusy => (true, "sqlite_busy"),
                ErrorCode::DatabaseLocked => (true, "sqlite_locked"),
                ErrorCode::DiskFull => (true, "sqlite_disk_full"),
                ErrorCode::SystemIoFailure => (true, "sqlite_io_failure"),
                ErrorCode::OutOfMemory => (true, "sqlite_out_of_memory"),
                _ => (false, "sqlite_other"),
            },
            WalletStorageError::Sqlite(_) => (false, "sqlite_other"),
            WalletStorageError::FlushRetryable { .. } => (true, "flush_retryable"),
            WalletStorageError::Io(_) => (false, "io"),
            WalletStorageError::Migration(_) => (false, "migration"),
            WalletStorageError::IntegrityCheckFailed { .. } => (false, "integrity_check_failed"),
            WalletStorageError::IntegrityCheckRunFailed { .. } => {
                (false, "integrity_check_run_failed")
            }
            WalletStorageError::SourceOpenFailed { .. } => (false, "source_open_failed"),
            WalletStorageError::SchemaHistoryMissing => (false, "schema_history_missing"),
            WalletStorageError::SchemaVersionUnsupported { .. } => {
                (false, "schema_version_unsupported")
            }
            WalletStorageError::AutoBackupDisabled { .. } => (false, "auto_backup_disabled"),
            WalletStorageError::AutoBackupDirUnwritable { .. } => {
                (false, "auto_backup_dir_unwritable")
            }
            WalletStorageError::InsecureParentDir { .. } => (false, "insecure_parent_dir"),
            WalletStorageError::WalletNotFound { .. } => (false, "wallet_not_found"),
            WalletStorageError::WalletIdMismatch { .. } => (false, "wallet_id_mismatch"),
            WalletStorageError::LockPoisoned => (false, "lock_poisoned"),
            WalletStorageError::RestoreDestinationLocked => (false, "restore_destination_locked"),
            WalletStorageError::InvalidWalletIdHex { .. } => (false, "invalid_wallet_id_hex"),
            WalletStorageError::InvalidWalletIdLength { .. } => (false, "invalid_wallet_id_length"),
            WalletStorageError::ConfigInvalid { .. } => (false, "config_invalid"),
            WalletStorageError::BincodeEncode { .. } => (false, "bincode_encode"),
            WalletStorageError::BincodeDecode { .. } => (false, "bincode_decode"),
            WalletStorageError::BlobDecode { .. } => (false, "blob_decode"),
            WalletStorageError::HashDecode { .. } => (false, "hash_decode"),
            WalletStorageError::ConsensusCodec { .. } => (false, "consensus_codec"),
            WalletStorageError::AddressDecode { .. } => (false, "address_decode"),
            WalletStorageError::BackupDestinationExists { .. } => {
                (false, "backup_destination_exists")
            }
            WalletStorageError::IdentityKeyEntryMismatch => (false, "identity_key_entry_mismatch"),
            WalletStorageError::IdentityEntryIdMismatch => (false, "identity_entry_id_mismatch"),
            WalletStorageError::OrphanedIdentityEntry { .. } => (false, "orphaned_identity_entry"),
            WalletStorageError::AssetLockEntryMismatch { .. } => {
                (false, "asset_lock_entry_mismatch")
            }
            WalletStorageError::BlobTooLarge { .. } => (false, "blob_too_large"),
            WalletStorageError::ForeignKeysNotEnforced => (false, "foreign_keys_not_enforced"),
            WalletStorageError::JournalModeNotApplied { .. } => (false, "journal_mode_not_applied"),
            WalletStorageError::SchemaHistoryMalformed { .. } => {
                (false, "schema_history_malformed")
            }
            WalletStorageError::NotAWalletDb { .. } => (false, "not_a_wallet_db"),
            WalletStorageError::AlreadyOpen { .. } => (false, "already_open"),
            WalletStorageError::IntegerOverflow { .. } => (false, "integer_overflow"),
            WalletStorageError::AccountRegistrationEntryMismatch => {
                (false, "account_registration_entry_mismatch")
            }
            WalletStorageError::ProviderKeyAccountEntryMismatch => {
                (false, "provider_key_account_entry_mismatch")
            }
            WalletStorageError::ProviderKeyAccountConflict { .. } => {
                (false, "provider_key_account_conflict")
            }
            WalletStorageError::TypedPoolKeyConflict { .. } => (false, "typed_pool_key_conflict"),
            WalletStorageError::MissingAccount { .. } => {
                (false, "missing_account_registration_entry")
            }
            WalletStorageError::AccountRecordInvalid { .. } => (false, "account_record_invalid"),
            WalletStorageError::AccountRejected { .. } => (false, "account_rejected"),
            WalletStorageError::RehydrationPoolMismatch { .. } => {
                (false, "rehydration_pool_mismatch")
            }
            WalletStorageError::RehydrationPoolTypeMismatch { .. } => {
                (false, "rehydration_pool_type_mismatch")
            }
        }
    }

    for err in samples() {
        let (expected_transient, expected_kind) = classify(&err);
        assert_eq!(
            err.is_transient(),
            expected_transient,
            "is_transient mismatch for variant `{expected_kind}`: got {}",
            err.is_transient()
        );
        assert_eq!(
            err.error_kind_str(),
            expected_kind,
            "error_kind_str mismatch for variant `{expected_kind}`: got {}",
            err.error_kind_str()
        );
    }
}

/// `FlushRetryable` flowing through the `From` impl into
/// `PersistenceError::Backend { kind, source }`: the outer
/// `Display` carries the variant markers ops grep for, and the typed
/// source chain still reaches the inner rusqlite payload (consumers
/// downcast or `Error::source`-walk to get there).
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
    let source = match pe {
        PersistenceError::Backend { source, .. } => source,
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    };
    let outer = source.to_string();
    assert!(
        outer.contains("FlushRetryable"),
        "missing FlushRetryable variant marker: {outer}"
    );
    assert!(
        outer.contains("flush failed transiently"),
        "missing FlushRetryable display body: {outer}"
    );
    assert!(
        outer.contains("abab"),
        "missing wallet_id hex prefix: {outer}"
    );

    // Walk the typed source chain to the inner rusqlite payload: the
    // source is `Box<dyn Error + Send + Sync>`, so the chain is preserved
    // structurally, not just stringified.
    let mut chain = String::new();
    let mut cur: Option<&(dyn std::error::Error + 'static)> = source.source();
    while let Some(e) = cur {
        chain.push_str(&e.to_string());
        chain.push('\n');
        cur = e.source();
    }
    assert!(
        chain.contains("database is locked"),
        "inner source text missing from chain walk: {chain}"
    );
}
