use crate::logging::config::LogConfig;
use crate::logging::destination::{LogDestinationWriter, Writer};
use crate::logging::error::Error;
use crate::logging::{LogConfigs, LogFormat, LogLevel};
use lazy_static::__Deref;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::registry;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

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
    pub fn add(&mut self, configuration_name: &str, config: &LogConfig) -> Result<(), Error> {
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

    /// Returns a Logger with the specified ID.
    pub fn get(&self, id: &str) -> Option<&Logger> {
        self.0.get(id)
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
    pub fn try_install(&self) -> Result<(), Error> {
        // Based on examples from https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/layer/index.html
        let loggers = self
            .0
            .values()
            .map(|l| Ok(Box::new(l.layer()?)))
            .collect::<Result<Vec<_>, _>>()?;

        // Initialize Tokio console subscriber

        // #[cfg(feature = "console")]
        // {
        //     let console_layer = console_subscriber::spawn();
        //     registry();
        // }

        // TODO: Must be under feature flag

        let console_layer = console_subscriber::spawn();

        registry()
            .with(loggers)
            .with(console_layer)
            .try_init()
            .map_err(Error::TryInitError)
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
        self.0.values().try_for_each(|logger| logger.rotate())
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

// Individual logger
#[derive(Debug)]
pub struct Logger {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    pub(super) destination: Arc<Mutex<LogDestinationWriter>>,

    /// Log verbosity level preset
    level: LogLevel,

    /// Whether to use colored output
    color: Option<bool>,

    /// Log format to use
    format: LogFormat,
}

impl TryFrom<&LogConfig> for Logger {
    type Error = Error;
    fn try_from(config: &LogConfig) -> Result<Self, Self::Error> {
        let destination = config.try_into()?;

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
            destination: Arc::new(Mutex::new(LogDestinationWriter::StdOut)),
            level: LogLevel::Info,
            color: None,
            format: LogFormat::Full,
        }
    }
}

impl Logger {
    /// Register the logger in a registry
    // : Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>
    fn layer(&self) -> Result<impl Layer<Registry>, Error> {
        let ansi = self
            .color
            .unwrap_or(match self.destination.lock().unwrap().deref() {
                LogDestinationWriter::StdOut => atty::is(atty::Stream::Stdout),
                LogDestinationWriter::StdErr => atty::is(atty::Stream::Stderr),
                _ => false,
            });

        let cloned = self.destination.clone();
        let make_writer = { move || Writer::new(Arc::clone(&cloned)) };

        let filter = EnvFilter::try_from(&self.level)?;

        let formatter = fmt::layer::<Registry>()
            .with_writer(make_writer)
            .with_ansi(ansi)
            .with_thread_names(true)
            .with_thread_ids(true);

        let formatter = match self.format {
            LogFormat::Full => formatter.with_filter(filter).boxed(),
            LogFormat::Compact => formatter.compact().with_filter(filter).boxed(),
            LogFormat::Pretty => formatter.pretty().with_filter(filter).boxed(),
            LogFormat::Json => formatter.json().with_filter(filter).boxed(),
        };

        Ok(formatter)
    }

    /// Rotate log files
    pub fn rotate(&self) -> Result<(), Error> {
        let cloned = self.destination.clone();
        let guard = cloned.lock().expect("logging lock poisoned");

        guard.rotate()
    }
}
