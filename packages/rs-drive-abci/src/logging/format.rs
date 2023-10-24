use serde::{Deserialize, Serialize};

/// Format of logs to use.
///
/// See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#formatters
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum LogFormat {
    /// Default, human-readable, single-line logs
    #[default]
    Full,
    /// A variant of the default formatter, optimized for short line lengths
    Compact,
    /// Pretty, multi-line logs, optimized for human readability
    Pretty,
    /// Outputs newline-delimited JSON logs, for machine processing
    Json,
}
