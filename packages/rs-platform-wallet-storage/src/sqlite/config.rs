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

/// How `load()` reacts to a recoverable inconsistency in persisted rows.
///
/// The two policies are not symmetric: `Strict` is the safe default and
/// `Recovery` is a diagnostic escape hatch that reproduces the historical
/// best-effort behaviour verbatim. `Recovery` never tolerates anything
/// `Strict` would not also have reached — an oversize blob, an unusable
/// schema version, or a failed `PRAGMA integrity_check` still hard-error.
///
/// # Open-time gates are unconditional
///
/// `open()` runs migrations, and migrating a structurally corrupt file
/// amplifies the damage, so the integrity check, schema-version gate,
/// foreign-key gate, schema-history probe, and wallet-identity check stay
/// hard in both policies. SQLite-level corruption reaching the decoders
/// yields arbitrary garbage rows that `Recovery` would then tolerate and
/// count, inverting the point of the feature. A database failing
/// `integrity_check` needs
/// [`restore_from`](crate::SqlitePersister::restore_from) or
/// `sqlite3 .recover`, not recovery mode.
///
/// # Examples
///
/// ```rust
/// use platform_wallet_storage::{LoadPolicy, SqlitePersisterConfig};
///
/// let config = SqlitePersisterConfig::new("/tmp/wallets.db")
///     .with_load_policy(LoadPolicy::Recovery);
/// assert_eq!(config.load_policy, LoadPolicy::Recovery);
/// ```
// TODO(recovery-mode): no FFI entry point constructs SqlitePersister today; when
// one is added, plumb SqlitePersisterConfig::with_load_policy and expose
// last_load_degradation across the boundary.
// TODO(recovery-mode): Recovery has no human-facing surface. The maintenance
// CLI deliberately has no --recovery flag — none of its subcommands call
// load(), so the flag would be a no-op that additionally blocked migrate and
// prune. A `verify` subcommand (open Recovery, load, print the per-site
// counts, exit non-zero when degraded) is the shape that would fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadPolicy {
    /// Any inconsistency aborts the load. A corrupted wallet is never
    /// handed to the caller half-formed.
    #[default]
    Strict,
    /// Best-effort load: tolerable inconsistencies are logged and counted
    /// instead of returned. The persister is **read-only** — every write
    /// entry point returns
    /// [`WalletStorageError::ReadOnlyRecoveryMode`](crate::WalletStorageError::ReadOnlyRecoveryMode)
    /// so a degraded projection can never be written back over good rows.
    /// See [`SqlitePersister::last_load_degradation`](crate::SqlitePersister::last_load_degradation).
    /// Diagnostic / rescue only.
    Recovery,
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
///
/// `Normal` (the default, paired with WAL) is **app-crash durable**: a
/// committed write survives a process crash but NOT a power loss / OS
/// crash mid-checkpoint, where the last transactions in the WAL can be
/// lost. Choose `Full` for power-loss durability at the cost of an fsync
/// per commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Synchronous {
    Off,
    /// WAL default: durable across application crash, not power loss.
    #[default]
    Normal,
    /// fsync on every commit: durable across power loss / OS crash.
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
    /// How `load()` reacts to a recoverable inconsistency. Defaults to
    /// [`LoadPolicy::Strict`] — safety must not depend on the caller
    /// passing a flag, including on the struct-literal construction path.
    pub load_policy: LoadPolicy,
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
            load_policy: LoadPolicy::default(),
        }
    }

    /// Override flush mode.
    pub fn with_flush_mode(mut self, mode: FlushMode) -> Self {
        self.flush_mode = mode;
        self
    }

    /// Override the load policy. [`LoadPolicy::Recovery`] additionally
    /// makes the persister read-only and requires `auto_backup_dir` to be
    /// set.
    pub fn with_load_policy(mut self, policy: LoadPolicy) -> Self {
        self.load_policy = policy;
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
/// Public so the CLI binary (a separate compilation unit) shares the same
/// resolution as `SqlitePersisterConfig::new`.
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
        assert_eq!(cfg.load_policy, LoadPolicy::Strict);
        assert_eq!(
            cfg.auto_backup_dir.as_deref(),
            Some(std::path::Path::new("/tmp/backups/auto"))
        );
    }
}
