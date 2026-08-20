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
//! dedicated variant; everything else flows through
//! `PersistenceError::Backend { kind, source }` — `kind` is classified
//! by [`WalletStorageError::persistence_kind`] (Transient / Constraint /
//! Fatal) and `source` carries the boxed typed error so consumers can
//! walk `Error::source()` to the underlying `rusqlite` payload.

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
    /// called with an id that has no matching `wallet_metadata` row.
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

    /// Two different identities claimed one wallet's derivation slot.
    /// `identity_index` is an HD path component, so `(wallet_id,
    /// identity_index)` names exactly one identity; a second claim is a
    /// contradiction rather than a competition, and is refused instead
    /// of orphaning the displaced identity's keys at the next load.
    #[error(
        "identity index conflict: index {identity_index} of wallet {} is held by identity {}, cannot assign it to {}",
        hex::encode(wallet_id),
        hex::encode(existing),
        hex::encode(incoming)
    )]
    IdentityIndexConflict {
        wallet_id: [u8; 32],
        identity_index: u32,
        existing: [u8; 32],
        incoming: [u8; 32],
    },

    /// A wallet-less identity carried a derivation index. Out-of-wallet
    /// identities are keyed by identity id alone and have no derivation
    /// context, so an index on one is state that can never be honoured.
    #[error(
        "wallet-less identity {} carries derivation index {identity_index}",
        hex::encode(identity_id)
    )]
    WalletlessIdentityIndex {
        identity_id: [u8; 32],
        identity_index: u32,
    },

    /// An `asset_locks` row's typed-column `(outpoint, account_index)`
    /// disagreed with the lifecycle blob's `(out_point, account_index)`.
    /// Mirrors `IdentityKeyEntryMismatch` — a torn write, partial
    /// migration, or restored corruption that survives the per-row
    /// `integrity_check` is still rejected at decode time rather than
    /// mis-bucketing the lock under the wrong account.
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

    /// A blob payload exceeded the configured allocation cap during
    /// decode. Surfaced separately from generic [`Self::BlobDecode`] so
    /// operators can distinguish a hostile or corrupted oversize blob
    /// from a structural decode failure. Defaults to 16 MiB — well
    /// above any legitimate per-row payload.
    #[error("blob exceeded decode size limit ({len_bytes} bytes > {limit_bytes} byte cap)")]
    BlobTooLarge {
        len_bytes: usize,
        limit_bytes: usize,
    },

    /// An unspent UTXO named an address absent from
    /// `core_derived_addresses`, so its owning account index can't be
    /// resolved. Persisting it would mis-file live funds under account
    /// 0 with no path back to the real account, so the write is refused.
    /// Spent-only placeholder rows tolerate a missing mapping (they're
    /// excluded from the unspent set) and do not raise this.
    #[error("unspent utxo address {address} is not in core_derived_addresses")]
    UtxoAddressNotDerived { address: String },

    /// `PRAGMA foreign_keys = ON` was issued on open but the read-back
    /// reported the constraint enforcement is still off — the linked
    /// SQLite build silently ignores the pragma (no FK support compiled
    /// in). Hard-error at open rather than letting orphan rows accrue.
    #[error("SQLite foreign-key enforcement could not be enabled on this connection")]
    ForeignKeysNotEnforced,

    /// A value couldn't be cast to the database's native i64
    /// representation without losing magnitude.
    #[error("integer overflow casting `{field}` (value={value}) to {target}")]
    IntegerOverflow {
        field: &'static str,
        value: u64,
        target: SafeCastTarget,
    },

    /// Flush failed transiently (e.g. `SQLITE_BUSY` / `SQLITE_LOCKED`)
    /// for `wallet_id`. The buffered changeset has been restored — the
    /// next `flush(wallet_id)` will retry the same data merged with
    /// anything stored in between. Callers should back off and retry
    /// rather than dropping state.
    ///
    /// **Use exponential backoff; do NOT tight-loop on this error** —
    /// hammering the persister at full speed turns a transient lock
    /// contention into a hot CPU spin and delays whoever holds the
    /// lock from releasing it.
    ///
    /// The variant name `FlushRetryable` is intentionally embedded in
    /// the `Display` output so operators grepping production logs can
    /// match on the variant directly.
    #[error(
        "FlushRetryable: flush failed transiently for wallet {}; buffer preserved for retry",
        hex::encode(wallet_id)
    )]
    FlushRetryable {
        wallet_id: [u8; 32],
        #[source]
        source: rusqlite::Error,
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
    /// Construct a typed `BlobDecode` error from a static reason.
    /// Used by schema modules that hit a structural decode error
    /// (e.g. a 32-byte id column with the wrong length, or trailing
    /// bytes after a payload).
    pub(crate) fn blob_decode(reason: &'static str) -> Self {
        Self::BlobDecode { reason }
    }

    /// `true` when the underlying failure is safe to retry — the
    /// caller should preserve in-flight state and call again.
    /// Transient codes:
    /// - `DatabaseBusy` / `DatabaseLocked`: contention.
    /// - `DiskFull`: operator clears disk space.
    /// - `SystemIoFailure`: kernel-level I/O blip (NFS, raid rebuild).
    /// - `OutOfMemory`: transient memory pressure.
    ///
    /// All four classes are recoverable environmental conditions —
    /// dropping buffered state on them would be data loss for a
    /// problem the operator (or kernel) clears on its own.
    ///
    /// The OUTER match on `WalletStorageError` is intentionally
    /// wildcard-free: the enum MUST NOT gain `#[non_exhaustive]` so a
    /// future variant forces the author to classify it here. The
    /// INNER match on `rusqlite::ErrorCode` uses a wildcard because
    /// `ErrorCode` is `#[non_exhaustive]` upstream.
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
            // TODO(qa): `LockPoisoned` is classified as fatal here, but
            // the end-to-end mutex-poison flow has no automated test (a
            // panicking thread + join is hard to reproduce
            // deterministically). Manual verification only via the
            // table-driven test in `tests/sqlite_error_classification`.
            // If you change this classification, re-derive
            // `handle_flush_error`'s fatal-branch behavior to match.
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
            | Self::IdentityKeyEntryMismatch
            | Self::IdentityEntryIdMismatch
            | Self::IdentityIndexConflict { .. }
            | Self::WalletlessIdentityIndex { .. }
            | Self::AssetLockEntryMismatch { .. }
            | Self::BlobTooLarge { .. }
            | Self::UtxoAddressNotDerived { .. }
            | Self::IntegerOverflow { .. } => false,
        }
    }

    /// Trait-boundary classification for the
    /// [`PersistenceError::Backend`] kind field. Three classes:
    ///
    /// - [`PersistenceErrorKind::Transient`] — every variant where
    ///   [`Self::is_transient`] is `true`. Caller MAY retry.
    /// - [`PersistenceErrorKind::Constraint`] — SQL constraint /
    ///   FK / NOT NULL / UNIQUE / PK / CHECK violations. Schema /
    ///   integrity failure; caller bug, not infra.
    /// - [`PersistenceErrorKind::Fatal`] — everything else.
    ///
    /// [`Self::LockPoisoned`] is handled by the `From` impl directly
    /// (it maps to [`PersistenceError::LockPoisoned`] rather than
    /// flowing through `Backend`).
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
            // Uniqueness of `(wallet_id, identity_index)` is enforced in
            // Rust, not by a SQL constraint, so it has to be classified
            // here by hand — it is a caller-data violation all the same.
            Self::IdentityIndexConflict { .. } | Self::WalletlessIdentityIndex { .. } => {
                PersistenceErrorKind::Constraint
            }
            // Refinery surfaces FK / constraint problems through
            // rusqlite; if that path leaks through here the typed
            // variant lives in `Self::Migration`, which we leave as
            // `Fatal` since a migration failure isn't a caller bug.
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
            Self::IdentityKeyEntryMismatch => "identity_key_entry_mismatch",
            Self::IdentityEntryIdMismatch => "identity_entry_id_mismatch",
            Self::IdentityIndexConflict { .. } => "identity_index_conflict",
            Self::WalletlessIdentityIndex { .. } => "walletless_identity_index",
            Self::AssetLockEntryMismatch { .. } => "asset_lock_entry_mismatch",
            Self::BlobTooLarge { .. } => "blob_too_large",
            Self::UtxoAddressNotDerived { .. } => "utxo_address_not_derived",
            Self::IntegerOverflow { .. } => "integer_overflow",
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
