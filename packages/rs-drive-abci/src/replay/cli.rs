use clap::Args;
use std::path::PathBuf;

/// Replay ABCI requests captured from drive-abci logs.
#[derive(Debug, Args, Clone)]
#[command(about, long_about)]
pub struct ReplayArgs {
    /// Path to the GroveDB database that should be used for execution.
    /// Defaults to the path from drive-abci configuration.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub db_path: Option<PathBuf>,

    /// drive-abci JSON log that contains TRACE level "received ABCI request" entries.
    /// Relevant requests will be extracted and replayed chronologically. The answer the log
    /// has after a ProcessProposal request ("sending ABCI response") is read as well, and the
    /// replay stops where its own answer differs from it in status, app hash or a state
    /// transition's result code or gas_used. After a ProcessProposal request, lines of white
    /// space and log entries that are neither a request nor an answer ("sending ABCI
    /// exception" is an answer too) are passed over, and the first other line decides: an
    /// answer that names the thread ("threadName", "threadId") the request's line names is
    /// taken for the request's; a request, an answer of another thread or of none, or text
    /// that is no log entry leaves the request without an answer. A node logs a request and
    /// its answer on one thread, with no other request or answer of that thread between
    /// them; the log has to be of one node at a time. Elsewhere, log entries that are no
    /// requests and lines of text that are no log entries are ignored. A line that is not
    /// text stops the replay with an error.
    #[arg(long = "log", value_hint = clap::ValueHint::FilePath)]
    pub log: PathBuf,

    /// Log progress information at INFO level after each finalize_block.
    #[arg(short, long)]
    pub progress: bool,

    /// Skip replaying specific log entries by their line numbers (supports ranges and comma lists).
    #[arg(
        long = "skip",
        value_name = "LINE[-LINE]",
        value_delimiter = ',',
        value_parser = parse_skip_selector
    )]
    pub skip: Vec<SkipSelector>,

    /// Stop replay after reaching this block height (inclusive). A replay that reaches its
    /// stop height, or the end of its log, directly after a block that was executed and not
    /// committed ends with an error, since its database is not at that block: mainnet's
    /// block 32326 is one, as on the chain. A replay that is cancelled there says so in a
    /// warning.
    #[arg(long, value_name = "HEIGHT")]
    pub stop_height: Option<u64>,

    /// Keep Dash Core's answer to each request in this file while replaying beside a Core
    /// node, so that a later replay can run without one (`--core-answers`). Each distinct
    /// request is kept once, with the first answer Core gave to it that is not one of its
    /// errors for a node that is not ready (starting, not connected, loading blocks). A file
    /// that exists is
    /// added to. Core has to answer a request alike each time it is asked, so record beside a
    /// Core whose chain stands still (caught up, then `setnetworkactive false`); whether it
    /// does answer alike is checked and not assumed: a second, different answer to a recorded
    /// request (one of the errors for a node that is not ready aside) stops the replay, as
    /// does a method Core refuses with HTTP status 403, or an error of a request that is
    /// none of the connection (a reply with HTTP status 200 whose body is no JSON, for one).
    /// A request Core gives no answer to (an
    /// error of the connection, or one of the errors for a node that is not ready) is asked
    /// up to four times; the replay goes on with the first answer, and stops when none of the
    /// four is answered. The file may not be inside the database directory.
    #[arg(
        long,
        value_name = "FILE",
        value_hint = clap::ValueHint::FilePath,
        conflicts_with = "core_answers"
    )]
    pub core_answers_record: Option<PathBuf>,

    /// Answer every request to Dash Core from this file, recorded by `--core-answers-record`,
    /// and ask no Core node. A request the file has no answer to stops the replay. Both
    /// options need a build that unwinds on a panic, which the node's builds do.
    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub core_answers: Option<PathBuf>,

    /// Write one JSON line for each finalized block to this file: height, round, core height,
    /// protocol version, app hash, GroveDB root hash, every state transition's result code
    /// and gas_used (its storage and processing fee), whether those were held against the
    /// log's answer, and a hash over each whole answer. Two replays of the same log that
    /// answered every request alike and left the same root hashes write the same bytes. The
    /// fields taken from the block's proposal and from the answer to it are null unless the last request
    /// replayed before FinalizeBlock (Info, ExtendVote and VerifyVoteExtension requests
    /// aside) was the accepted ProcessProposal of that very block. A report is written from a
    /// new database on. For a database that has blocks the
    /// report has to be its own: one block after the other, ending at the block the database
    /// is at with its app hash. The file is locked while a replay writes it or is held
    /// against it, and may not be inside the database directory.
    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub report: Option<PathBuf>,

    /// Hold every finalized block against the line another replay wrote for it with
    /// `--report`, on another storage engine for one, and stop at the first block that
    /// differs, naming each field that does; its line is then not written. Lines an earlier
    /// run wrote to `--report` are held against that file first, and counted apart from the
    /// blocks this run executes. The other replay's line for a block is read before the
    /// first ProcessProposal of the block's height is executed, so a file that has none
    /// stops the replay before that, and before the block is finalized (a PrepareProposal
    /// of the height that comes earlier in the log has been executed by then); and where the
    /// log ends, the file has to end too, unless `--stop-height` was reached. The file is
    /// locked while a replay is held against it: one that another replay is still writing is
    /// refused.
    #[arg(
        long,
        value_name = "FILE",
        value_hint = clap::ValueHint::FilePath,
        requires = "report"
    )]
    pub compare_report: Option<PathBuf>,
}

/// Selector used by `--skip` flag to filter log line numbers.
#[derive(Debug, Clone, Copy)]
pub enum SkipSelector {
    Line(usize),
    Range { start: usize, end: usize },
}

impl SkipSelector {
    pub fn matches(&self, line: usize) -> bool {
        match self {
            SkipSelector::Line(target) => line == *target,
            SkipSelector::Range { start, end } => (*start..=*end).contains(&line),
        }
    }
}

pub fn parse_skip_selector(raw: &str) -> Result<SkipSelector, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("skip value cannot be empty".to_string());
    }

    if let Some((start, end)) = trimmed.split_once('-') {
        let start_line = parse_line_number(start)?;
        let end_line = parse_line_number(end)?;
        if start_line > end_line {
            return Err(format!(
                "invalid skip range {}-{} (start must be <= end)",
                start_line, end_line
            ));
        }
        return Ok(SkipSelector::Range {
            start: start_line,
            end: end_line,
        });
    }

    let line = parse_line_number(trimmed)?;
    Ok(SkipSelector::Line(line))
}

fn parse_line_number(raw: &str) -> Result<usize, String> {
    raw.trim().parse::<usize>().map_err(|_| {
        format!(
            "invalid skip target '{}'; expected valid line number",
            raw.trim()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SkipSelector::matches ----

    #[test]
    fn skip_selector_line_matches_exact() {
        let selector = SkipSelector::Line(42);
        assert!(selector.matches(42));
        assert!(!selector.matches(41));
        assert!(!selector.matches(43));
    }

    #[test]
    fn skip_selector_range_matches_inclusive() {
        let selector = SkipSelector::Range { start: 10, end: 20 };
        assert!(!selector.matches(9));
        assert!(selector.matches(10));
        assert!(selector.matches(15));
        assert!(selector.matches(20));
        assert!(!selector.matches(21));
    }

    #[test]
    fn skip_selector_range_single_element() {
        let selector = SkipSelector::Range { start: 5, end: 5 };
        assert!(selector.matches(5));
        assert!(!selector.matches(4));
        assert!(!selector.matches(6));
    }

    // ---- parse_skip_selector ----

    #[test]
    fn parse_skip_selector_single_line() {
        let result = parse_skip_selector("42").unwrap();
        assert!(matches!(result, SkipSelector::Line(42)));
    }

    #[test]
    fn parse_skip_selector_range() {
        let result = parse_skip_selector("10-20").unwrap();
        match result {
            SkipSelector::Range { start, end } => {
                assert_eq!(start, 10);
                assert_eq!(end, 20);
            }
            _ => panic!("expected Range"),
        }
    }

    #[test]
    fn parse_skip_selector_with_whitespace() {
        let result = parse_skip_selector("  42  ").unwrap();
        assert!(matches!(result, SkipSelector::Line(42)));
    }

    #[test]
    fn parse_skip_selector_empty_error() {
        let err = parse_skip_selector("").unwrap_err();
        assert!(err.contains("cannot be empty"));
    }

    #[test]
    fn parse_skip_selector_whitespace_only_error() {
        let err = parse_skip_selector("   ").unwrap_err();
        assert!(err.contains("cannot be empty"));
    }

    #[test]
    fn parse_skip_selector_inverted_range_error() {
        let err = parse_skip_selector("20-10").unwrap_err();
        assert!(err.contains("start must be <= end"));
    }

    #[test]
    fn parse_skip_selector_non_numeric_error() {
        let err = parse_skip_selector("abc").unwrap_err();
        assert!(err.contains("invalid skip target"));
    }

    #[test]
    fn parse_skip_selector_range_non_numeric_start() {
        let err = parse_skip_selector("abc-10").unwrap_err();
        assert!(err.contains("invalid skip target"));
    }

    #[test]
    fn parse_skip_selector_range_non_numeric_end() {
        let err = parse_skip_selector("10-xyz").unwrap_err();
        assert!(err.contains("invalid skip target"));
    }

    #[test]
    fn parse_skip_selector_equal_range() {
        let result = parse_skip_selector("5-5").unwrap();
        match result {
            SkipSelector::Range { start, end } => {
                assert_eq!(start, 5);
                assert_eq!(end, 5);
            }
            _ => panic!("expected Range"),
        }
    }
}
