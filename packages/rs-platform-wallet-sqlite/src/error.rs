//! Typed errors for `platform-wallet-sqlite`.
//!
//! Every variant maps onto `PersistenceError` at the trait boundary via
//! the [`From`] impl at the bottom of this file. The special-case
//! `LockPoisoned` mapping is preserved end-to-end so callers can still
//! pattern-match the trait-level variant.

use std::path::PathBuf;

use platform_wallet::changeset::PersistenceError;

/// Which destructive operation tried to take an automatic backup and
/// failed because the configuration had no backup directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AutoBackupOperation {
    #[error("open (pending migration)")]
    OpenMigration,
    #[error("delete_wallet")]
    DeleteWallet,
}

/// Errors produced by the SQLite-backed persister.
#[derive(Debug, thiserror::Error)]
pub enum SqlitePersisterError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),

    #[error("migration left the database in a dirty state (applied={applied} pending={pending})")]
    MigrationDirty { applied: usize, pending: usize },

    #[error("integrity check failed: {check_output}")]
    IntegrityCheckFailed { check_output: String },

    #[error("source backup is missing schema_history (not a platform-wallet-sqlite database)")]
    SchemaHistoryMissing,

    #[error("source backup schema version {found} is outside supported range {expected_range}")]
    SchemaVersionUnsupported { found: i64, expected_range: String },

    #[error("auto-backup is disabled for operation: {operation}")]
    AutoBackupDisabled { operation: AutoBackupOperation },

    #[error("auto-backup directory {} could not be prepared: {source}", dir.display())]
    AutoBackupDirUnwritable {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("wallet not found: {}", hex::encode(wallet_id))]
    WalletNotFound { wallet_id: [u8; 32] },

    #[error("persister lock poisoned")]
    LockPoisoned,

    #[error("restore destination is locked or in use")]
    RestoreDestinationLocked,

    #[error("invalid wallet id: {0}")]
    InvalidWalletId(String),

    #[error("invalid configuration: {0}")]
    ConfigInvalid(&'static str),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("backup destination already exists: {}", path.display())]
    BackupDestinationExists { path: PathBuf },
}

impl From<SqlitePersisterError> for PersistenceError {
    fn from(err: SqlitePersisterError) -> Self {
        match err {
            SqlitePersisterError::LockPoisoned => PersistenceError::LockPoisoned,
            other => PersistenceError::Backend(other.to_string()),
        }
    }
}

impl SqlitePersisterError {
    /// Helper for the bincode serialize/deserialize boundary.
    pub(crate) fn serialization(msg: impl std::fmt::Display) -> Self {
        Self::Serialization(msg.to_string())
    }
}
