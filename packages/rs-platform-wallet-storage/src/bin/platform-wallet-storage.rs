//! CLI front-end for the SQLite persister.
//!
//! Output convention: stdout = data; stderr = diagnostics + error
//! messages (lower-cased, no trailing period, single line).

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use platform_wallet_storage::{
    default_auto_backup_dir, AutoBackupOperation, RetentionPolicy, SqlitePersister,
    SqlitePersisterConfig, WalletStorageError,
};

#[derive(Debug, Parser)]
#[command(
    name = "platform-wallet-storage",
    version,
    about = "Maintenance CLI for the SQLite-backed platform wallet persister"
)]
struct Cli {
    /// Path to the SQLite database file. Required by `migrate`,
    /// `backup`, and `restore`; ignored by `prune` (which operates
    /// purely on the backups directory).
    #[arg(long, value_name = "PATH", global = true)]
    db: Option<PathBuf>,
    /// Auto-backup directory. To disable auto-backup, pass the
    /// subcommand flag `--no-auto-backup` (supported by `migrate` and
    /// `restore`).
    #[arg(long, value_name = "PATH", global = true)]
    auto_backup_dir: Option<PathBuf>,
    /// Increase log verbosity (stderr). Repeat for more: `-v` enables
    /// `info`, `-vv` enables `debug`, `-vvv` enables `trace`.
    #[arg(long, short, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    /// Suppress non-error stderr output (overrides `--verbose`).
    #[arg(long, short, global = true)]
    quiet: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run migrations only (auto-backs-up by default).
    Migrate(MigrateArgs),
    /// Online backup to a timestamped `.db` file (or explicit path).
    Backup(BackupArgs),
    /// Replace --db with the contents of a backup.
    Restore(RestoreArgs),
    /// Apply retention to a backup directory.
    Prune(PruneArgs),
}

#[derive(Debug, Args)]
struct MigrateArgs {
    #[arg(long)]
    no_auto_backup: bool,
}

#[derive(Debug, Args)]
struct BackupArgs {
    /// Output directory OR full file path.
    #[arg(long, value_name = "PATH")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct RestoreArgs {
    #[arg(long, value_name = "PATH")]
    from: PathBuf,
    #[arg(long)]
    yes: bool,
    /// Skip the pre-restore auto-backup of the live destination DB.
    /// Without this, the persister writes `pre-restore-<db>-<ts>.db` to
    /// `--auto-backup-dir` before clobbering the destination.
    #[arg(long)]
    no_auto_backup: bool,
}

#[derive(Debug, Args)]
struct PruneArgs {
    #[arg(long = "in", value_name = "DIR")]
    in_dir: PathBuf,
    #[arg(long)]
    keep_last: Option<usize>,
    #[arg(long, value_parser = parse_duration)]
    max_age: Option<Duration>,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| format!("invalid duration `{s}`: {e}"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.quiet);
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {}", err.message);
            err.code
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool) {
    use tracing_subscriber::EnvFilter;
    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("platform_wallet_storage={level}")));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

struct CliError {
    message: String,
    code: ExitCode,
}

impl CliError {
    fn runtime(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            code: ExitCode::from(1),
        }
    }
    fn usage(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            code: ExitCode::from(2),
        }
    }
    fn validation(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            code: ExitCode::from(3),
        }
    }
}

/// Render `err` and its full `#[source]` chain, joined by `": "`.
///
/// `WalletStorageError`'s `Display` is deliberately terse for variants that
/// keep their detail in `#[source]` (`error.rs:1-4`) — "sqlite error",
/// "migration error", "cannot open candidate source database" and friends
/// carry nothing an operator can act on by themselves. The CLI is the one
/// place all of that detail needs to reach a human, so every path here
/// walks the chain back out instead of stopping at the head.
fn chain_message(err: &dyn Error) -> String {
    let mut out = err.to_string();
    let mut cur = err.source();
    while let Some(source) = cur {
        out.push_str(": ");
        out.push_str(&source.to_string());
        cur = source.source();
    }
    out
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let auto_backup_dir: Option<PathBuf> = cli.auto_backup_dir;

    // `prune` is a pure filesystem op; `--db` is meaningless, so handle it
    // before requiring `cli.db`.
    if let Cmd::Prune(args) = &cli.cmd {
        return run_prune(args);
    }

    let db = cli.db.ok_or_else(|| CliError::usage("--db is required"))?;

    // `restore` is an associated function; no persister needed beforehand.
    if let Cmd::Restore(args) = &cli.cmd {
        return run_restore(&db, args, auto_backup_dir.as_deref());
    }

    // `migrate --no-auto-backup` clears `auto_backup_dir` so the open-time
    // pre-migration backup is skipped; other subcommands keep the default.
    let mut config = SqlitePersisterConfig::new(&db);
    if let Some(dir) = auto_backup_dir.clone() {
        config = config.with_auto_backup_dir(Some(dir));
    }
    if let Cmd::Migrate(m) = &cli.cmd {
        if m.no_auto_backup {
            config = config.with_auto_backup_dir(None);
            eprintln!("warning: auto-backup skipped (--no-auto-backup)");
        }
    }

    // Migrate is done by `open`; capture pre/post versions to print
    // "applied: N". A read failure must surface, not be read as 0.
    if let Cmd::Migrate(_) = &cli.cmd {
        let pre_version =
            peek_schema_version(&db).map_err(|e| CliError::runtime(chain_message(&e)))?;
        let _persister = SqlitePersister::open(config.clone()).map_err(map_open_err_for_cli)?;
        let post_version =
            peek_schema_version(&db).map_err(|e| CliError::runtime(chain_message(&e)))?;
        let applied = post_version
            .unwrap_or(0)
            .saturating_sub(pre_version.unwrap_or(0)) as usize;
        println!("applied: {applied}");
        return Ok(ExitCode::SUCCESS);
    }

    match cli.cmd {
        Cmd::Migrate(_) | Cmd::Prune(_) | Cmd::Restore(_) => unreachable!(),
        Cmd::Backup(args) => {
            let persister = SqlitePersister::open(config).map_err(map_open_err_for_cli)?;
            run_backup(&persister, args)
        }
    }
}

fn map_open_err_for_cli(err: WalletStorageError) -> CliError {
    match err {
        WalletStorageError::AutoBackupDisabled {
            operation: AutoBackupOperation::OpenMigration,
        } => CliError {
            message: "auto-backup directory not configured; pass --no-auto-backup to proceed"
                .to_string(),
            code: ExitCode::from(1),
        },
        WalletStorageError::Io(e) => {
            CliError::runtime(format!("failed to open database: {}", chain_message(&e)))
        }
        other => CliError::runtime(chain_message(&other)),
    }
}

/// Read the highest applied migration version. `Ok(None)` means the
/// DB has no `refinery_schema_history` row yet (fresh DB); a real open
/// or query failure is propagated as `Err` so callers don't mistake a
/// transient failure for "version 0".
fn peek_schema_version(db: &Path) -> Result<Option<i64>, rusqlite::Error> {
    use rusqlite::{OpenFlags, OptionalExtension};
    // A missing path is a normal fresh `migrate`: `Ok(None)` lets
    // `SqlitePersister::open` create the file under the 0o600 invariant,
    // instead of materialising a stub here that bypasses it.
    if !db.exists() {
        return Ok(None);
    }
    // READ-ONLY, URI parsing off (matches the open-conn choke-point) so a
    // `--db` path can't smuggle `file:` query params defeating read-only.
    let conn = rusqlite::Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // Pre-migration the history table may legitimately not exist.
    let has_history = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !has_history {
        return Ok(None);
    }
    let v = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?
        .flatten();
    Ok(v)
}

fn run_backup(persister: &SqlitePersister, args: BackupArgs) -> Result<ExitCode, CliError> {
    // `backup_to` owns refuse-to-overwrite (`BackupDestinationExists`).
    let path = persister.backup_to(&args.out).map_err(|e| match e {
        WalletStorageError::BackupDestinationExists { path } => CliError::runtime(format!(
            "backup destination exists and refuses to overwrite: {}",
            path.display()
        )),
        other => CliError::runtime(chain_message(&other)),
    })?;
    println!("{}", path.display());
    Ok(ExitCode::SUCCESS)
}

fn run_restore(
    db: &Path,
    args: &RestoreArgs,
    auto_backup_dir: Option<&Path>,
) -> Result<ExitCode, CliError> {
    if !args.yes {
        return Err(CliError::usage("refusing to restore without --yes"));
    }
    let result = if args.no_auto_backup {
        eprintln!("warning: auto-backup skipped (--no-auto-backup)");
        SqlitePersister::restore_from_skip_backup(db, &args.from)
    } else {
        // No persister is opened here, so compute the config default
        // (`<db_dir>/backups/auto/`) inline.
        let resolved_dir: PathBuf = match auto_backup_dir {
            None => default_auto_backup_dir(db),
            Some(p) => p.to_path_buf(),
        };
        SqlitePersister::restore_from(db, &args.from, Some(&resolved_dir))
    };
    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(WalletStorageError::IntegrityCheckFailed { report }) => Err(CliError::validation(
            format!("source backup failed integrity check: {report}"),
        )),
        Err(err @ WalletStorageError::IntegrityCheckRunFailed { .. }) => {
            Err(CliError::validation(chain_message(&err)))
        }
        Err(WalletStorageError::SchemaHistoryMissing) => Err(CliError::validation(
            "source backup schema history missing".to_string(),
        )),
        Err(
            err @ (WalletStorageError::NotAWalletDb { .. }
            | WalletStorageError::SchemaVersionUnsupported { .. }
            | WalletStorageError::SchemaHistoryMalformed { .. }
            | WalletStorageError::SourceOpenFailed { .. }),
        ) => Err(CliError::validation(chain_message(&err))),
        Err(WalletStorageError::AutoBackupDisabled { .. }) => Err(CliError::runtime(
            "auto-backup directory not configured; pass --no-auto-backup to proceed",
        )),
        Err(other) => Err(CliError::runtime(chain_message(&other))),
    }
}

fn run_prune(args: &PruneArgs) -> Result<ExitCode, CliError> {
    if args.keep_last.is_none() && args.max_age.is_none() {
        return Err(CliError::usage(
            "at least one of --keep-last or --max-age is required",
        ));
    }
    let policy = RetentionPolicy {
        keep_last_n: args.keep_last,
        max_age: args.max_age,
    };
    let report = platform_wallet_storage::sqlite::backup::prune(&args.in_dir, policy)
        .map_err(|e| CliError::runtime(chain_message(&e)))?;
    for p in &report.removed {
        println!("{}", p.display());
    }
    for (p, e) in &report.failed_removals {
        eprintln!("warning: failed to remove {}: {e}", p.display());
    }
    // Non-zero exit when any per-file removal failed so scripts can
    // detect the partial-success case.
    if report.failed_removals.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `chain_message` must not stop at the head of the chain — that is
    /// the whole point of RUST-004: `WalletStorageError`'s `Display` is
    /// terse by design, so nothing this CLI prints can rely on `to_string`.
    #[test]
    fn chain_message_joins_the_whole_source_chain() {
        #[derive(Debug)]
        struct Leaf;
        impl std::fmt::Display for Leaf {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "leaf cause")
            }
        }
        impl std::error::Error for Leaf {}

        #[derive(Debug)]
        struct Mid(Leaf);
        impl std::fmt::Display for Mid {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "mid layer")
            }
        }
        impl std::error::Error for Mid {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        assert_eq!(chain_message(&Mid(Leaf)), "mid layer: leaf cause");
    }

    /// End-to-end through a real crate error: `WalletStorageError::Io`'s
    /// `Display` is the bare word "io error" (`error.rs:38`); the operator
    /// only learns anything from the wrapped `io::Error`.
    #[test]
    fn chain_message_surfaces_the_io_error_wrapped_by_wallet_storage_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let err = WalletStorageError::Io(io_err);
        assert_eq!(chain_message(&err), "io error: permission denied");
    }

    /// `map_open_err_for_cli`'s `Io` special case must still route through
    /// `chain_message`, not a bare `{e}` that happens to work only because
    /// `io::Error` rarely nests further.
    #[test]
    fn map_open_err_for_cli_io_variant_keeps_the_inner_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let cli_err = map_open_err_for_cli(WalletStorageError::Io(io_err));
        assert_eq!(
            cli_err.message,
            "failed to open database: permission denied"
        );
    }

    /// `peek_schema_version` on a missing path must not materialise a stub
    /// file (opening READ-ONLY) that would lack the 0o600 invariant.
    #[test]
    fn peek_schema_version_on_missing_db_does_not_create_stub() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let typo = tmp.path().join("absent.db");
        assert!(!typo.exists(), "precondition: path must not exist");

        let v = peek_schema_version(&typo).expect("must succeed with Ok(None)");
        assert_eq!(v, None);

        assert!(
            !typo.exists(),
            "peek_schema_version silently created a stub at {}",
            typo.display()
        );
    }
}
