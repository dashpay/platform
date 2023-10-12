use crate::config::FromEnv;
use crate::logging::level::LogLevelPreset;
use crate::logging::{LogDestination, LogFormat};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Logging configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LogConfig {
    /// Destination of logs.
    pub destination: LogDestination,
    /// Log level
    #[serde(default)]
    pub level: LogLevelPreset,
    /// Whether or not to use colorful output; defaults to autodetect
    #[serde(default)]
    pub color: Option<bool>,
    /// Output format to use.
    #[serde(default)]
    pub format: LogFormat,
    /// Max number of daily files to store, excluding active one; only used when storing logs in file; defaults to 0 - rotation disabled
    #[serde(default)]
    pub max_files: usize,
}

/// Configuration of log destinations.
///
/// Logs can be sent to multiple destinations. Configuration of each of them is prefixed with `ABCI_LOG_<key>_`,
/// where `<key>` is some arbitrary alphanumeric name of log configuaration.
///
/// Key must match pattern `[A-Za-z0-9]+`.
///
/// Note that parsing of LogConfigs is implemented in [PlatformConfig::from_env()] due to limitations of [envy] crate.
///
/// ## Example
///
/// ```bash
/// # First logger, logging to stderr on verbosity level 5
/// ABCI_LOG_STDERR_DESTINATION=stderr
/// ABCI_LOG_STDERR_LEVEL=trace
///
/// # Second logger, logging to stdout on verbosity level 1
/// ABCI_LOG_STDOUT_DESTINATION=stdout
/// ABCI_LOG_STDOUT_LEVEL=info
/// ```
///
///
/// [PlatformConfig::from_env()]: crate::config::PlatformConfig::from_env()
pub type LogConfigs = HashMap<String, LogConfig>;

impl FromEnv for LogConfigs {
    /// create new object using values from environment variables
    fn from_env() -> Result<Self, crate::error::Error>
    where
        Self: Sized,
    {
        let re: Regex = Regex::new(r"^ABCI_LOG_([0-9a-zA-Z]+)_DESTINATION$").unwrap();
        let keys = std::env::vars().filter_map(|(key, _)| {
            re.captures(&key)
                .and_then(|capt| capt.get(1))
                .map(|m| m.as_str().to_string())
        });

        let mut configs: HashMap<String, LogConfig> = HashMap::new();
        for key in keys {
            let config: LogConfig = envy::prefixed(format! {"ABCI_LOG_{}_", key.as_str()})
                .from_env()
                .map_err(crate::error::Error::from)?;

            configs.insert(key.as_str().to_string(), config);
        }

        Ok(configs)
    }
}
