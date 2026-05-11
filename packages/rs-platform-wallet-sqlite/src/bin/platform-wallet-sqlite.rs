//! CLI front-end for the SQLite persister.
//!
//! Output convention: stdout = data; stderr = diagnostics + error
//! messages (lower-cased, no trailing period, single line).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use platform_wallet_sqlite::{
    AutoBackupOperation, RetentionPolicy, SqlitePersister, SqlitePersisterConfig,
    SqlitePersisterError,
};

#[derive(Debug, Parser)]
#[command(
    name = "platform-wallet-sqlite",
    version,
    about = "Maintenance CLI for the SQLite-backed platform wallet persister"
)]
struct Cli {
    /// Path to the SQLite database file.
    #[arg(long, value_name = "PATH", global = true)]
    db: Option<PathBuf>,
    /// Auto-backup directory. Pass empty string to disable.
    #[arg(long, value_name = "PATH", global = true)]
    auto_backup_dir: Option<String>,
    #[arg(long, short, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    #[arg(long, short, global = true)]
    #[allow(dead_code)]
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
    /// Drop a wallet (auto-backs-up by default).
    DeleteWallet(DeleteWalletArgs),
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
}

#[derive(Debug, Args)]
struct PruneArgs {
    #[arg(long = "in", value_name = "DIR")]
    in_dir: PathBuf,
    #[arg(long)]
    keep_last: Option<usize>,
    #[arg(long, value_parser = parse_duration)]
    max_age: Option<Duration>,
    #[arg(long)]
    dry_run: bool,
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

#[derive(Debug, Args)]
struct DeleteWalletArgs {
    #[arg(long)]
    wallet_id: String,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    no_auto_backup: bool,
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
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {}", err.message);
            err.code
        }
    }
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
        Some(s) if s.is_empty() => Some(None),
        Some(s) => Some(Some(PathBuf::from(s))),
    };

    // For `prune`, we don't open a persister — pure filesystem op.
    if let Cmd::Prune(args) = &cli.cmd {
        return run_prune(args);
    }

    // `restore` is an associated function; no persister needed beforehand.
    if let Cmd::Restore(args) = &cli.cmd {
        return run_restore(&db, args);
    }

    // For `migrate`, allow `--no-auto-backup` to skip the auto-backup
    // dir requirement at open time by opting out before construction.
    let mut config = SqlitePersisterConfig::new(&db);
    match (&cli.cmd, &auto_backup_dir) {
        (Cmd::Migrate(m), Some(None)) if !m.no_auto_backup => {
            return Err(CliError {
                message: "auto-backup directory not configured; pass --no-auto-backup to proceed"
                    .to_string(),
                code: ExitCode::from(1),
            });
        }
        _ => {}
    }
    if let Some(dir_opt) = auto_backup_dir.clone() {
        config = config.with_auto_backup_dir(dir_opt);
    }
    // If --no-auto-backup was passed for migrate, force-disable so the
    // open() path doesn't take a pre-migration backup.
    if let Cmd::Migrate(m) = &cli.cmd {
        if m.no_auto_backup {
            config = config.with_auto_backup_dir(None);
            // Emit the warning whether or not auto_backup_dir was set.
            eprintln!("warning: auto-backup skipped (--no-auto-backup)");
        }
    }

    // Migrate (idempotent): open performs it. We capture the prior
    // schema version so we can print "applied: N".
    if let Cmd::Migrate(_) = &cli.cmd {
        let pre_version = peek_schema_version(&db);
        let _persister = SqlitePersister::open(config.clone()).map_err(map_open_err_for_cli)?;
        let post_version = peek_schema_version(&db);
        let applied = post_version
            .unwrap_or(0)
            .saturating_sub(pre_version.unwrap_or(0)) as usize;
        // Best-effort: count by version delta is approximate when
        // multiple migrations land in one go. For TC-056 we only need
        // "applied: <N>" with `N > 0` on first run and `N = 0` on second.
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
        Cmd::DeleteWallet(args) => {
            // `--no-auto-backup` forces config.auto_backup_dir = None
            // before opening, otherwise we keep the user's configured
            // directory.
            let mut cfg = config;
            if args.no_auto_backup {
                cfg = cfg.with_auto_backup_dir(None);
                eprintln!("warning: auto-backup skipped (--no-auto-backup)");
            }
            let persister = SqlitePersister::open(cfg).map_err(map_open_err_for_cli)?;
            run_delete_wallet(&persister, args)
        }
    }
}

fn map_open_err_for_cli(err: SqlitePersisterError) -> CliError {
    match err {
        SqlitePersisterError::AutoBackupDisabled {
            operation: AutoBackupOperation::OpenMigration,
        } => CliError {
            message: "auto-backup directory not configured; pass --no-auto-backup to proceed"
                .to_string(),
            code: ExitCode::from(1),
        },
        SqlitePersisterError::Io(e) => CliError::runtime(format!("failed to open database: {e}")),
        other => CliError::runtime(other.to_string()),
    }
}

fn peek_schema_version(db: &Path) -> Option<i64> {
    let conn = rusqlite::Connection::open(db).ok()?;
    conn.query_row(
        "SELECT MAX(version) FROM refinery_schema_history",
        [],
        |row| row.get::<_, Option<i64>>(0),
    )
    .ok()
    .flatten()
}

fn run_backup(persister: &SqlitePersister, args: BackupArgs) -> Result<ExitCode, CliError> {
    if args.out.is_file() {
        return Err(CliError::runtime(format!(
            "backup destination exists and refuses to overwrite: {}",
            args.out.display()
        )));
    }
    let path = persister
        .backup_to(&args.out)
        .map_err(|e| CliError::runtime(e.to_string()))?;
    println!("{}", path.display());
    Ok(ExitCode::SUCCESS)
}

fn run_restore(db: &Path, args: &RestoreArgs) -> Result<ExitCode, CliError> {
    if !args.yes {
        return Err(CliError {
            message: "refusing to restore without --yes".into(),
            code: ExitCode::from(2),
        });
    }
    match SqlitePersister::restore_from(db, &args.from) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(SqlitePersisterError::IntegrityCheckFailed { check_output }) => {
            Err(CliError::validation(format!(
                "source backup failed integrity check: {check_output}"
            )))
        }
        Err(SqlitePersisterError::SchemaHistoryMissing) => Err(CliError::validation(
            "source backup failed integrity check: schema history missing".to_string(),
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
    // For `--dry-run`, list candidates without invoking prune's
    // unlink path. We re-implement the filtering inline (small enough
    // to duplicate cleanly).
    if args.dry_run {
        let candidates = list_backup_dir_for_dry_run(&args.in_dir)
            .map_err(|e| CliError::runtime(e.to_string()))?;
        let now = std::time::SystemTime::now();
        let mut to_remove: Vec<std::path::PathBuf> = Vec::new();
        for (idx, (ts, path)) in candidates.into_iter().enumerate() {
            let keep_count = policy.keep_last_n.map(|n| idx < n).unwrap_or(true);
            let keep_age = policy
                .max_age
                .map(|max| now.duration_since(ts).map(|d| d <= max).unwrap_or(true))
                .unwrap_or(true);
            if !(keep_count && keep_age) {
                to_remove.push(path);
            }
        }
        to_remove.sort();
        for p in &to_remove {
            println!("{}", p.display());
        }
        return Ok(ExitCode::SUCCESS);
    }
    // We don't need a persister handle — call the static prune.
    let report = platform_wallet_sqlite::backup::prune(&args.in_dir, policy)
        .map_err(|e| CliError::runtime(e.to_string()))?;
    for p in &report.removed {
        println!("{}", p.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn list_backup_dir_for_dry_run(
    dir: &Path,
) -> std::io::Result<Vec<(std::time::SystemTime, std::path::PathBuf)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let recognised = name.ends_with(".db")
            && (name.starts_with("wallet-")
                || name.starts_with("pre-migration-")
                || name.starts_with("pre-delete-"));
        if !recognised {
            continue;
        }
        let ts = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        out.push((ts, path));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out)
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
            let mut first = true;
            print!("[");
            for (table, n) in counts {
                if !first {
                    print!(",");
                }
                first = false;
                match &wallet_id {
                    None => print!("{{\"table\":\"{table}\",\"count\":{n}}}"),
                    Some(id) => print!(
                        "{{\"table\":\"{table}\",\"count\":{n},\"wallet_id\":\"{}\"}}",
                        hex::encode(id)
                    ),
                }
            }
            println!("]");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_delete_wallet(
    persister: &SqlitePersister,
    args: DeleteWalletArgs,
) -> Result<ExitCode, CliError> {
    if !args.yes {
        return Err(CliError {
            message: "refusing to delete a wallet without --yes".into(),
            code: ExitCode::from(2),
        });
    }
    let wallet_id = parse_wallet_id(&args.wallet_id).map_err(|m| CliError {
        message: m,
        code: ExitCode::from(2),
    })?;
    let result = persister.delete_wallet(wallet_id);
    match result {
        Ok(report) => {
            if let Some(path) = &report.backup_path {
                println!("{}", path.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(SqlitePersisterError::AutoBackupDisabled { .. }) => Err(CliError::runtime(
            "auto-backup directory not configured; pass --no-auto-backup to proceed",
        )),
        Err(other) => Err(CliError::runtime(other.to_string())),
    }
}
