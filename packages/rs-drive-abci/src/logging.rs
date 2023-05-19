use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use file_rotate::suffix::AppendTimestamp;
use file_rotate::suffix::FileLimit;
use file_rotate::ContentLimit;
use file_rotate::TimeFrequency;
use itertools::Itertools;
use lazy_static::__Deref;
use serde::Deserialize;
use serde::Serialize;
use tracing::metadata::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::registry;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

/// Logging configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogConfig {
    /// Destination of logs; either absolute path to dir where log files will be stored, "stdout" or "stderr".
    ///
    /// For testing, use "bytes".
    pub destination: String,
    /// Verbosity level, 0 to 5; see `-v` option in `drive-abci --help` for more details.
    pub verbosity: u8,
    /// Whether or not to use colorful output; defaults to autodetect
    pub color: Option<bool>,

    /// Max number of daily files to store; only used when storing logs in file; defaults to 7
    pub max_files: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            destination: String::from("stderr"),
            verbosity: 0,
            color: None,
            max_files: 7,
        }
    }
}

/// Errors returned by logging subsystem
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// File rotation error
    #[error("file rotation: {0}")]
    FileRotate(std::io::Error),

    /// Invalid destination
    #[error(
        "invalid destination {0}: must be one of: stderr, stdout, or absolute path to a directory"
    )]
    InvalidDestination(String),

    /// Log file path is invalid
    #[error("log file path {0}: {1}")]
    FilePath(PathBuf, String),
}
/// LogController is managing logging methods.
pub struct LogController {
    loggers: Vec<Logger>,
}

type LoggerID = usize;

impl LogController {
    /// Create new LogController
    pub fn new() -> Self {
        Self {
            loggers: Vec::new(),
        }
    }

    /// Add new logger to log controller.
    ///
    /// Returns ID of log controller.
    pub fn add(&mut self, config: &LogConfig) -> Result<LoggerID, Error> {
        let logger = Logger::try_from(config)?;
        self.loggers.push(logger);
        Ok(self.loggers.len() - 1)
    }

    /// Flush all loggers.
    ///
    /// Note that we ignore errors.
    pub fn flush(&self) {
        for logger in self.loggers.iter() {
            logger
                .destination
                .clone()
                .lock()
                .expect("logging lock poisoned")
                .to_writer()
                .flush()
                .ok();
        }
    }

    /// Trigger log rotation.
    ///
    /// In case of multiple errors, we return error from last logger.
    pub fn rotate(&self) -> Result<(), Error> {
        let mut result: Result<(), Error> = Ok(());

        for logger in self.loggers.iter() {
            let logger = logger.destination.lock().expect("logging lock poisoned");

            if let LogDestination::RotationWriter(writer) = logger.deref() {
                if let Err(e) = writer.lock().expect("logging lock poisoned").inner.rotate() {
                    result = Err(Error::FileRotate(e));
                };
            };
        }
        result
    }

    /// Initialize logging subsystem after finishing configuration.
    ///
    /// Panics if logging subsystem is already initialized.
    pub fn finalize(&self) {
        // Based on  examples from  https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/layer/index.html
        let loggers = self
            .loggers
            .iter()
            .map(|l| Box::new(l.layer()))
            .collect_vec();

        registry().with(loggers).init();
    }
}

//
// NON-PUBLIC TYPES
//

/// Where to send logs
#[derive(Default)]
enum LogDestination {
    #[default]
    /// Standard error
    StdErr,
    /// Standard out
    StdOut,
    /// File that is logrotated
    RotationWriter(Arc<Mutex<RotationWriter>>),
    #[cfg(test)]
    // Just some bytes, for testing
    Bytes(Arc<Mutex<Vec<u8>>>),
}

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

impl LogDestination {
    fn to_writer(&self) -> Box<dyn std::io::Write> {
        let writer = match self {
            LogDestination::StdErr => Box::new(std::io::stderr()) as Box<dyn std::io::Write>,
            LogDestination::StdOut => Box::new(std::io::stdout()) as Box<dyn std::io::Write>,
            LogDestination::RotationWriter(w) => Box::new(Writer(Arc::clone(w))),
            #[cfg(test)]
            LogDestination::Bytes(buf) => Box::new(Writer(buf.clone())) as Box<dyn std::io::Write>,
        };

        writer
    }

    /// Return human-readable name of selected log destination
    fn name(&self) -> String {
        let s = match self {
            LogDestination::StdOut => "stdout",
            LogDestination::StdErr => "stderr",
            #[cfg(test)]
            LogDestination::Bytes(_) => "ByteBuffer",
            LogDestination::RotationWriter(_) => "RotationWriter",
        };

        String::from(s)
    }
}

impl Debug for LogDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

impl std::io::Write for LogDestination {
    delegate::delegate! {
        to self.to_writer() {
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

/// RotationWriter allows writing logs to a file that is automatically rotated
struct RotationWriter {
    inner: file_rotate::FileRotate<AppendTimestamp>,
}

impl RotationWriter {
    /// Create new rotating writer
    fn new(path: &PathBuf, max_files: usize) -> Self {
        let suffix_scheme = AppendTimestamp::default(FileLimit::MaxFiles(max_files));
        let content_limit = ContentLimit::Time(TimeFrequency::Daily);
        let compression = file_rotate::compression::Compression::OnRotate(2);
        let mode = Some(0o600);
        let f = file_rotate::FileRotate::new(path, suffix_scheme, content_limit, compression, mode);

        Self { inner: f }
    }
}

impl std::io::Write for RotationWriter {
    delegate::delegate! {
        to self.inner {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;

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

impl Debug for RotationWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RotationWriter")
    }
}

/// Logger configuration
#[derive(Debug)]
struct Logger {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    destination: Arc<Mutex<LogDestination>>,

    /// Log verbosity level, number; see [super::Cli::verbose].
    verbosity: u8,

    /// Whether to use colored output
    color: Option<bool>,
}

impl TryFrom<&LogConfig> for Logger {
    type Error = Error;
    fn try_from(config: &LogConfig) -> Result<Self, Self::Error> {
        let max_files = config.max_files;
        let destination = match config.destination.as_str() {
            "stdout" => LogDestination::StdOut,
            "stderr" => LogDestination::StdErr,
            #[cfg(test)]
            "bytes" => LogDestination::Bytes(Arc::new(Mutex::new(Vec::<u8>::new()))),
            dest => {
                let path = PathBuf::from(dest);

                if !path.is_absolute() {
                    return Err(Error::InvalidDestination(dest.to_string()));
                }
                if !path.is_dir() {
                    return Err(Error::FilePath(
                        path,
                        "Path must be an existing directory".into(),
                    ));
                }

                let writer = RotationWriter::new(&PathBuf::from(path), max_files);
                LogDestination::RotationWriter(Arc::new(Mutex::new(writer)))
            }
        };
        let verbosity = config.verbosity;
        let color = config.color;

        Ok(Self {
            destination: Arc::new(Mutex::new(destination)),
            verbosity,
            color,
        })
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            destination: Arc::new(Mutex::new(LogDestination::StdErr)),
            verbosity: 0,
            color: None,
        }
    }
}

// fn with<L>(self, layer: L) -> Layered<L, Self>
//     where
//         L: Layer<Self>,
//         Self: Sized,

impl<S: tracing::Subscriber> Layer<S> for Logger {}

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
        let dest = self.destination.clone();
        let cloned = dest;
        // let make_writer = { move || Writer(dest.clone()) };
        let make_writer = { move || Writer(Arc::clone(&cloned)) };

        let filter = self.env_filter();
        let formatter = fmt::layer::<Registry>()
            .with_writer(make_writer)
            .with_ansi(ansi);

        let layered = formatter.with_filter(filter);

        layered
    }

    fn env_filter(&self) -> EnvFilter {
        match self.verbosity {
            0 => EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
            1 => EnvFilter::new("error,tenderdash_abci=info,drive_abci=info"),
            2 => EnvFilter::new("info,tenderdash_abci=debug,drive_abci=debug"),
            3 => EnvFilter::new("debug"),
            4 => EnvFilter::new("debug,tenderdash_abci=trace,drive_abci=trace"),
            5 => EnvFilter::new("trace"),
            _ => panic!("max verbosity level is 5"),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use std::{io::Write, ops::DerefMut, str::from_utf8};

    fn dest_bytes(dest: Arc<Mutex<LogDestination>>) -> String {
        let dest = dest.lock().unwrap();
        let bytes_v0 = if let LogDestination::Bytes(b) = dest.deref() {
            let guard = b.lock().unwrap();
            let b = guard.clone();
            b
        } else {
            panic!("wrong type of log destination 0")
        };

        from_utf8(bytes_v0.as_slice()).unwrap().to_string()
    }

    /// Test that two loggers can work independently, with different log levels.
    ///
    /// Note that, due to limitation of [tracing::subscriber::set_global_default()], we can only have one test.
    #[test]
    fn test_two_loggers_bytes() {
        let mut logging = LogController::new();

        let logger_stdout = LogConfig {
            destination: "stdout".to_string(),
            verbosity: 0,
            ..Default::default()
        };
        logging.add(&logger_stdout).unwrap();

        let logger_stderr = LogConfig {
            destination: "stderr".to_string(),
            verbosity: 4,
            ..Default::default()
        };
        logging.add(&logger_stderr).unwrap();

        let logger_v0 = LogConfig {
            destination: "bytes".to_string(),
            verbosity: 0,
            ..Default::default()
        };
        logging.add(&logger_v0).unwrap();

        let logger_v4 = LogConfig {
            destination: "bytes".to_string(),
            verbosity: 4,
            ..Default::default()
        };
        logging.add(&logger_v4).unwrap();

        let dir_v0 = TempDir::new().unwrap();
        let logger_dir_v0 = LogConfig {
            destination: dir_v0
                .path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            verbosity: 4,
            ..Default::default()
        };
        logging.add(&logger_dir_v0).unwrap();

        logging.finalize();

        const TEST_STRING_DEBUG: &str = "testing debug trace";
        const TEST_STRING_ERROR: &str = "testing error trace";
        tracing::error!(TEST_STRING_ERROR);
        tracing::debug!(TEST_STRING_DEBUG);

        // CHECK ASSERTIONS

        let result_verbosity_0 = dest_bytes(logging.loggers[2].destination.clone());
        let result_verbosity_4 = dest_bytes(logging.loggers[3].destination.clone());

        println!("{:?}", result_verbosity_0);
        println!("{:?}", result_verbosity_4);
        println!("Dest dir: {:?}", dir_v0);

        assert!(result_verbosity_0.contains(TEST_STRING_ERROR));
        assert!(result_verbosity_4.contains(TEST_STRING_ERROR));

        assert!(!result_verbosity_0.contains(TEST_STRING_DEBUG));
        assert!(result_verbosity_4.contains(TEST_STRING_DEBUG));

        if let LogDestination::RotationWriter(w) = logging.loggers[4]
            .destination
            .clone()
            .lock()
            .unwrap()
            .deref()
        {
            let c = w.clone();
            let mut w = c.lock().unwrap();
            let mut cloned = w.deref_mut();
            cloned.write_all("test\n".as_bytes()).unwrap();
            println!("{:?}", cloned.inner.log_paths())
        } else {
            panic!("not a rotation writer")
        }

        // std::thread::sleep(std::time::Duration::from_secs(30));
    }
}
