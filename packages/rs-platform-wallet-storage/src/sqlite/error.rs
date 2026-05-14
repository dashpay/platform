//! Typed errors for `platform-wallet-storage`.
//!
//! Every variant carries the upstream error via `#[source]` (or
//! `#[from]` where the conversion is the only thing the trait does),
//! never via a stringified copy. Variants never store user-facing
//! prose — the `#[error("...")]` attribute provides the renderable
//! `Display` form; the typed fields carry diagnostics.
//!
//! At the `PlatformWalletPersistence` trait boundary, this type
//! converts into `PersistenceError`: `LockPoisoned` keeps its
//! dedicated variant, everything else flows through
//! `PersistenceError::Backend` with the full `Display` chain.

use std::path::PathBuf;

use platform_wallet::changeset::PersistenceError;

use crate::sqlite::util::safe_cast::SafeCastTarget;

/// Which automatic-backup operation was attempted when the
/// configured backup directory was missing or otherwise unwritable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AutoBackupOperation {
    #[error("open (pending migration)")]
    OpenMigration,
    #[error("delete_wallet")]
    DeleteWallet,
    #[error("restore_from")]
    Restore,
}

/// Errors produced by the wallet-storage SQLite backend.
///
/// `SqlitePersisterError` is preserved as a deprecated alias for one
/// cycle; new code should use `WalletStorageError`.
#[derive(Debug, thiserror::Error)]
pub enum WalletStorageError {
    /// File-system I/O error reaching the database or backup files.
    #[error("io error")]
    Io(#[from] std::io::Error),

    /// Error from rusqlite — covers SQL errors, busy timeouts, and
    /// schema-level failures alike. The inner `rusqlite::Error`
    /// already discriminates between them.
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),

    /// Refinery migration runner failure.
    #[error("migration error")]
    Migration(#[from] refinery::Error),

    /// The migration runner left the schema in an inconsistent state
    /// (some migrations applied, some still pending).
    #[error(
        "migration left the database in a dirty state \
         (applied={applied} pending={pending})"
    )]
    MigrationDirty { applied: usize, pending: usize },

    /// `PRAGMA integrity_check` ran successfully but reported a
    /// non-`ok` result. `report` carries SQLite's own diagnostic
    /// text — not a user-facing message, not a stringified source.
    #[error("integrity check failed: {report}")]
    IntegrityCheckFailed { report: String },

    /// Failed to even run the integrity-check pragma.
    #[error("integrity check could not run")]
    IntegrityCheckRunFailed {
        #[source]
        source: rusqlite::Error,
    },

    /// Cannot open the candidate source database file (most likely
    /// not a SQLite database at all, or bytes are torn).
    #[error("cannot open candidate source database")]
    SourceOpenFailed {
        #[source]
        source: rusqlite::Error,
    },

    /// Source backup file lacks the `refinery_schema_history` table —
    /// it isn't a wallet-storage database.
    #[error("source backup is missing schema_history (not a platform-wallet-storage database)")]
    SchemaHistoryMissing,

    /// Source backup carries a schema version beyond what this build
    /// can apply.
    #[error(
        "source backup schema version {found} is beyond the supported maximum {max_supported}"
    )]
    SchemaVersionUnsupported { found: i64, max_supported: i64 },

    /// A destructive operation needed an automatic backup but the
    /// configuration disabled them.
    #[error("auto-backup is disabled for operation: {operation}")]
    AutoBackupDisabled { operation: AutoBackupOperation },

    /// The configured auto-backup directory could not be created or
    /// written to.
    #[error("auto-backup directory {} could not be prepared", dir.display())]
    AutoBackupDirUnwritable {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `delete_wallet` (or another wallet-id-keyed operation) was
    /// called with an id that has no matching `wallet_metadata` row.
    #[error("wallet not found: {}", hex::encode(wallet_id))]
    WalletNotFound { wallet_id: [u8; 32] },

    /// A previous holder of an internal mutex panicked. Maps to the
    /// trait-level [`PersistenceError::LockPoisoned`] so callers can
    /// still pattern-match the boundary variant cleanly.
    #[error("persister lock poisoned")]
    LockPoisoned,

    /// `restore_from` tried to acquire an exclusive file-lock on the
    /// destination and couldn't — another process is holding it open.
    #[error("restore destination is locked or in use")]
    RestoreDestinationLocked,

    /// A wallet-id hex string couldn't be parsed.
    #[error("invalid wallet id: bad hex")]
    InvalidWalletIdHex {
        #[source]
        source: hex::FromHexError,
    },

    /// A wallet-id hex string had the wrong length (must be 64 chars
    /// for a 32-byte id).
    #[error("invalid wallet id length: expected 64 hex chars, got {actual}")]
    InvalidWalletIdLength { actual: usize },

    /// A `SqlitePersisterConfig` field carries an unsupported value
    /// (e.g. `synchronous = Off`). The `reason` is a compile-time
    /// `&'static str` constant naming the rejected setting.
    #[error("invalid configuration: {reason}")]
    ConfigInvalid { reason: &'static str },

    /// bincode-serde refused to encode a value (typically because
    /// the value's serde representation needs `deserialize_any`-style
    /// dispatch — see dpp's `IdentityPublicKey` workaround).
    #[error("bincode encode error")]
    BincodeEncode {
        #[source]
        source: bincode::error::EncodeError,
    },

    /// bincode-serde refused to decode a payload.
    #[error("bincode decode error")]
    BincodeDecode {
        #[source]
        source: bincode::error::DecodeError,
    },

    /// A typed-column decode failed (e.g. outpoint had the wrong
    /// length, or a column held a value the schema doesn't recognise).
    #[error("blob/column decode failed: {reason}")]
    BlobDecode { reason: &'static str },

    /// A typed-column decode failed because an underlying
    /// `dashcore::hashes` deserialisation rejected the bytes.
    #[error("hash decode failed")]
    HashDecode {
        #[source]
        source: dashcore::hashes::Error,
    },

    /// A `dashcore` consensus encode/decode failed.
    #[error("dashcore consensus encoding failed")]
    ConsensusCodec {
        #[source]
        source: dashcore::consensus::encode::Error,
    },

    /// The CLI's `backup` subcommand refuses to overwrite an existing
    /// destination file.
    #[error("backup destination already exists: {}", path.display())]
    BackupDestinationExists { path: PathBuf },

    /// A value couldn't be cast to the database's native i64
    /// representation without losing magnitude.
    #[error("integer overflow casting `{field}` (value={value}) to {target}")]
    IntegerOverflow {
        field: &'static str,
        value: u64,
        target: SafeCastTarget,
    },

    /// A `load()` call succeeded but skipped some sub-areas because
    /// their reconstruction is not yet implemented. The `unimplemented`
    /// list names the affected `ClientStartState` field paths so
    /// callers can decide whether to proceed.
    ///
    /// `load()` itself returns `Ok(ClientStartState)` and surfaces
    /// the same information via `tracing::warn!`; this variant exists
    /// for callers that route through trait-error propagation paths
    /// or explicitly want partial-completion as a value.
    #[error(
        "load() did not reconstruct {} sub-area(s); unimplemented: {unimplemented:?}",
        unimplemented.len()
    )]
    LoadIncomplete {
        unimplemented: &'static [&'static str],
    },

    /// Flush failed transiently (e.g. `SQLITE_BUSY` / `SQLITE_LOCKED`)
    /// for `wallet_id`. The buffered changeset has been restored — the
    /// next `flush(wallet_id)` will retry the same data merged with
    /// anything stored in between. Callers should back off and retry
    /// rather than dropping state.
    #[error(
        "flush failed transiently for wallet {}; buffer preserved for retry",
        hex::encode(wallet_id)
    )]
    FlushRetryable {
        wallet_id: [u8; 32],
        #[source]
        source: rusqlite::Error,
    },
}

/// Deprecated alias preserved for one cycle. Switch downstream
/// references to [`WalletStorageError`].
#[deprecated(since = "3.1.0-dev.1", note = "renamed to WalletStorageError")]
pub type SqlitePersisterError = WalletStorageError;

impl From<WalletStorageError> for PersistenceError {
    fn from(err: WalletStorageError) -> Self {
        match err {
            WalletStorageError::LockPoisoned => PersistenceError::LockPoisoned,
            other => PersistenceError::Backend(format!("{}", DisplayChain(&other))),
        }
    }
}

/// Renders an error and its `#[source]` chain for the
/// `PersistenceError::Backend` (`String`) boundary. The trait can't
/// carry typed sources, so the chain is concatenated for diagnostic
/// purposes — every typed variant is still preserved on the
/// `WalletStorageError` value the trait `From` impl consumes.
struct DisplayChain<'a>(&'a WalletStorageError);

impl std::fmt::Display for DisplayChain<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::error::Error;
        write!(f, "{}", self.0)?;
        let mut cur: Option<&dyn Error> = self.0.source();
        while let Some(err) = cur {
            write!(f, ": {err}")?;
            cur = err.source();
        }
        Ok(())
    }
}

impl WalletStorageError {
    /// Construct a typed `BlobDecode` error from a static reason.
    /// Used by schema modules that hit a structural decode error
    /// (e.g. an outpoint column that isn't 36 bytes).
    pub(crate) fn blob_decode(reason: &'static str) -> Self {
        Self::BlobDecode { reason }
    }

    /// `true` when the underlying failure is safe to retry — the
    /// caller should preserve in-flight state and call again. Today
    /// only `SQLITE_BUSY` / `SQLITE_LOCKED` (raw or wrapped via
    /// [`Self::FlushRetryable`]) qualify; every other variant is
    /// fatal.
    ///
    /// The match is intentionally wildcard-free: `WalletStorageError`
    /// MUST NOT gain `#[non_exhaustive]`, otherwise adding a future
    /// variant would skip this gate (it'd silently fall into a
    /// catch-all instead of forcing the author to classify it).
    pub fn is_transient(&self) -> bool {
        use rusqlite::ErrorCode;
        match self {
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _)) => {
                matches!(e.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
            }
            Self::FlushRetryable { .. } => true,
            // Every other rusqlite variant — non-`SqliteFailure` (e.g.
            // `ToSqlConversionFailure`, `InvalidColumnIndex`) — is a
            // logic bug, not a contention failure.
            Self::Sqlite(_) => false,
            Self::Io(_)
            | Self::Migration(_)
            | Self::MigrationDirty { .. }
            | Self::IntegrityCheckFailed { .. }
            | Self::IntegrityCheckRunFailed { .. }
            | Self::SourceOpenFailed { .. }
            | Self::SchemaHistoryMissing
            | Self::SchemaVersionUnsupported { .. }
            | Self::AutoBackupDisabled { .. }
            | Self::AutoBackupDirUnwritable { .. }
            | Self::WalletNotFound { .. }
            | Self::LockPoisoned
            | Self::RestoreDestinationLocked
            | Self::InvalidWalletIdHex { .. }
            | Self::InvalidWalletIdLength { .. }
            | Self::ConfigInvalid { .. }
            | Self::BincodeEncode { .. }
            | Self::BincodeDecode { .. }
            | Self::BlobDecode { .. }
            | Self::HashDecode { .. }
            | Self::ConsensusCodec { .. }
            | Self::BackupDestinationExists { .. }
            | Self::IntegerOverflow { .. }
            | Self::LoadIncomplete { .. } => false,
        }
    }

    /// Short, lowercase, snake-case tag for tracing fields. One tag
    /// per variant family — readers grep for these in production
    /// logs.
    pub fn error_kind_str(&self) -> &'static str {
        use rusqlite::ErrorCode;
        match self {
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _)) => match e.code {
                ErrorCode::DatabaseBusy => "sqlite_busy",
                ErrorCode::DatabaseLocked => "sqlite_locked",
                _ => "sqlite_other",
            },
            Self::Sqlite(_) => "sqlite_other",
            Self::FlushRetryable { .. } => "flush_retryable",
            Self::Io(_) => "io",
            Self::Migration(_) => "migration",
            Self::MigrationDirty { .. } => "migration_dirty",
            Self::IntegrityCheckFailed { .. } => "integrity_check_failed",
            Self::IntegrityCheckRunFailed { .. } => "integrity_check_run_failed",
            Self::SourceOpenFailed { .. } => "source_open_failed",
            Self::SchemaHistoryMissing => "schema_history_missing",
            Self::SchemaVersionUnsupported { .. } => "schema_version_unsupported",
            Self::AutoBackupDisabled { .. } => "auto_backup_disabled",
            Self::AutoBackupDirUnwritable { .. } => "auto_backup_dir_unwritable",
            Self::WalletNotFound { .. } => "wallet_not_found",
            Self::LockPoisoned => "lock_poisoned",
            Self::RestoreDestinationLocked => "restore_destination_locked",
            Self::InvalidWalletIdHex { .. } => "invalid_wallet_id_hex",
            Self::InvalidWalletIdLength { .. } => "invalid_wallet_id_length",
            Self::ConfigInvalid { .. } => "config_invalid",
            Self::BincodeEncode { .. } => "bincode_encode",
            Self::BincodeDecode { .. } => "bincode_decode",
            Self::BlobDecode { .. } => "blob_decode",
            Self::HashDecode { .. } => "hash_decode",
            Self::ConsensusCodec { .. } => "consensus_codec",
            Self::BackupDestinationExists { .. } => "backup_destination_exists",
            Self::IntegerOverflow { .. } => "integer_overflow",
            Self::LoadIncomplete { .. } => "load_incomplete",
        }
    }
}

impl From<bincode::error::EncodeError> for WalletStorageError {
    fn from(source: bincode::error::EncodeError) -> Self {
        Self::BincodeEncode { source }
    }
}

impl From<bincode::error::DecodeError> for WalletStorageError {
    fn from(source: bincode::error::DecodeError) -> Self {
        Self::BincodeDecode { source }
    }
}

impl From<dashcore::hashes::Error> for WalletStorageError {
    fn from(source: dashcore::hashes::Error) -> Self {
        Self::HashDecode { source }
    }
}

impl From<dashcore::consensus::encode::Error> for WalletStorageError {
    fn from(source: dashcore::consensus::encode::Error) -> Self {
        Self::ConsensusCodec { source }
    }
}
