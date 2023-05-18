use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use file_rotate::suffix::AppendTimestamp;
use file_rotate::suffix::FileLimit;
use file_rotate::ContentLimit;
use file_rotate::TimeFrequency;
use lazy_static::__Deref;
use tracing::metadata::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

/// Where to send logs
#[derive(Default)]
pub enum LogDestination {
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

// impl<T> From<T> for Writer<T>
// where
//     T: std::io::Write,
// {
//     fn from(value: T) -> Self {
//         Self(Arc::new(Mutex::new(value)))
//     }
// }

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
            LogDestination::RotationWriter(w) => Box::new(Writer(w.clone())),
            #[cfg(test)]
            LogDestination::Bytes(buf) => {
                // let mut d = buf.lock().unwrap();
                Box::new(Writer(buf.clone())) as Box<dyn std::io::Write>
            }
        };

        writer
    }

    /// Return human-readable name of selected log destination
    pub fn name(&self) -> String {
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
pub struct RotationWriter {
    inner: file_rotate::FileRotate<AppendTimestamp>,
}

impl RotationWriter {
    /// Create new rotating writer
    pub fn new(path: &PathBuf, max_files: usize) -> Self {
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
pub struct Logger {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    pub destination: Arc<Mutex<LogDestination>>,

    /// Log verbosity level, number; see [super::Cli::verbose].
    pub verbosity: u8,

    /// Whether to use colored output
    pub color: Option<bool>,

    /// Max number of files to keep; only applies to file destination
    pub max_files: usize,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            destination: Arc::new(Mutex::new(LogDestination::StdErr)),
            verbosity: 0,
            color: None,
            max_files: 7,
        }
    }
}

// fn with<L>(self, layer: L) -> Layered<L, Self>
//     where
//         L: Layer<Self>,
//         Self: Sized,

impl Logger {
    /// Register the logger in a registry
    pub fn layer<S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>>(
        &self,
    ) -> impl Layer<S> {
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
        let formatter = fmt::layer::<S>().with_writer(make_writer).with_ansi(ansi);

        let layered = formatter.with_filter(filter);

        layered
    }

    fn env_filter(&self) -> EnvFilter {
        const default_logging: &str = "*=error,tenderdash_abci=warn,drive_abci=warn";
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

pub struct LogConfig {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    destination: String,
    /// Verbosity level, 0 to 5; see `-v` option in `drive-abci --help` for more details.
    verbosity: u8,
    /// Whether or not to use colorful output; defaults to autodetect
    color: Option<bool>,

    /// Max number of daily files to store; only used when storing logs in file; defaults to 7
    max_files: usize,
}

// TODO: move to correct place
pub enum LogError {
    Generic(String),
}

impl TryFrom<LogConfig> for Logger {
    type Error = LogError;
    fn try_from(config: LogConfig) -> Result<Self, Self::Error> {
        let max_files = config.max_files;
        let destination = match config.destination.as_str() {
            "stdout" => LogDestination::StdOut,
            "stderr" => LogDestination::StdErr,
            path => {
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
            max_files,
        })
    }
}

pub struct LogController {}

impl LogController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn add() {}
}

#[cfg(test)]
mod tests {
    use std::str::from_utf8;

    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, registry};

    use super::*;

    /// Test that two loggers can work independently, with different log levels.
    ///
    /// Note that, due to limitation of [tracing::subscriber::set_global_default()], we can only have one test.
    #[test]
    fn test_two_loggers_bytes() {
        let buf_v0 = Arc::new(Mutex::new(Vec::<u8>::new()));
        let logger_v0 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::Bytes(buf_v0.clone()))),
            verbosity: 0,
            ..Default::default()
        };

        let buf_v4 = Arc::new(Mutex::new(Vec::<u8>::new()));
        let logger_v4 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::Bytes(buf_v4.clone()))),
            verbosity: 4,
            ..Default::default()
        };

        registry()
            .with(logger_v0.layer())
            .with(logger_v4.layer())
            .init();

        const TEST_STRING_DEBUG: &str = "testing debug trace";
        const TEST_STRING_ERROR: &str = "testing error trace";
        tracing::error!(TEST_STRING_ERROR);
        tracing::debug!(TEST_STRING_DEBUG);

        let bytes_v0 = buf_v0.lock().unwrap();
        let bytes_v0 = from_utf8(&bytes_v0).unwrap();
        let bytes_v4 = buf_v4.lock().unwrap();
        let bytes_v4 = from_utf8(&bytes_v4).unwrap();

        assert!(String::from(bytes_v0).contains(TEST_STRING_ERROR));
        assert!(String::from(bytes_v4).contains(TEST_STRING_ERROR));

        assert!(!String::from(bytes_v0).contains(TEST_STRING_DEBUG));
        assert!(String::from(bytes_v4).contains(TEST_STRING_DEBUG));
    }
}
