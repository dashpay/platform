//! CLI front-end for the SQLite persister.
//!
//! Output convention: stdout = data; stderr = diagnostics + error
//! messages (lower-cased, no trailing period, single line).

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
    /// Path to the SQLite database file.
    #[arg(long, value_name = "PATH", global = true)]
    db: Option<PathBuf>,
    /// Auto-backup directory. The empty-string ("") form is
    /// **deprecated** as a way to disable auto-backup — use the
    /// subcommand flag `--no-auto-backup` instead (supported by
    /// `migrate` and `restore`). The empty-string form still parses for
    /// one release; a deprecation warning is logged when used.
    #[arg(long, value_name = "PATH", global = true)]
    auto_backup_dir: Option<String>,
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
    /// Dump per-table row counts.
    Inspect(InspectArgs),
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
    /// Without this, the persister writes `pre-restore-<ts>.db` to
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

#[derive(Debug, Args)]
struct InspectArgs {
    #[arg(long)]
    wallet_id: Option<String>,
    #[arg(long, default_value = "text")]
    format: InspectFormat,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum InspectFormat {
    Text,
    Tsv,
    Json,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| format!("invalid duration `{s}`: {e}"))
}

fn parse_wallet_id(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 {
        return Err(format!(
            "wallet id must be 64 hex characters, got {} (`{}`)",
            s.len(),
            s
        ));
    }
    let bytes = hex::decode(s).map_err(|e| format!("wallet id is not valid hex: {e}"))?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
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
    fn validation(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            code: ExitCode::from(3),
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let db = cli
        .db
        .ok_or_else(|| CliError::runtime("--db is required"))?;
    let auto_backup_dir = match cli.auto_backup_dir {
        None => None,
        Some(s) if s.is_empty() => {
            // CODE-030: empty-string sentinel for "disable auto-backup"
            // is deprecated in favour of the subcommand flag
            // `--no-auto-backup`. Keep parsing it for one release so
            // existing operators don't break overnight, but emit a
            // loud deprecation warning on stderr.
            eprintln!(
                "warning: `--auto-backup-dir \"\"` to disable auto-backup is deprecated; \
                 pass `--no-auto-backup` to the subcommand instead"
            );
            Some(None)
        }
        Some(s) => Some(Some(PathBuf::from(s))),
    };

    // For `prune`, we don't open a persister — pure filesystem op.
    if let Cmd::Prune(args) = &cli.cmd {
        return run_prune(args);
    }

    // `restore` is an associated function; no persister needed beforehand.
    if let Cmd::Restore(args) = &cli.cmd {
        return run_restore(&db, args, auto_backup_dir.as_ref());
    }

    // For `migrate --no-auto-backup`, we must keep `auto_backup_dir =
    // None` so the open-time pre-migration backup is skipped. For
    // every other subcommand we leave the user-configured dir (or the
    // default) in place — the library's safe-by-default semantics
    // still apply.
    let mut config = SqlitePersisterConfig::new(&db);
    if let Some(dir_opt) = auto_backup_dir.clone() {
        config = config.with_auto_backup_dir(dir_opt);
    }
    if let Cmd::Migrate(m) = &cli.cmd {
        if matches!(&auto_backup_dir, Some(None)) && !m.no_auto_backup {
            return Err(CliError {
                message: "auto-backup directory not configured; pass --no-auto-backup to proceed"
                    .to_string(),
                code: ExitCode::from(1),
            });
        }
        if m.no_auto_backup {
            config = config.with_auto_backup_dir(None);
            eprintln!("warning: auto-backup skipped (--no-auto-backup)");
        }
    }

    // Migrate (idempotent): open performs it. We capture the prior
    // schema version so we can print "applied: N". A transient read
    // failure must surface — silently reading 0 would print a wrong
    // `applied:` count.
    if let Cmd::Migrate(_) = &cli.cmd {
        let pre_version = peek_schema_version(&db).map_err(|e| CliError::runtime(e.to_string()))?;
        let _persister = SqlitePersister::open(config.clone()).map_err(map_open_err_for_cli)?;
        let post_version =
            peek_schema_version(&db).map_err(|e| CliError::runtime(e.to_string()))?;
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
        Cmd::Inspect(args) => {
            let persister = SqlitePersister::open(config).map_err(map_open_err_for_cli)?;
            run_inspect(&persister, args)
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
        WalletStorageError::Io(e) => CliError::runtime(format!("failed to open database: {e}")),
        other => CliError::runtime(other.to_string()),
    }
}

/// Read the highest applied migration version. `Ok(None)` means the
/// DB has no `refinery_schema_history` row yet (fresh DB); a real open
/// or query failure is propagated as `Err` so callers don't mistake a
/// transient failure for "version 0".
fn peek_schema_version(db: &Path) -> Result<Option<i64>, rusqlite::Error> {
    use rusqlite::OptionalExtension;
    let conn = rusqlite::Connection::open(db)?;
    // Pre-migration the history table may not exist yet — that is a
    // legitimate "no version" answer, not a failure.
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
    // `backup_to` is the single authority on refuse-to-overwrite — it
    // returns `BackupDestinationExists` for a pre-existing file path.
    let path = persister.backup_to(&args.out).map_err(|e| match e {
        WalletStorageError::BackupDestinationExists { path } => CliError::runtime(format!(
            "backup destination exists and refuses to overwrite: {}",
            path.display()
        )),
        other => CliError::runtime(other.to_string()),
    })?;
    println!("{}", path.display());
    Ok(ExitCode::SUCCESS)
}

fn run_restore(
    db: &Path,
    args: &RestoreArgs,
    auto_backup_dir: Option<&Option<PathBuf>>,
) -> Result<ExitCode, CliError> {
    if !args.yes {
        return Err(CliError {
            message: "refusing to restore without --yes".into(),
            code: ExitCode::from(2),
        });
    }
    let result = if args.no_auto_backup {
        eprintln!("warning: auto-backup skipped (--no-auto-backup)");
        SqlitePersister::restore_from_skip_backup(db, &args.from)
    } else {
        // CLI default mirrors the persister config default
        // (`<db_dir>/backups/auto/`). The CLI doesn't open a
        // persister here, so we compute the default inline.
        let resolved_dir: Option<PathBuf> = match auto_backup_dir {
            None => Some(default_auto_backup_dir(db)),
            Some(opt) => opt.clone(),
        };
        SqlitePersister::restore_from(db, &args.from, resolved_dir.as_deref())
    };
    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(WalletStorageError::IntegrityCheckFailed { report }) => Err(CliError::validation(
            format!("source backup failed integrity check: {report}"),
        )),
        Err(WalletStorageError::SchemaHistoryMissing) => Err(CliError::validation(
            "source backup failed integrity check: schema history missing".to_string(),
        )),
        Err(WalletStorageError::AutoBackupDisabled { .. }) => Err(CliError::runtime(
            "auto-backup directory not configured; pass --no-auto-backup to proceed",
        )),
        Err(other) => Err(CliError::runtime(other.to_string())),
    }
}

fn run_prune(args: &PruneArgs) -> Result<ExitCode, CliError> {
    if args.keep_last.is_none() && args.max_age.is_none() {
        return Err(CliError {
            message: "at least one of --keep-last or --max-age is required".into(),
            code: ExitCode::from(2),
        });
    }
    let policy = RetentionPolicy {
        keep_last_n: args.keep_last,
        max_age: args.max_age,
    };
    let report = platform_wallet_storage::sqlite::backup::prune(&args.in_dir, policy)
        .map_err(|e| CliError::runtime(e.to_string()))?;
    for p in &report.removed {
        println!("{}", p.display());
    }
    for (p, e) in &report.failed_removals {
        eprintln!("warning: failed to remove {}: {e}", p.display());
    }
    // ATOM-011: non-zero exit when any per-file removal failed so
    // scripts can detect the partial-success case.
    if report.failed_removals.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn run_inspect(persister: &SqlitePersister, args: InspectArgs) -> Result<ExitCode, CliError> {
    let wallet_id = match args.wallet_id.as_deref() {
        None => None,
        Some(s) => Some(parse_wallet_id(s).map_err(|m| CliError {
            message: m,
            code: ExitCode::from(2),
        })?),
    };
    let counts = persister
        .inspect_counts(wallet_id.as_ref())
        .map_err(|e| CliError::runtime(e.to_string()))?;
    match args.format {
        InspectFormat::Text | InspectFormat::Tsv => {
            for (table, n) in counts {
                println!("{table}\t{n}");
            }
        }
        InspectFormat::Json => {
            let entries: Vec<serde_json::Value> = counts
                .into_iter()
                .map(|(table, n)| match &wallet_id {
                    None => serde_json::json!({ "table": table, "count": n }),
                    Some(id) => serde_json::json!({
                        "table": table,
                        "count": n,
                        "wallet_id": hex::encode(id),
                    }),
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string(&entries).map_err(|e| CliError::runtime(e.to_string()))?
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}
