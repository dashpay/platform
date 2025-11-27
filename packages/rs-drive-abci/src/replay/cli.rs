use clap::Args;
use std::path::{Path, PathBuf};

/// Replay ABCI requests captured from drive-abci logs.
#[derive(Debug, Args, Clone)]
#[command(about, long_about)]
pub struct ReplayArgs {
    /// Path to the GroveDB database that should be used for execution.
    /// Defaults to the path from drive-abci configuration.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub db_path: Option<PathBuf>,

    /// drive-abci JSON logs that contain TRACE level "received ABCI request" entries.
    /// Relevant requests will be extracted and replayed chronologically.
    /// Other log entries are ignored.
    #[arg(long = "log", value_hint = clap::ValueHint::FilePath)]
    pub logs: Vec<PathBuf>,

    /// Log progress information at INFO level after each finalize_block.
    #[arg(short, long)]
    pub progress: bool,

    /// Skip replaying specific requests (use PATH for files or PATH:LINE for log entries).
    #[arg(long = "skip", value_name = "PATH[:LINE]", value_parser = parse_skip_request)]
    pub skip: Vec<SkipRequest>,

    /// Stop replay after reaching this block height (inclusive).
    #[arg(long, value_name = "HEIGHT")]
    pub stop_height: Option<u64>,
}

/// Request selector used by `--skip` flag.
#[derive(Debug, Clone)]
pub struct SkipRequest {
    /// Canonicalized log path.
    pub path: PathBuf,
    /// Optional line number to match entries within the log.
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

/// Check if a log path and optional line match the skip selector.
pub fn skip_matches(path: &Path, line: Option<usize>, needle: &SkipRequest) -> bool {
    if !paths_equal(path, &needle.path) {
        return false;
    }

    match (line, needle.line) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual == expected,
        _ => false,
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if let (Ok(canon_a), Ok(canon_b)) = (a.canonicalize(), b.canonicalize()) {
        canon_a == canon_b
    } else {
        a == b
    }
}
