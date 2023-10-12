use std::path::PathBuf;

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
}
