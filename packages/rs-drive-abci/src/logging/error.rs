use std::path::PathBuf;
use tracing_subscriber::filter::ParseError;
use tracing_subscriber::util::TryInitError;

/// Errors returned by logging subsystem
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// File rotation error
    #[error("file rotation: {0}")]
    FileRotate(std::io::Error),

    /// File creation error
    #[error("create file {0}: {1}")]
    FileCreate(PathBuf, std::io::Error),

    /// Log file path is invalid
    #[error("log file path {0}: {1}")]
    FilePath(PathBuf, String),

    /// Duplicate config
    #[error("duplicate log configuration name {0}")]
    DuplicateConfigName(String),

    /// Undefined verbosity level
    #[error("undefined log verbosity level {0}")]
    InvalidVerbosityLevel(u8),

    /// Failed to parse log specification string
    #[error("invalid log specification {0}")]
    InvalidLogSpecification(ParseError),

    /// Failed to initialize logging
    #[error("failed to initialize logging {0}")]
    TryInitError(TryInitError),
}
