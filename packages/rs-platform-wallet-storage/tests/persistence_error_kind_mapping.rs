//! `WalletStorageError -> PersistenceError` kind-classification table.
//!
//! Every `WalletStorageError` variant carries through the boundary
//! with the right `PersistenceErrorKind` (`Transient` / `Fatal` /
//! `Constraint`). The `From` impl in `sqlite/error.rs` is the single
//! source of truth; this test pins the mapping so changes to it are
//! deliberate.
//!
//! `WalletStorageError::is_transient()` and
//! `WalletStorageError::error_kind_str()` must remain wildcard-free so
//! adding a new variant forces an explicit classification update. This
//! test parses the source file and refuses to compile around a `_ =>`
//! arm in either method.

use std::path::PathBuf;

use platform_wallet::changeset::{PersistenceError, PersistenceErrorKind};
use platform_wallet_storage::sqlite::error::{AutoBackupOperation, WalletStorageError};
use platform_wallet_storage::sqlite::util::safe_cast::SafeCastTarget;
use rusqlite::ErrorCode;

/// Classify a converted `PersistenceError` to its `PersistenceErrorKind`.
/// Panics if the converted error is `LockPoisoned`, which is its own
/// trait-level variant rather than a `Backend { kind, .. }`.
fn kind_of(err: WalletStorageError) -> PersistenceErrorKind {
    match PersistenceError::from(err) {
        PersistenceError::Backend { kind, .. } => kind,
        PersistenceError::LockPoisoned => {
            panic!("LockPoisoned has no Backend.kind — test was given LockPoisoned by accident")
        }
    }
}

fn sqlite_failure(code: ErrorCode, extended: i32) -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code,
            extended_code: extended,
        },
        Some("simulated".into()),
    ))
}

/// `LockPoisoned` keeps its dedicated variant.
#[test]
fn tc_code_004_b_lock_poisoned_maps_to_lock_poisoned() {
    let pe: PersistenceError = WalletStorageError::LockPoisoned.into();
    assert!(matches!(pe, PersistenceError::LockPoisoned));
}

/// Every `is_transient() == true` variant maps to
/// `PersistenceErrorKind::Transient` at the trait boundary.
#[test]
fn tc_code_004_b_transient_variants_map_to_transient_kind() {
    let transient_cases = [
        ("DatabaseBusy", sqlite_failure(ErrorCode::DatabaseBusy, 5)),
        (
            "DatabaseLocked",
            sqlite_failure(ErrorCode::DatabaseLocked, 6),
        ),
        ("DiskFull", sqlite_failure(ErrorCode::DiskFull, 13)),
        (
            "SystemIoFailure",
            sqlite_failure(ErrorCode::SystemIoFailure, 10),
        ),
        ("OutOfMemory", sqlite_failure(ErrorCode::OutOfMemory, 7)),
        (
            "FlushRetryable",
            WalletStorageError::FlushRetryable {
                wallet_id: [0xAB; 32],
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::DatabaseBusy,
                        extended_code: 5,
                    },
                    Some("busy".into()),
                ),
            },
        ),
    ];

    for (label, err) in transient_cases {
        assert!(
            err.is_transient(),
            "{label}: WalletStorageError::is_transient must be true"
        );
        assert_eq!(
            kind_of(err),
            PersistenceErrorKind::Transient,
            "{label}: trait-boundary kind must be Transient"
        );
    }
}

/// SQLite constraint failures map to
/// `PersistenceErrorKind::Constraint` so consumers can distinguish
/// "your data is wrong" from "the storage engine is unhappy".
#[test]
fn tc_code_004_b_constraint_variants_map_to_constraint_kind() {
    let constraint_codes = [
        ErrorCode::ConstraintViolation,
        // Specific constraint sub-codes covered by SQLite's extended
        // error codes — checked via the outer ErrorCode group.
    ];
    for code in constraint_codes {
        let err = sqlite_failure(code, 19);
        assert!(
            !err.is_transient(),
            "constraint must not be transient ({code:?})"
        );
        assert_eq!(
            kind_of(err),
            PersistenceErrorKind::Constraint,
            "{code:?} must map to Constraint"
        );
    }
}

/// Identity-slot uniqueness is enforced in Rust rather than by a SQL
/// constraint, so its variants have to claim the `Constraint` kind
/// explicitly — they are caller-data violations like any FK breach.
#[test]
fn tc_code_004_b_identity_index_variants_map_to_constraint_kind() {
    let cases: Vec<(&str, WalletStorageError)> = vec![
        (
            "IdentityIndexConflict",
            WalletStorageError::IdentityIndexConflict {
                wallet_id: [0xAA; 32],
                identity_index: 1,
                existing: [0xBB; 32],
                incoming: [0xCC; 32],
            },
        ),
        (
            "WalletlessIdentityIndex",
            WalletStorageError::WalletlessIdentityIndex {
                identity_id: [0xDD; 32],
                identity_index: 2,
            },
        ),
    ];
    for (label, err) in cases {
        assert!(!err.is_transient(), "{label}: must not be transient");
        assert_eq!(
            kind_of(err),
            PersistenceErrorKind::Constraint,
            "{label}: trait-boundary kind must be Constraint"
        );
    }
}

/// Every remaining fatal-but-not-constraint variant maps to `Fatal`.
/// Spot-check enough variants to lock the table; the
/// exhaustiveness is guarded by the wildcard-free invariant test.
#[test]
fn tc_code_004_b_fatal_variants_map_to_fatal_kind() {
    let fatal_cases: Vec<(&str, WalletStorageError)> = vec![
        ("Io", WalletStorageError::Io(std::io::Error::other("io"))),
        (
            "Sqlite-other",
            WalletStorageError::Sqlite(rusqlite::Error::InvalidColumnIndex(0)),
        ),
        (
            "IntegrityCheckFailed",
            WalletStorageError::IntegrityCheckFailed {
                report: "bad".into(),
            },
        ),
        (
            "SchemaHistoryMissing",
            WalletStorageError::SchemaHistoryMissing,
        ),
        (
            "SchemaVersionUnsupported",
            WalletStorageError::SchemaVersionUnsupported {
                found: 9,
                max_supported: 2,
            },
        ),
        (
            "AutoBackupDisabled",
            WalletStorageError::AutoBackupDisabled {
                operation: AutoBackupOperation::DeleteWallet,
            },
        ),
        (
            "AutoBackupDirUnwritable",
            WalletStorageError::AutoBackupDirUnwritable {
                dir: PathBuf::from("/nope"),
                source: std::io::Error::other("io"),
            },
        ),
        (
            "InsecureParentDir",
            WalletStorageError::InsecureParentDir { mode: 0o777 },
        ),
        (
            "WalletNotFound",
            WalletStorageError::WalletNotFound {
                wallet_id: [0xCD; 32],
            },
        ),
        (
            "WalletIdMismatch",
            WalletStorageError::WalletIdMismatch {
                expected: [0xAA; 32],
                found: [0xBB; 32],
            },
        ),
        (
            "RestoreDestinationLocked",
            WalletStorageError::RestoreDestinationLocked,
        ),
        (
            "InvalidWalletIdLength",
            WalletStorageError::InvalidWalletIdLength {
                column: "wallets.wallet_id",
                actual: 12,
            },
        ),
        (
            "ConfigInvalid",
            WalletStorageError::ConfigInvalid {
                reason: "synchronous=Off",
            },
        ),
        (
            "BlobDecode",
            WalletStorageError::BlobDecode { reason: "len" },
        ),
        (
            "ForeignKeysNotEnforced",
            WalletStorageError::ForeignKeysNotEnforced,
        ),
        (
            "IdentityKeyEntryMismatch",
            WalletStorageError::IdentityKeyEntryMismatch,
        ),
        (
            "BlobTooLarge",
            WalletStorageError::BlobTooLarge {
                len_bytes: 1,
                limit_bytes: 0,
            },
        ),
        (
            "IntegerOverflow",
            WalletStorageError::IntegerOverflow {
                field: "x",
                value: 1,
                target: SafeCastTarget::I64,
            },
        ),
        (
            "BackupDestinationExists",
            WalletStorageError::BackupDestinationExists {
                path: PathBuf::from("/tmp/x"),
            },
        ),
        (
            "ReadOnlyRecoveryMode",
            WalletStorageError::ReadOnlyRecoveryMode { operation: "store" },
        ),
        (
            "RehydrationEnsureDerivedFailed",
            WalletStorageError::RehydrationEnsureDerivedFailed { index: 42 },
        ),
        (
            "RehydrationGapLimitRefillTooLarge",
            WalletStorageError::RehydrationGapLimitRefillTooLarge {
                refill_target: 300_000,
                already_generated: 20,
                implied: 299_980,
                cap: 250_000,
            },
        ),
        (
            "RehydrationGapLimitFailed",
            WalletStorageError::RehydrationGapLimitFailed {
                source: key_wallet::error::Error::WatchOnly,
            },
        ),
        (
            "UsedAddressOwnerConflict",
            WalletStorageError::UsedAddressOwnerConflict {
                address: "yaddr".into(),
                pool_owner: "Standard[0]".into(),
                utxo_owner: "CoinJoin[0]".into(),
            },
        ),
        (
            "UnownedIdentityHasRegistrationIndex",
            WalletStorageError::UnownedIdentityHasRegistrationIndex {
                identity_id: [0xEF; 32],
                identity_index: 3,
            },
        ),
    ];

    for (label, err) in fatal_cases {
        assert!(!err.is_transient(), "{label}: must not be transient");
        assert_eq!(
            kind_of(err),
            PersistenceErrorKind::Fatal,
            "{label}: trait-boundary kind must be Fatal"
        );
    }
}

/// The boxed source preserves the typed `Error` chain so consumers can
/// walk it (the trait was the right boundary
/// for `Box<dyn Error + Send + Sync>` precisely so the rusqlite
/// source is recoverable). The outer `Display` carries the variant
/// marker ops grep for; `.source()` walks to the inner `rusqlite`
/// payload.
#[test]
fn tc_code_004_b_source_preserves_inner_display_chain() {
    let err = WalletStorageError::FlushRetryable {
        wallet_id: [0xAB; 32],
        source: rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("database is locked".into()),
        ),
    };
    let pe: PersistenceError = err.into();
    match pe {
        PersistenceError::Backend { kind, source } => {
            assert_eq!(kind, PersistenceErrorKind::Transient);
            let outer = source.to_string();
            assert!(outer.contains("FlushRetryable"), "marker missing: {outer}");
            assert!(
                outer.contains("flush failed transiently"),
                "body missing: {outer}"
            );
            assert!(outer.contains("abab"), "wallet_id hex missing: {outer}");

            // Walk the typed source chain — that's the whole point of
            // boxing the typed error rather than stringifying it.
            let mut chain_text = String::new();
            let mut cur: Option<&(dyn std::error::Error + 'static)> = source.source();
            while let Some(e) = cur {
                chain_text.push_str(&e.to_string());
                chain_text.push('\n');
                cur = e.source();
            }
            assert!(
                chain_text.contains("database is locked"),
                "inner source text missing from chain walk: {chain_text}"
            );
        }
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    }
}

/// `is_transient()` source must not regress to a wildcard arm on its
/// outer `match self`. The inner match on
/// `rusqlite::ErrorCode` is allowed to use a wildcard since
/// `ErrorCode` is `#[non_exhaustive]` upstream — we only guard the
/// outer `WalletStorageError` match.
#[test]
fn tc_code_004_e_is_transient_outer_match_is_wildcard_free() {
    let src = include_str!("../src/sqlite/error.rs");
    let outer = extract_outer_match_self_body(src, "pub fn is_transient(&self) -> bool")
        .expect("is_transient outer match body must be present");
    assert_no_wildcard(&outer, "is_transient");
}

/// Same guard for `error_kind_str()`. The outer match over
/// `WalletStorageError` MUST remain wildcard-free; the inner
/// match over `ErrorCode` may have its own wildcard.
#[test]
fn tc_code_004_e_error_kind_str_is_wildcard_free() {
    let src = include_str!("../src/sqlite/error.rs");
    let outer = extract_outer_match_self_body(src, "pub fn error_kind_str(&self) -> &'static str")
        .expect("error_kind_str outer match body must be present");
    assert_no_wildcard(&outer, "error_kind_str");
}

/// Locate the first `match self {` block inside `fn` `signature` and
/// return only its top-level body (arms at brace-depth = 0 relative
/// to the outer match). Nested matches and tuple patterns at deeper
/// depths are excluded so an inner `_ =>` on `ErrorCode` (which is
/// upstream-`#[non_exhaustive]`) doesn't trip the invariant.
fn extract_outer_match_self_body(src: &str, signature: &str) -> Option<String> {
    let start = src.find(signature)?;
    let after_sig = &src[start..];
    // Find the `match self {` opening *after* the signature. The
    // intervening braces from the fn body are handled by depth
    // counting below.
    let match_kw = after_sig.find("match self")?;
    let open = after_sig[match_kw..].find('{')? + match_kw;
    let bytes = after_sig.as_bytes();
    let mut depth = 0usize;
    let mut top_level_arms = String::new();
    let mut i = open;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'{' {
            depth += 1;
            if depth > 1 {
                // Skip past the nested block.
                let close = find_matching_close(bytes, i)?;
                i = close + 1;
                depth -= 1;
                continue;
            }
        } else if b == b'}' {
            if depth == 1 {
                return Some(top_level_arms);
            }
            depth -= 1;
        } else if depth == 1 {
            top_level_arms.push(b as char);
        }
        i += 1;
    }
    None
}

fn find_matching_close(bytes: &[u8], open_idx: usize) -> Option<usize> {
    debug_assert_eq!(bytes[open_idx], b'{');
    let mut depth = 0usize;
    for (j, &b) in bytes.iter().enumerate().skip(open_idx) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

fn assert_no_wildcard(outer_body: &str, fn_name: &str) {
    for line in outer_body.lines() {
        let t = line.trim_start();
        // A wildcard arm is either bare `_` or `_ if guard` at the
        // start of an arm. Catch the canonical forms operators write.
        let is_wildcard_arm = t.starts_with("_ =>")
            || t.starts_with("_=>")
            || t.starts_with("_ if ")
            || t == "_,"
            || t.starts_with("_ |");
        assert!(
            !is_wildcard_arm,
            "{fn_name}: outer Self match must remain wildcard-free; offending arm: `{line}`"
        );
    }
}
