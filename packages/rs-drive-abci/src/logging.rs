use file_rotate::suffix::AppendTimestamp;
use file_rotate::suffix::FileLimit;
use file_rotate::ContentLimit;
use file_rotate::FileRotate;
use file_rotate::TimeFrequency;
use itertools::Itertools;
use lazy_static::__Deref;
use regex::Regex;
use reopen::Reopen;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::prelude::OpenOptionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::metadata::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::registry;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::util::TryInitError;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

use crate::config::FromEnv;

const LOG_DESTINATION_STDOUT: &str = "stdout";
const LOG_DESTINATION_STDERR: &str = "stderr";
#[cfg(test)]
const LOG_DESTINATION_BYTES: &str = "bytes";

/// Logging configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogConfig {
    /// Destination of logs.
    ///
    /// One of:
    /// * "stdout",
    /// * "stderr",
    /// * absolute path to log file
    ///
    /// For testing, also "bytes" is available.
    pub destination: String,
    /// Log level: silent, error, info, warn, debug, trace
    #[serde(default)]
    pub level: LogLevelPreset,
    /// Whether or not to use colorful output; defaults to autodetect
    #[serde(default)]
    pub color: Option<bool>,
    /// Output format to use.
    ///
    /// One of:
    /// * full
    /// * compact
    /// * pretty
    /// * json
    ///
    /// See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/format/index.html#formatters for more
    /// detailed description.
    #[serde(default)]
    pub format: LogFormat,
    /// Max number of daily files to store, excluding active one; only used when storing logs in file; defaults to 0 - rotation disabled
    #[serde(default)]
    pub max_files: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            destination: LOG_DESTINATION_STDOUT.to_string(),
            level: Default::default(),
            color: Default::default(),
            format: Default::default(),
            max_files: Default::default(),
        }
    }
}

/// Log level presets
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum LogLevelPreset {
    /// No logs
    Silent,
    /// Uses RUST_LOG env or info level if not set
    Custom,
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

/// Creates log level preset from verbosity level
impl TryFrom<u8> for LogLevelPreset {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, <LogLevelPreset as TryFrom<u8>>::Error> {
        let level = match value {
            0 => LogLevelPreset::Custom,
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
            LogLevelPreset::Custom => EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
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
                EnvFilter::new("debug,tenderdash_abci=trace,drive_abci=trace,drive=trace,dpp=trace")
            }
            LogLevelPreset::Paranoid => EnvFilter::new("trace"),
        }
    }
}

/// Format of logs to use.
///
/// See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#formatters
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum LogFormat {
    /// Default, human-readable, single-line logs
    #[default]
    Full,
    ///  A variant of the default formatter, optimized for short line lengths
    Compact,
    /// Pretty, multi-line logs, optimized for human readability
    Pretty,
    /// Outputs newline-delimited JSON logs, for machine processing
    Json,
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

/// Errors returned by logging subsystem
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// File rotation error
    #[error("file rotation: {0}")]
    FileRotate(std::io::Error),

    /// File creation error
    #[error("create file {0}: {1}")]
    FileCreate(PathBuf, std::io::Error),

    /// Invalid destination
    #[error(
        "invalid destination {0}: must be one of: stderr, stdout, or absolute path to a directory"
    )]
    InvalidDestination(String),

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

/// Name of logging configuration
pub type LoggerID = String;

/// LogBuilder is a builder for configuring and initializing logging subsystem.
///
/// # Examples
///
/// ```
/// use drive_abci::logging::LogBuilder;
/// use drive_abci::logging::LogConfigs;
/// use drive_abci::logging::LogConfig;
///
/// // Create a new LogBuilder instance
/// let mut log_builder = LogBuilder::new();
///
/// // Define your LogConfigs
/// let mut log_configs = LogConfigs::new();
/// log_configs.insert("config1".to_string(), LogConfig::default());
///
/// // Add all configs to the LogBuilder
/// log_builder = log_builder.with_configs(&log_configs).unwrap();
///
/// // Add an individual config to the LogBuilder
/// let config2 = LogConfig::default();
/// log_builder = log_builder.with_config("config2", &config2).unwrap();
///
/// // Build the logging subsystem
/// let loggers = log_builder.build();
///
/// // Install logging subsystem handler
/// loggers.install();
/// ```
#[derive(Default)]
pub struct LogBuilder {
    loggers: HashMap<LoggerID, Logger>,
}

impl LogBuilder {
    /// Creates a new `LogBuilder` instance with default settings.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds multiple logging configurations to the `LogBuilder` at once.
    ///
    /// # Examples
    ///
    /// ```
    /// use drive_abci::logging::LogBuilder;
    /// use drive_abci::logging::LogConfigs;
    ///
    /// let mut log_builder = LogBuilder::new();
    /// let mut log_configs = LogConfigs::new();
    ///
    /// // Add configurations to log_configs
    ///
    /// log_builder = log_builder.with_configs(&log_configs).unwrap();
    /// ```
    pub fn with_configs(self, configs: &LogConfigs) -> Result<Self, Error> {
        let mut me = self;
        for (name, config) in configs {
            me.add(name, config)?;
        }
        Ok(me)
    }

    /// Adds a single logging configuration to the `LogBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// use drive_abci::logging::LogBuilder;
    /// use drive_abci::logging::LogConfig;
    ///
    /// let log_builder = LogBuilder::new();
    /// let config = LogConfig::default();
    ///
    /// let log_builder = log_builder.with_config("config_name", &config).unwrap();
    /// ```
    pub fn with_config(self, configuration_name: &str, config: &LogConfig) -> Result<Self, Error> {
        let mut me = self;
        me.add(configuration_name, config)?;
        Ok(me)
    }

    /// Adds a new logger to the `LogBuilder`.
    fn add(&mut self, configuration_name: &str, config: &LogConfig) -> Result<(), Error> {
        let logger = Logger::try_from(config)?;
        if self.loggers.contains_key(configuration_name) {
            return Err(Error::DuplicateConfigName(configuration_name.to_string()));
        }
        self.loggers.insert(configuration_name.to_string(), logger);
        Ok(())
    }

    /// Finalizes the build process and constructs loggers collection.
    ///
    /// This method is called after configuring the builder with all desired settings. It consumes
    /// the builder and returns the constructed object.
    ///
    /// # Panics
    pub fn build(self) -> Loggers {
        Loggers(self.loggers)
    }
}

/// Collection of loggers defined using [LogBuilder].
///
/// This struct holds a collection of loggers created using the [LogBuilder].
/// It provides methods for installing, flushing, and rotating logs.
pub struct Loggers(HashMap<LoggerID, Logger>);

impl Loggers {
    /// Installs loggers as a global tracing handler.
    ///
    /// Installs loggers prepared in the [LogBuilder] as a global tracing handler. It must be called exactly once.
    /// Panics if a global tracing handler was already defined.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use drive_abci::logging::{LogBuilder, Loggers};
    ///
    /// // Create logger(s) using LogBuilder
    /// let mut logger_builder = LogBuilder::new();
    /// // Configure logger_builder using its methods
    ///
    /// // Build Loggers instance
    /// let loggers: Loggers = logger_builder.build();
    ///
    /// // Install loggers as a global tracing handler
    /// loggers.install();
    /// ```
    ///
    /// # Panics
    ///
    /// This method panics if the logging subsystem is already initialized.
    pub fn install(&self) {
        if let Err(e) = self.try_install() {
            panic!("Logging subsystem is already initialized: {}", e)
        }
    }

    /// Installs loggers prepared in the [LogBuilder] as a global tracing handler.
    ///
    /// Same as [Loggers::install()], but returns error if the logging subsystem is already initialized.
    ///
    /// # Example
    ///
    /// The following code can be used in tests. It ignores errors, as tests might actually call it more than once,
    /// and we don't want to panic in this case.
    ///
    /// ```
    /// drive_abci::logging::Loggers::default().try_install().ok();
    /// ```
    pub fn try_install(&self) -> Result<(), TryInitError> {
        // Based on examples from https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/layer/index.html
        let loggers = self.0.values().map(|l| Box::new(l.layer())).collect_vec();

        registry().with(loggers).try_init()
    }

    /// Flushes all loggers.
    ///
    /// In case of multiple errors, returns only the last one.
    ///
    /// # Errors
    ///
    /// Returns an error if there's an issue flushing any of the loggers.
    pub fn flush(&self) -> Result<(), std::io::Error> {
        let mut result = Ok(());
        for logger in self.0.values() {
            if let Err(e) = logger
                .destination
                .clone()
                .lock()
                .expect("logging lock poisoned")
                .to_write()
                .flush()
            {
                result = Err(e);
            };
        }
        result
    }

    /// Triggers log rotation for log destinations that support this.
    ///
    /// In case of multiple errors, returns the error from the last logger.
    ///
    /// # Errors
    ///
    /// Returns an error if there's an issue rotating any of the logs.
    pub fn rotate(&self) -> Result<(), Error> {
        let mut result: Result<(), Error> = Ok(());

        for logger in self.0.values() {
            let cloned = logger.destination.clone();
            let guard = cloned.lock().expect("logging lock poisoned");

            match guard.deref() {
                LogDestination::RotationWriter(ref writer) => {
                    let mut inner = writer.0.lock().expect("logging lock poisoned");
                    if let Err(e) = inner.rotate() {
                        result = Err(Error::FileRotate(e));
                    };
                }
                LogDestination::File(ref f) => {
                    let mut inner = f.0.lock().expect("logging lock poisoned");

                    result = inner.flush().map_err(Error::FileRotate).map(|_| {
                        inner.handle().reopen();
                    })
                }
                _ => {}
            }
        }
        result
    }
}

impl Default for Loggers {
    /// Default loggers that are just printing human-readable logs, based on `RUST_LOG` env variable.
    ///
    /// Useful for tests.
    ///
    /// # Panics
    ///
    /// Panics in (a very unlikely) event when logger builder fails to add new logger.
    ///
    /// # Example
    ///
    /// ```
    /// use drive_abci::logging::Loggers;
    ///
    /// Loggers::default().try_install().ok();
    /// ```
    fn default() -> Self {
        let mut logger_builder = LogBuilder::new();
        logger_builder
            .add("default", &LogConfig::default())
            .expect("cannot configure default logger");
        logger_builder.build()
    }
}
//
// NON-PUBLIC TYPES
//

/// Writer wraps Arc<Mutex<...>> data structure to implement std::io::Write on it.
///
/// Implementation of std::io::Write is required by [tracing_subscriber] crate.
struct Writer<T>(Arc<Mutex<T>>)
where
    T: std::io::Write;

impl<T> std::io::Write for Writer<T>
where
    T: std::io::Write,
{
    delegate::delegate! {
        to self.0.lock().expect("logging mutex poisoned") {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;

            #[inline]
            fn flush(&mut self) -> std::io::Result<()> ;

            #[inline]
            fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize>;

            #[inline]
            fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;

            #[inline]
            fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()>;
        }
    }
}

impl<T: std::io::Write> From<T> for Writer<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

impl<T: std::io::Write> Clone for Writer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Log destination represents actual destination (implementing std::io::Write) where logs are sent
#[derive(Default)]
enum LogDestination {
    #[default]
    /// Standard error
    StdErr,
    /// Standard out
    StdOut,
    /// Standard file
    File(Writer<reopen::Reopen<std::fs::File>>),
    /// File that is logrotated
    RotationWriter(Writer<FileRotate<AppendTimestamp>>),
    #[cfg(test)]
    // Just some bytes, for testing
    Bytes(Writer<Vec<u8>>),
}

impl LogDestination {
    /// Convert this log destination to std::io::Write implementation
    fn to_write(&self) -> Box<dyn std::io::Write> {
        match self {
            LogDestination::StdErr => Box::new(std::io::stderr()) as Box<dyn std::io::Write>,
            LogDestination::StdOut => Box::new(std::io::stdout()) as Box<dyn std::io::Write>,
            LogDestination::File(f) => Box::new(f.clone()) as Box<dyn std::io::Write>,
            LogDestination::RotationWriter(w) => Box::new(w.clone()) as Box<dyn std::io::Write>,
            #[cfg(test)]
            LogDestination::Bytes(w) => Box::new(w.clone()) as Box<dyn std::io::Write>,
        }
    }

    /// Return human-readable name of selected log destination
    fn name(&self) -> String {
        let s = match self {
            LogDestination::StdOut => "stdout",
            LogDestination::StdErr => "stderr",
            LogDestination::File(_) => "file",
            LogDestination::RotationWriter(_) => "RotationWriter",
            #[cfg(test)]
            LogDestination::Bytes(_) => "ByteBuffer",
        };

        String::from(s)
    }
}

impl Debug for LogDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

/// Whenever we want to write to log destination, we delegate to the Writer implementation
impl std::io::Write for LogDestination {
    delegate::delegate! {
        to self.to_write() {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> ;

            #[inline]
            fn flush(&mut self) -> std::io::Result<()> ;

            #[inline]
            fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> ;

            #[inline]
            fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> ;

            #[inline]
            fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> ;
        }
    }
}

impl TryFrom<&LogConfig> for FileRotate<AppendTimestamp> {
    type Error = Error;
    /// Configure new FileRotate based on log configuration.
    ///
    /// In future, we might allow more detailed configuration, like log rotation frequency, compression, etc.
    fn try_from(config: &LogConfig) -> Result<Self, Self::Error> {
        let suffix_scheme = AppendTimestamp::default(FileLimit::MaxFiles(config.max_files));
        let content_limit = ContentLimit::Time(TimeFrequency::Daily);
        let compression = file_rotate::compression::Compression::OnRotate(2);
        // Only owner can see logs
        let mode = Some(0o600);
        let path = PathBuf::from(&config.destination);

        validate_log_path(&path)?;

        let f = FileRotate::new(path, suffix_scheme, content_limit, compression, mode);

        Ok(f)
    }
}

impl TryFrom<&LogConfig> for Reopen<File> {
    type Error = Error;
    /// Configure new File based on log configuration.
    fn try_from(config: &LogConfig) -> Result<Self, Self::Error> {
        // Only owner can see logs
        let mode = 0o600;
        let path = PathBuf::from(&config.destination);

        validate_log_path(&path)?;

        let opened_path = path.clone();
        let open_fn = move || {
            OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .mode(mode)
                .open(&opened_path)
        };

        Reopen::new(Box::new(open_fn)).map_err(|e| Error::FileCreate(path, e))
    }
}

// Individual logger
#[derive(Debug)]
struct Logger {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    destination: Arc<Mutex<LogDestination>>,

    /// Log verbosity level preset
    level: LogLevelPreset,

    /// Whether to use colored output
    color: Option<bool>,

    /// Log format to use
    format: LogFormat,
}

impl TryFrom<&LogConfig> for Logger {
    type Error = Error;
    fn try_from(config: &LogConfig) -> Result<Self, Self::Error> {
        let destination = match config.destination.to_lowercase().as_str() {
            LOG_DESTINATION_STDOUT => LogDestination::StdOut,
            LOG_DESTINATION_STDERR => LogDestination::StdErr,
            #[cfg(test)]
            LOG_DESTINATION_BYTES => LogDestination::Bytes(Vec::<u8>::new().into()),
            dest => {
                // we refer directly to config.destination, as dest was converted to lowercase
                let path = PathBuf::from(&config.destination);
                if !path.is_absolute() {
                    return Err(Error::InvalidDestination(dest.to_string()));
                }
                if config.max_files > 0 {
                    let file: FileRotate<AppendTimestamp> = FileRotate::try_from(config)?;
                    LogDestination::RotationWriter(file.into())
                } else {
                    let file: Reopen<File> = config.try_into()?;
                    LogDestination::File(file.into())
                }
            }
        };

        Ok(Self {
            destination: Arc::new(Mutex::new(destination)),
            level: config.level.clone(),
            color: config.color,
            format: config.format,
        })
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            destination: Arc::new(Mutex::new(LogDestination::StdOut)),
            level: LogLevelPreset::Info,
            color: None,
            format: LogFormat::Full,
        }
    }
}

impl Logger {
    /// Register the logger in a registry
    // : Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>
    fn layer(&self) -> impl Layer<Registry> {
        let ansi = self
            .color
            .unwrap_or(match self.destination.lock().unwrap().deref() {
                LogDestination::StdOut => atty::is(atty::Stream::Stdout),
                LogDestination::StdErr => atty::is(atty::Stream::Stderr),
                _ => false,
            });

        let cloned = self.destination.clone();
        let make_writer = { move || Writer(Arc::clone(&cloned)) };

        let filter = EnvFilter::from(&self.level);

        let formatter = fmt::layer::<Registry>()
            .with_writer(make_writer)
            .with_ansi(ansi);

        match self.format {
            LogFormat::Full => formatter.with_filter(filter).boxed(),
            LogFormat::Compact => formatter.compact().with_filter(filter).boxed(),
            LogFormat::Pretty => formatter.pretty().with_filter(filter).boxed(),
            LogFormat::Json => formatter.json().with_filter(filter).boxed(),
        }
    }
}

/// Helper that initializes logging in unit tests
///
///
/// For verbosity, see drive-abci --help or use 0 or 5
pub fn init_for_tests(level: LogLevelPreset) {
    let mut logger_builder = LogBuilder::new();
    let config = LogConfig {
        destination: LOG_DESTINATION_STDOUT.to_string(),
        level,
        color: None,
        format: LogFormat::Full,
        max_files: 0,
    };

    logger_builder
        .add("default", &config)
        .expect("cannot configure default logger");

    logger_builder.build().try_install().ok();
}

/// Verify log directory path and determine absolute path to log file.
fn validate_log_path<T: AsRef<Path>>(log_dir: T) -> Result<(), Error> {
    let log_dir = log_dir.as_ref();

    // TODO: Why it should be absolute?
    if !log_dir.is_absolute() {
        return Err(Error::FilePath(
            log_dir.to_owned(),
            "log path must be absolute".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use std::{fs, str::from_utf8};

    /// Reads data written to provided destination.
    ///
    /// Only [LogDestination::Bytes] and [LogDestination::RotationWriter] are supported.
    ///
    /// Contract: LogDestination::RotationWriter was rotated
    fn dest_read_as_string(dest: Arc<Mutex<LogDestination>>) -> String {
        let dest = dest.lock().unwrap();
        match dest.deref() {
            LogDestination::Bytes(b) => {
                let guard = b.0.lock().unwrap();
                let b = guard.clone();

                from_utf8(b.as_slice()).unwrap().to_string()
            }
            LogDestination::RotationWriter(w) => {
                let paths = w.0.lock().unwrap().log_paths();
                let path = paths.get(0).expect("exactly one path excepted");
                std::fs::read_to_string(path).unwrap()
            }
            _ => todo!(),
        }
    }

    /// Test that multiple loggers can work independently, with different log levels.
    ///
    /// Note that, due to limitation of [tracing::subscriber::set_global_default()], we can only have one test.
    #[test]
    fn test_logging() {
        let logger_stdout = LogConfig {
            destination: LOG_DESTINATION_STDOUT.to_string(),
            level: LogLevelPreset::Info,
            format: LogFormat::Pretty,
            ..Default::default()
        };

        let logger_stderr = LogConfig {
            destination: LOG_DESTINATION_STDERR.to_string(),
            level: LogLevelPreset::Debug,
            ..Default::default()
        };

        let logger_v0 = LogConfig {
            destination: LOG_DESTINATION_BYTES.to_string(),
            level: LogLevelPreset::Info,
            ..Default::default()
        };

        let logger_v4 = LogConfig {
            destination: "bytes".to_string(),
            level: LogLevelPreset::Debug,
            format: LogFormat::Json,
            ..Default::default()
        };

        let dir = TempDir::new().unwrap();

        let file_v0_path = dir.path().join("log.v0");
        let logger_file_v0 = LogConfig {
            destination: file_v0_path.to_string_lossy().to_string(),
            level: LogLevelPreset::Info,
            max_files: 4,
            ..Default::default()
        };

        let file_v4_path = dir.path().join("log.v4");
        let logger_file_v4 = LogConfig {
            destination: file_v4_path.to_string_lossy().to_string(),
            level: LogLevelPreset::Debug,
            max_files: 0, // no rotation
            ..Default::default()
        };

        let logging = LogBuilder::new()
            .with_config("stdout", &logger_stdout)
            .unwrap()
            .with_config("stderr", &logger_stderr)
            .unwrap()
            .with_config("v0", &logger_v0)
            .unwrap()
            .with_config("v4", &logger_v4)
            .unwrap()
            .with_config("file_v0", &logger_file_v0)
            .unwrap()
            .with_config("file_v4", &logger_file_v4)
            .unwrap()
            .build();
        logging.install();

        const TEST_STRING_DEBUG: &str = "testing debug trace";
        const TEST_STRING_ERROR: &str = "testing error trace";
        tracing::error!(TEST_STRING_ERROR);
        tracing::debug!(TEST_STRING_DEBUG);

        logging.flush().unwrap();
        logging.rotate().unwrap();

        // CHECK ASSERTIONS

        let result_verb_0 = dest_read_as_string(logging.0["v0"].destination.clone());
        let result_verb_4 = dest_read_as_string(logging.0["v4"].destination.clone());
        let result_dir_verb_0 = dest_read_as_string(logging.0["file_v0"].destination.clone());

        let result_file_verb_4 = std::fs::read_to_string(&file_v4_path)
            .map_err(|e| panic!("{:?}: {:?}", file_v4_path.clone(), e.to_string()))
            .unwrap();

        println!("{:?}", result_verb_0);
        println!("{:?}", result_verb_4);

        assert!(result_verb_0.contains(TEST_STRING_ERROR));
        assert!(result_dir_verb_0.contains(TEST_STRING_ERROR));
        assert!(result_verb_4.contains(TEST_STRING_ERROR));
        assert!(result_file_verb_4.contains(TEST_STRING_ERROR));

        assert!(!result_verb_0.contains(TEST_STRING_DEBUG));
        assert!(!result_dir_verb_0.contains(TEST_STRING_DEBUG));
        assert!(result_verb_4.contains(TEST_STRING_DEBUG));
        assert!(result_file_verb_4.contains(TEST_STRING_DEBUG));
    }

    /// Test rotation of RotationWriter destination.
    ///
    /// Given that the RotationWriter is rotated 3 times, we expect to see 4 files:
    /// - 1 file with the original name
    /// - 3 files with the original name and timestamp suffix
    #[test]
    fn test_rotation_writer_rotate() {
        let tempdir = TempDir::new().unwrap();
        let config = LogConfig {
            destination: tempdir.path().to_string_lossy().to_string(),
            level: LogLevelPreset::Trace,
            format: LogFormat::Pretty,
            max_files: 3,
            ..Default::default()
        };

        let loggers = LogBuilder::new()
            .with_config("rotate", &config)
            .expect("configure log builder")
            .build();
        let logger = loggers.0.get("rotate").expect("get logger");

        for i in 0..config.max_files + 2 {
            logger
                .destination
                .lock()
                .unwrap()
                .write_all(format!("file {}\n", i).as_bytes())
                .unwrap();

            loggers.rotate().expect("rotate logs");
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        let mut counter = 0;
        tempdir.path().read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let path = path.to_string_lossy();
            println!("{}", path);
            assert!(path.contains("drive-abci.log"));
            counter = counter + 1;
        });
        assert_eq!(counter, config.max_files + 1);
    }

    /// Test rotation of File destination.
    ///
    /// Given that we move the File and then Rotate it, we expect the file to be recreated in new location.
    #[test]
    fn test_file_rotate() {
        const ITERATIONS: usize = 4;

        let tempdir = TempDir::new().unwrap();
        let filepath = tempdir.path().join("drive-abci.log");
        let config = LogConfig {
            destination: tempdir.path().to_string_lossy().to_string(),
            level: LogLevelPreset::Trace,
            format: LogFormat::Pretty,
            max_files: 0,
            ..Default::default()
        };

        let loggers = LogBuilder::new()
            .with_config("rotate", &config)
            .expect("configure log builder")
            .build();
        let logger = loggers.0.get("rotate").expect("get logger");

        for i in 0..ITERATIONS {
            let mut guard = logger.destination.lock().unwrap();
            guard
                .write_all(format!("file {}, before rotate\n", i).as_bytes())
                .unwrap();

            fs::rename(
                &filepath,
                tempdir.path().join(format!("drive-abci.log.{}", i)),
            )
            .unwrap();
            // rotate() locks, so we need to drop guard here
            drop(guard);

            loggers.rotate().expect("rotate logs");
            let mut guard = logger.destination.lock().unwrap();
            guard
                .write_all(format!("file {}, after rotate\n", i + 1).as_bytes())
                .unwrap();
            guard.flush().unwrap();

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Close all files, so that we can read them
        drop(loggers);

        let mut counter = 0;
        tempdir.path().read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let path_str = path.to_string_lossy();
            let read = fs::read_to_string(&path).unwrap();
            println!("{}: {}", path_str, read);
            assert!(path_str.contains("drive-abci.log"));

            if counter < ITERATIONS - 1 {
                assert!(
                    read.contains(format!("file {}, before rotate\n", counter).as_str()),
                    "expect: file {}, before rotate, read: {}",
                    counter,
                    read
                )
            };
            if counter > 0 {
                assert!(
                    read.contains(format!("file {}, after rotate\n", counter).as_str()),
                    "expect: file {}, after rotate, read: {}",
                    counter,
                    read
                )
            }

            counter = counter + 1;
        });
        assert_eq!(counter, ITERATIONS + 1);
    }
}
