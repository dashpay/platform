use crate::logging::error::Error;
use derive_more::Display;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use tracing_subscriber::EnvFilter;

/// Log level presets
#[derive(Serialize, Debug, Clone, Default, Display)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    /// No logs
    Silent,
    /// Uses RUST_LOG format to set verbosity level
    Custom(String),
    /// Only errors
    Error,
    /// Warnings and errors. Errors for 3rd party dependencies
    Warn,
    /// Info level and lower. Warnings for 3rd party dependencies
    #[default]
    Info,
    /// Debug level and lower. Info level for 3rd party dependencies
    Debug,
    /// Trace level and lower. Debug level for 3rd party dependencies
    Trace,
    /// Trace level for everything
    Paranoid,
}

/// Creates log level preset from string
impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value {
            "silent" => LogLevel::Silent,
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            "paranoid" => LogLevel::Paranoid,
            configuration => LogLevel::Custom(configuration.to_string()),
        }
    }
}

struct LogLevelPresetVisitor;

impl<'de> Visitor<'de> for LogLevelPresetVisitor {
    type Value = LogLevel;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("silent, error, warn, info, debug, trace, paranoid or custom level using RUST_LOG format")
    }

    fn visit_str<E>(self, value: &str) -> Result<LogLevel, E>
    where
        E: de::Error,
    {
        let level = LogLevel::from(value);

        Ok(level)
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LogLevelPresetVisitor)
    }
}

/// Creates log level preset from verbosity level
impl TryFrom<u8> for LogLevel {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, <LogLevel as TryFrom<u8>>::Error> {
        let level = match value {
            0 => LogLevel::Info,
            1 => LogLevel::Debug,
            2 => LogLevel::Trace,
            3 => LogLevel::Paranoid,
            verbosity => return Err(Error::InvalidVerbosityLevel(verbosity)),
        };

        Ok(level)
    }
}

impl TryFrom<&LogLevel> for EnvFilter {
    type Error = Error;

    fn try_from(value: &LogLevel) -> Result<Self, Error> {
        let env_filter = match value {
            LogLevel::Silent => EnvFilter::default(),
            LogLevel::Custom(configuration) => {
                EnvFilter::try_new(configuration).map_err(Error::InvalidLogSpecification)?
            }
            LogLevel::Error => {
                EnvFilter::try_new("error").expect("should be a valid log specification")
            }
            LogLevel::Warn => {
                EnvFilter::try_new("error,tenderdash_abci=warn,drive_abci=warn,drive=warn,dpp=warn")
                    .expect("should be a valid log specification")
            }
            LogLevel::Info => {
                EnvFilter::try_new("error,tenderdash_abci=info,drive_abci=info,drive=info,dpp=info")
                    .expect("should be a valid log specification")
            }
            LogLevel::Debug => EnvFilter::try_new(
                "info,tenderdash_abci=debug,drive_abci=debug,drive=debug,dpp=debug",
            )
            .expect("should be a valid log specification"),
            LogLevel::Trace => EnvFilter::try_new(
                "debug,tenderdash_abci=trace,drive_abci=trace,drive=trace,dpp=trace",
            )
            .expect("should be a valid log specification"),
            LogLevel::Paranoid => {
                EnvFilter::try_new("trace").expect("should be a valid log specification")
            }
        };

        Ok(env_filter)
    }
}
