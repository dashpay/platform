use crate::logging::error::Error;
use derive_more::Display;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use tracing_subscriber::EnvFilter;

/// Log level presets
#[derive(Serialize, Debug, Clone, Default, Display)]
#[serde(rename_all = "camelCase")]
pub enum LogLevelPreset {
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
impl From<&str> for LogLevelPreset {
    fn from(value: &str) -> Self {
        match value {
            "silent" => LogLevelPreset::Silent,
            "error" => LogLevelPreset::Error,
            "warn" => LogLevelPreset::Warn,
            "info" => LogLevelPreset::Info,
            "debug" => LogLevelPreset::Debug,
            "trace" => LogLevelPreset::Trace,
            "paranoid" => LogLevelPreset::Paranoid,
            configuration => LogLevelPreset::Custom(configuration.to_string()),
        }
    }
}

struct LogLevelPresetVisitor;

impl<'de> Visitor<'de> for LogLevelPresetVisitor {
    type Value = LogLevelPreset;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("silent, error, warn, info, debug, trace, paranoid or custom level using RUST_LOG format")
    }

    fn visit_str<E>(self, value: &str) -> Result<LogLevelPreset, E>
    where
        E: de::Error,
    {
        let level = LogLevelPreset::from(value);

        Ok(level)
    }
}

impl<'de> Deserialize<'de> for LogLevelPreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LogLevelPresetVisitor)
    }
}

/// Creates log level preset from verbosity level
impl TryFrom<u8> for LogLevelPreset {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, <LogLevelPreset as TryFrom<u8>>::Error> {
        let level = match value {
            0 => LogLevelPreset::Info,
            1 => LogLevelPreset::Debug,
            2 => LogLevelPreset::Trace,
            3 => LogLevelPreset::Paranoid,
            verbosity => return Err(Error::InvalidVerbosityLevel(verbosity)),
        };

        Ok(level)
    }
}

impl From<&LogLevelPreset> for EnvFilter {
    fn from(value: &LogLevelPreset) -> Self {
        match value {
            LogLevelPreset::Silent => EnvFilter::default(),
            LogLevelPreset::Custom(configuration) => EnvFilter::new(configuration),
            LogLevelPreset::Error => EnvFilter::new("error"),
            LogLevelPreset::Warn => {
                EnvFilter::new("error,tenderdash_abci=warn,drive_abci=warn,drive=warn,dpp=warn")
            }
            LogLevelPreset::Info => {
                EnvFilter::new("error,tenderdash_abci=info,drive_abci=info,drive=info,dpp=info")
            }
            LogLevelPreset::Debug => {
                EnvFilter::new("info,tenderdash_abci=debug,drive_abci=debug,drive=debug,dpp=debug")
            }
            LogLevelPreset::Trace => {
                EnvFilter::new("debug,tenderdash_abci=trace,drive_abci=trace,drive=trace,drive::grovedb_operations=debug,dpp=trace")
            }
            LogLevelPreset::Paranoid => EnvFilter::new("trace"),
        }
    }
}
