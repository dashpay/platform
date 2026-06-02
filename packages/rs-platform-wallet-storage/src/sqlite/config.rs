//! Configuration for [`SqlitePersister`](crate::SqlitePersister).

use std::path::{Path, PathBuf};
use std::time::Duration;

/// When `store()` makes data durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlushMode {
    /// `store()` only buffers. Caller must call `flush()` (or
    /// `commit_writes()`) to make changes durable.
    Manual,
    /// `store()` flushes inline at the end of the call. Safest default.
    #[default]
    Immediate,
}

/// SQLite journal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JournalMode {
    #[default]
    Wal,
    Delete,
    Memory,
    Off,
    Truncate,
    Persist,
}

impl JournalMode {
    pub(crate) fn pragma_value(self) -> &'static str {
        match self {
            JournalMode::Wal => "WAL",
            JournalMode::Delete => "DELETE",
            JournalMode::Memory => "MEMORY",
            JournalMode::Off => "OFF",
            JournalMode::Truncate => "TRUNCATE",
            JournalMode::Persist => "PERSIST",
        }
    }
}

/// SQLite synchronous mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Synchronous {
    Off,
    #[default]
    Normal,
    Full,
    Extra,
}

impl Synchronous {
    pub(crate) fn pragma_value(self) -> &'static str {
        match self {
            Synchronous::Off => "OFF",
            Synchronous::Normal => "NORMAL",
            Synchronous::Full => "FULL",
            Synchronous::Extra => "EXTRA",
        }
    }
}

/// Persister configuration.
///
/// Defaults match the dash-evo-tool behaviour: `Immediate` flushes,
/// 5 s busy timeout, WAL journal, `NORMAL` synchronous, automatic
/// backups under `<db_dir>/backups/auto/`.
#[derive(Debug, Clone)]
pub struct SqlitePersisterConfig {
    pub path: PathBuf,
    pub flush_mode: FlushMode,
    pub busy_timeout: Duration,
    pub journal_mode: JournalMode,
    pub synchronous: Synchronous,
    /// Where automatic backups (pre-migration, pre-wallet-deletion) are
    /// written. Set to `None` to disable automatic backups — library
    /// API destructive operations then return
    /// [`WalletStorageError::AutoBackupDisabled`](crate::WalletStorageError::AutoBackupDisabled).
    pub auto_backup_dir: Option<PathBuf>,
}

impl SqlitePersisterConfig {
    /// Build a config with sensible defaults for the given DB path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let auto_backup_dir = default_auto_backup_dir(&path);
        Self {
            path,
            flush_mode: FlushMode::default(),
            busy_timeout: Duration::from_secs(5),
            journal_mode: JournalMode::default(),
            synchronous: Synchronous::default(),
            auto_backup_dir: Some(auto_backup_dir),
        }
    }

    /// Override flush mode.
    pub fn with_flush_mode(mut self, mode: FlushMode) -> Self {
        self.flush_mode = mode;
        self
    }

    /// Override auto-backup dir. Pass `None` to opt out.
    pub fn with_auto_backup_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.auto_backup_dir = dir;
        self
    }
}

/// `<db_dir>/backups/auto/` (or `./backups/auto/` if the DB path has no parent).
///
/// Public so the CLI binary (a separate compilation unit) can share the
/// same resolution as the library's `SqlitePersisterConfig::new`. The
/// preferred narrower visibility would be `pub(super)`, but `pub use`
/// re-exports up to the crate root cannot expose a `pub(super)` item.
pub fn default_auto_backup_dir(db_path: &Path) -> PathBuf {
    let parent = db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    parent.join("backups").join("auto")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let cfg = SqlitePersisterConfig::new("/tmp/w.db");
        assert_eq!(cfg.flush_mode, FlushMode::Immediate);
        assert_eq!(cfg.busy_timeout, Duration::from_secs(5));
        assert_eq!(cfg.journal_mode, JournalMode::Wal);
        assert_eq!(cfg.synchronous, Synchronous::Normal);
        assert_eq!(
            cfg.auto_backup_dir.as_deref(),
            Some(std::path::Path::new("/tmp/backups/auto"))
        );
    }
}
