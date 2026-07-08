//! Typed errors for `platform-wallet-storage`.
//!
//! Variants carry the upstream error via `#[source]`/`#[from]`, never a
//! stringified copy; the `#[error("...")]` attribute provides `Display`.
//!
//! At the `PlatformWalletPersistence` boundary this converts into
//! `PersistenceError`: `LockPoisoned` keeps its dedicated variant, and
//! everything else flows through `Backend { kind, source }` where `kind`
//! comes from [`WalletStorageError::persistence_kind`] and `source`
//! preserves the typed error for `Error::source()` walking.

use std::path::PathBuf;

use platform_wallet::changeset::{PersistenceError, PersistenceErrorKind};

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

    /// `PRAGMA integrity_check` ran successfully but reported a
    /// non-`ok` result. `report` carries SQLite's own diagnostic
    /// text — not a user-facing message, not a stringified source.
    /// May be multi-line (`\n`-joined): SQLite returns one row per
    /// detected problem and the helper preserves every line.
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
    /// called with an id that has no matching `wallets` row.
    #[error("wallet not found: {}", hex::encode(wallet_id))]
    WalletNotFound { wallet_id: [u8; 32] },

    /// A changeset entry named a `wallet_id` different from the wallet
    /// the flush is scoped to — writing it would mis-file the row under
    /// the wrong parent.
    #[error(
        "wallet id mismatch: entry names {} but flush is scoped to {}",
        hex::encode(found),
        hex::encode(expected)
    )]
    WalletIdMismatch { expected: [u8; 32], found: [u8; 32] },

    /// A previous holder of an internal mutex panicked. Maps to the
    /// trait-level [`PersistenceError::LockPoisoned`] so callers can
    /// still pattern-match the boundary variant cleanly.
    #[error("persister lock poisoned")]
    LockPoisoned,

    /// `restore_from` tried to take a SQLite-native `BEGIN EXCLUSIVE`
    /// on the destination and a peer (another `SqlitePersister`, a
    /// bare `rusqlite::Connection`, the CLI) is holding it busy
    /// beyond `busy_timeout`.
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

    /// An `identity_keys` upsert entry's `(identity_id, key_id,
    /// wallet_id)` fields disagreed with the map key / flush scope the
    /// typed columns are bound from — persisting it would leave the
    /// typed columns and the serialized blob describing different rows.
    #[error("identity key entry fields disagree with its map key / wallet scope")]
    IdentityKeyEntryMismatch,

    /// An `identities` upsert entry's `id` disagreed with the map key the
    /// `identity_id` column is bound from — persisting it would leave the
    /// typed id column and the serialized blob naming different
    /// identities.
    #[error("identity entry id disagrees with its map key")]
    IdentityEntryIdMismatch,

    /// An `account_registrations` row's typed `(account_type, account_index)`
    /// columns disagreed with the decoded `AccountRegistrationEntry` blob.
    /// Rejected at decode time so the manifest oracle never hands back an
    /// entry that names a different account type or index than the indexed
    /// columns it was selected by.
    #[error(
        "account_registrations entry fields disagree with typed columns \
         (typed columns vs blob account_type or account_index mismatch)"
    )]
    AccountRegistrationEntryMismatch,

    /// Account was rejected by the wallet manager (e.g. `account_type` is unknown, or
    /// `account_index` is out of range). The `cause` is a static string describing the reason.
    #[error("account rejected by wallet manager: {cause}")]
    AccountRejected { cause: String },

    /// An `account_registrations` row is missing for a given `(account_type, account_index)`.
    #[error("required account information is missing for wallet_id {wallet_id:?}")]
    MissingAccount { wallet_id: [u8; 32] },

    /// Account record is invalid
    #[error("account record is corrupted or invalid: {e}")]
    AccountRecordInvalid {
        #[source]
        e: key_wallet::error::Error,
    },

    /// An `asset_locks` row's typed-column `(outpoint, account_index)`
    /// disagreed with the lifecycle blob's. Rejected at decode time rather
    /// than mis-bucketing the lock under the wrong account.
    #[error(
        "asset_lock entry fields disagree with typed columns \
         (typed outpoint={typed_outpoint}, blob outpoint={blob_outpoint}, \
          typed account_index={typed_account_index}, blob account_index={blob_account_index})"
    )]
    AssetLockEntryMismatch {
        typed_outpoint: String,
        blob_outpoint: String,
        typed_account_index: u32,
        blob_account_index: u32,
    },

    /// A blob exceeded the decode allocation cap (default 16 MiB).
    /// Separate from [`Self::BlobDecode`] so operators can distinguish an
    /// oversize blob from a structural decode failure.
    #[error("blob exceeded decode size limit ({len_bytes} bytes > {limit_bytes} byte cap)")]
    BlobTooLarge {
        len_bytes: usize,
        limit_bytes: usize,
    },

    /// `PRAGMA foreign_keys = ON` was issued on open but the read-back
    /// reported the constraint enforcement is still off — the linked
    /// SQLite build silently ignores the pragma (no FK support compiled
    /// in). Hard-error at open rather than letting orphan rows accrue.
    #[error("SQLite foreign-key enforcement could not be enabled on this connection")]
    ForeignKeysNotEnforced,

    /// The requested `journal_mode` read back as a different mode —
    /// SQLite silently fell back (e.g. WAL→DELETE on some FUSE mounts).
    /// With `synchronous=NORMAL` that risks corruption on power loss, so
    /// open hard-errors instead of running downgraded.
    #[error("journal_mode {requested} could not be applied (SQLite reports {actual})")]
    JournalModeNotApplied {
        requested: &'static str,
        actual: String,
    },

    /// A pre-existing / restored DB passed `integrity_check` but its
    /// `refinery_schema_history` carries a malformed row (non-RFC3339
    /// `applied_on` or non-numeric `checksum`). Probed BEFORE refinery
    /// runs so a foreign or corrupted-but-integrity-valid input returns
    /// a typed error instead of refinery panicking on the parse.
    #[error("refinery_schema_history is malformed: {reason}")]
    SchemaHistoryMalformed { reason: &'static str },

    /// A restore source / opened DB carries a `refinery_schema_history`
    /// (so it is refinery-versioned) but its `application_id` header does
    /// not match the wallet-storage magic — it is a foreign SQLite DB,
    /// not a wallet database. Rejected before it can be persisted over
    /// the live wallet DB or migrated in place.
    #[error(
        "not a platform-wallet-storage database: application_id {found:#010x} != expected {expected:#010x}"
    )]
    NotAWalletDb { expected: i32, found: i32 },

    /// A second [`SqlitePersister`](crate::SqlitePersister) `open()` on a
    /// path already open in THIS process. Each handle has its own
    /// `Mutex<Connection>` and write buffer, so buffered writes on one are
    /// invisible to the other — silent state divergence. Refused until the
    /// first persister drops.
    #[error("a SqlitePersister is already open on {} in this process", path.display())]
    AlreadyOpen { path: PathBuf },

    /// A value couldn't be cast to the database's native i64
    /// representation without losing magnitude.
    #[error("integer overflow casting `{field}` (value={value}) to {target}")]
    IntegerOverflow {
        field: &'static str,
        value: u64,
        target: SafeCastTarget,
    },

    /// Flush failed transiently (e.g. `SQLITE_BUSY` / `SQLITE_LOCKED`) for
    /// `wallet_id`. The buffered changeset is restored, so the next
    /// `flush(wallet_id)` retries it merged with anything stored in
    /// between. Use **exponential backoff** — tight-looping turns lock
    /// contention into a CPU spin that starves the lock holder.
    #[error(
        "FlushRetryable: flush failed transiently for wallet {}; buffer preserved for retry",
        hex::encode(wallet_id)
    )]
    FlushRetryable {
        wallet_id: [u8; 32],
        #[source]
        source: rusqlite::Error,
    },

    /// Rehydration's discovery probes don't mirror the real account's
    /// address pools 1:1 (`probes.len() != pools.len()`) — a structural
    /// invariant break, not user-reachable. Fail-closed rather than apply a
    /// probe's discovered depth to the wrong pool by position.
    #[error(
        "rehydration pool count mismatch: expected {expected} probe pool(s), found {found}"
    )]
    RehydrationPoolMismatch { expected: usize, found: usize },

    /// Rehydration's discovery probes mirror the real account's pools by
    /// count but not by chain identity at `position` — applying the
    /// probe's discovered depth here would misattribute derivation to the
    /// wrong pool.
    #[error(
        "rehydration pool type mismatch at position {position}: expected {expected:?}, found {found:?}"
    )]
    RehydrationPoolTypeMismatch {
        position: usize,
        expected: key_wallet::managed_account::address_pool::AddressPoolType,
        found: key_wallet::managed_account::address_pool::AddressPoolType,
    },
}

impl From<WalletStorageError> for PersistenceError {
    fn from(err: WalletStorageError) -> Self {
        match err {
            WalletStorageError::LockPoisoned => PersistenceError::LockPoisoned,
            other => {
                let kind = other.persistence_kind();
                PersistenceError::backend_with_kind(kind, other)
            }
        }
    }
}

impl WalletStorageError {
    /// Construct a `BlobDecode` error from a static reason. Used by schema
    /// modules on a structural decode error (wrong-length id, trailing
    /// bytes).
    pub(crate) fn blob_decode(reason: &'static str) -> Self {
        Self::BlobDecode { reason }
    }

    /// `true` when the failure is safe to retry — the caller should
    /// preserve in-flight state and call again. Transient codes are the
    /// recoverable environmental ones: `DatabaseBusy`/`DatabaseLocked`
    /// (contention), `DiskFull`, `SystemIoFailure`, `OutOfMemory`.
    ///
    /// The OUTER match is intentionally wildcard-free so a future variant
    /// forces explicit classification here; the INNER `ErrorCode` match
    /// needs a wildcard because that enum is upstream `#[non_exhaustive]`.
    pub fn is_transient(&self) -> bool {
        use rusqlite::ErrorCode;
        match self {
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _)) => matches!(
                e.code,
                ErrorCode::DatabaseBusy
                    | ErrorCode::DatabaseLocked
                    | ErrorCode::DiskFull
                    | ErrorCode::SystemIoFailure
                    | ErrorCode::OutOfMemory
            ),
            Self::FlushRetryable { .. } => true,
            // Every other rusqlite variant — non-`SqliteFailure` (e.g.
            // `ToSqlConversionFailure`, `InvalidColumnIndex`) — is a
            // logic bug, not a contention failure.
            Self::Sqlite(_) => false,
            Self::Io(_)
            | Self::Migration(_)
            | Self::IntegrityCheckFailed { .. }
            | Self::IntegrityCheckRunFailed { .. }
            | Self::SourceOpenFailed { .. }
            | Self::SchemaHistoryMissing
            | Self::SchemaVersionUnsupported { .. }
            | Self::AutoBackupDisabled { .. }
            | Self::AutoBackupDirUnwritable { .. }
            | Self::WalletNotFound { .. }
            | Self::WalletIdMismatch { .. }
            // TODO(qa): `LockPoisoned` fatal classification has no e2e
            // mutex-poison test; verified manually via
            // `tests/sqlite_error_classification`. Re-check
            // `handle_flush_error`'s fatal branch if you change it.
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
            | Self::ForeignKeysNotEnforced
            | Self::JournalModeNotApplied { .. }
            | Self::SchemaHistoryMalformed { .. }
            | Self::NotAWalletDb { .. }
            | Self::AlreadyOpen { .. }
            | Self::IdentityKeyEntryMismatch
            | Self::IdentityEntryIdMismatch
            | Self::AccountRegistrationEntryMismatch
            | Self::AccountRecordInvalid { .. }
            | Self::MissingAccount { .. }
            | Self::AccountRejected { .. }
            | Self::AssetLockEntryMismatch { .. }
            | Self::BlobTooLarge { .. }
            | Self::IntegerOverflow { .. }
            | Self::RehydrationPoolMismatch { .. }
            | Self::RehydrationPoolTypeMismatch { .. } => false,
        }
    }

    /// Trait-boundary classification for [`PersistenceError::Backend`]:
    ///
    /// - [`PersistenceErrorKind::Transient`] — [`Self::is_transient`] true; caller MAY retry.
    /// - [`PersistenceErrorKind::Constraint`] — SQL constraint/FK/CHECK violation; caller bug.
    /// - [`PersistenceErrorKind::Fatal`] — everything else.
    ///
    /// [`Self::LockPoisoned`] never reaches here; the `From` impl maps it
    /// straight to [`PersistenceError::LockPoisoned`].
    pub fn persistence_kind(&self) -> PersistenceErrorKind {
        use rusqlite::ErrorCode;
        if self.is_transient() {
            return PersistenceErrorKind::Transient;
        }
        match self {
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if matches!(e.code, ErrorCode::ConstraintViolation) =>
            {
                PersistenceErrorKind::Constraint
            }
            // A migration failure (`Self::Migration`) isn't a caller bug,
            // so it stays `Fatal` rather than `Constraint`.
            _ => PersistenceErrorKind::Fatal,
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
                ErrorCode::DiskFull => "sqlite_disk_full",
                ErrorCode::SystemIoFailure => "sqlite_io_failure",
                ErrorCode::OutOfMemory => "sqlite_out_of_memory",
                _ => "sqlite_other",
            },
            Self::Sqlite(_) => "sqlite_other",
            Self::FlushRetryable { .. } => "flush_retryable",
            Self::Io(_) => "io",
            Self::Migration(_) => "migration",
            Self::IntegrityCheckFailed { .. } => "integrity_check_failed",
            Self::IntegrityCheckRunFailed { .. } => "integrity_check_run_failed",
            Self::SourceOpenFailed { .. } => "source_open_failed",
            Self::SchemaHistoryMissing => "schema_history_missing",
            Self::SchemaVersionUnsupported { .. } => "schema_version_unsupported",
            Self::AutoBackupDisabled { .. } => "auto_backup_disabled",
            Self::AutoBackupDirUnwritable { .. } => "auto_backup_dir_unwritable",
            Self::WalletNotFound { .. } => "wallet_not_found",
            Self::WalletIdMismatch { .. } => "wallet_id_mismatch",
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
            Self::ForeignKeysNotEnforced => "foreign_keys_not_enforced",
            Self::JournalModeNotApplied { .. } => "journal_mode_not_applied",
            Self::SchemaHistoryMalformed { .. } => "schema_history_malformed",
            Self::NotAWalletDb { .. } => "not_a_wallet_db",
            Self::AlreadyOpen { .. } => "already_open",
            Self::IdentityKeyEntryMismatch => "identity_key_entry_mismatch",
            Self::IdentityEntryIdMismatch => "identity_entry_id_mismatch",
            Self::AccountRecordInvalid { .. } => "account_record_invalid",
            Self::MissingAccount { .. } => "missing_account_registration_entry",
            Self::AccountRejected { .. } => "account_rejected",
            Self::AccountRegistrationEntryMismatch => "account_registration_entry_mismatch",
            Self::AssetLockEntryMismatch { .. } => "asset_lock_entry_mismatch",
            Self::BlobTooLarge { .. } => "blob_too_large",
            Self::IntegerOverflow { .. } => "integer_overflow",
            Self::RehydrationPoolMismatch { .. } => "rehydration_pool_mismatch",
            Self::RehydrationPoolTypeMismatch { .. } => "rehydration_pool_type_mismatch",
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
