use clap::{Parser, ValueEnum};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::path::{Path, PathBuf};

/// Replay helper for Request* dumps.
#[derive(Debug, Parser)]
#[command(
    name = "replay_abci_requests",
    author,
    version,
    about = "Replay serialized ABCI requests against an existing GroveDB database.",
    long_about = "Feed captured Request* payloads (RON or JSON) or entire drive-abci JSON trace logs \
sequentially into the Drive ABCI application to recompute app hashes, inspect tx outcomes, and debug \
state mismatches. Request files accept both the outer Request wrapper or the specific request type, \
and configuration mirrors drive-abci's .env loading so you can point at the same RPC credentials. \
\n\nTo ingest log files enable trace level JSON logging first, e.g.:\n  dashmate config set platform.drive.abci.logs.file.format json\n  dashmate config set platform.drive.abci.logs.file.level trace \
\n\nExample:\n  RUST_LOG=trace replay_abci_requests --db-path /path/to/grovedb --requests dump.ron \
--config /path/to/.env --request-format ron\n\nUse multiple --requests flags to replay several inputs \
in chronological order."
)]
pub struct Cli {
    /// Path to the GroveDB database that should be used for execution.
    /// You can use a command like `./state_backup.sh export --component abci abci.tar.gz testnet`
    /// to dump the GroveDB database from existing Platform node.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub db_path: PathBuf,

    /// Files that contain serialized Request*, RequestPrepareProposal, or RequestProcessProposal payloads.
    /// They will be executed sequentially before any --logs entries.
    ///
    /// See vectors/ directory for example request payloads.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    pub requests: Vec<PathBuf>,

    /// drive-abci JSON logs that contain trace level "received ABCI request" entries.
    /// Relevant requests will be extracted and replayed chronologically.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    pub logs: Vec<PathBuf>,

    /// Optional .env file path. Defaults to walking up the filesystem like drive-abci.
    /// .env file format is the same as used by drive-abci.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    /// Format of the serialized request payload.
    #[arg(long, value_enum, default_value_t = RequestFormat::Ron)]
    pub request_format: RequestFormat,

    /// Print progress information (height + app hash) to stdout after each finalize_block.
    #[arg(short, long)]
    pub progress: bool,

    /// Write structured JSON logs to the provided file instead of stderr.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Skip replaying specific requests (use PATH for files or PATH:LINE for log entries).
    #[arg(long = "skip", value_name = "PATH[:LINE]", value_parser = parse_skip_request)]
    pub skip: Vec<SkipRequest>,

    /// Stop replay after reaching this block height (inclusive).
    #[arg(long, value_name = "HEIGHT")]
    pub stop_height: Option<u64>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum RequestFormat {
    Json,
    Ron,
}

#[derive(Debug, Clone)]
pub struct SkipRequest {
    pub path: PathBuf,
    pub line: Option<usize>,
}

pub fn parse_skip_request(raw: &str) -> Result<SkipRequest, String> {
    let (path_part, line_part) = match raw.rsplit_once(':') {
        Some((path, line_str)) => match line_str.parse::<usize>() {
            Ok(line) => (path, Some(line)),
            Err(_) => (raw, None),
        },
        None => (raw, None),
    };

    let path_buf = PathBuf::from(path_part);
    let canonical = path_buf
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path_part));

    Ok(SkipRequest {
        path: canonical,
        line: line_part,
    })
}

pub fn load_env(path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    if let Some(path) = path {
        dotenvy::from_path(path)?;
        return Ok(());
    }

    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(err) if err.not_found() => {
            tracing::warn!("warning: no .env file found");
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

pub fn parse_with<T>(raw: &str, format: RequestFormat) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    match format {
        RequestFormat::Json => Ok(serde_json::from_str(raw)?),
        RequestFormat::Ron => Ok(ron::from_str(raw)?),
    }
}
