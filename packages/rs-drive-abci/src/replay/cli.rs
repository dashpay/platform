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
    /// Relevant requests will be extracted and replayed chronologically.
    /// Other log entries are ignored.
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

    /// Stop replay after reaching this block height (inclusive).
    #[arg(long, value_name = "HEIGHT")]
    pub stop_height: Option<u64>,
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
