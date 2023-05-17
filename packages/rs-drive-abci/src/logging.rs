use std::fmt::Debug;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use bytes::Bytes;
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
    RotationWriter(RotationWriter),
    // Just some bytes
    Bytes(Arc<Mutex<Vec<u8>>>),
}

struct Writer<T>(Arc<Mutex<T>>);

impl<T: std::io::Write> std::io::Write for Writer<T> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("logging mutex poisoned").write(buf)
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().expect("logging mutex poisoned").flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("logging mutex poisoned")
            .write_vectored(bufs)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.0
            .lock()
            .expect("logging mutex poisoned")
            .write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.0
            .lock()
            .expect("logging mutex poisoned")
            .write_fmt(fmt)
    }
}

// trait LogWriter {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
//     fn flush(&mut self) -> std::io::Result<()>;
//     fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize>;
//     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
//     fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()>;
// }

// impl<T: std::io::Write> LogWriter for Mutex<T> {
//     #[inline]
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         self.lock().expect("logging mutex poisoned").write(buf)
//     }

//     #[inline]
//     fn flush(&mut self) -> std::io::Result<()> {
//         self.lock().expect("logging mutex poisoned").flush()
//     }

//     #[inline]
//     fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
//         self.lock()
//             .expect("logging mutex poisoned")
//             .write_vectored(bufs)
//     }

//     #[inline]
//     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
//         self.lock().expect("logging mutex poisoned").write_all(buf)
//     }

//     #[inline]
//     fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
//         self.lock().expect("logging mutex poisoned").write_fmt(fmt)
//     }
// }
impl LogDestination {
    fn to_writer(&self) -> Box<dyn std::io::Write> {
        let writer = match self {
            LogDestination::StdErr => Box::new(std::io::stderr()) as Box<dyn std::io::Write>,
            LogDestination::StdOut => Box::new(std::io::stdout()) as Box<dyn std::io::Write>,
            // LogDestination::RotationWriter(w) => {
            //     let mut w = Arc::clone(w);
            //     let mut w = w.lock().unwrap().inner;
            //     Box::new(&mut w) as Self::Writer
            // }
            // LogDestination::ByteBuffer(buf) => {
            //     let b = buf.lock().expect("logging mux is poisoned").as_mut();
            //     Box::new(Arc::clone(buf.lock().expect("logging mux is poisoned")))
            // }
            _ => todo!(),
        };

        writer
    }

    /// Return human-readable name of selected log destination
    pub fn name(&self) -> String {
        let s = match self {
            LogDestination::StdOut => "stdout",
            LogDestination::StdErr => "stderr",
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
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.to_writer().write(buf)
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.to_writer().flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.to_writer().write_vectored(bufs)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.to_writer().write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.to_writer().write_fmt(fmt)
    }
}

/// RotationWriter allows writing logs to a file that is automatically rotated
pub struct RotationWriter {
    inner: file_rotate::FileRotate<AppendTimestamp>,
}

impl Debug for RotationWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RotationWriter")
    }
}

impl RotationWriter {
    /// Create new rotating writer
    pub fn new(path: &Path, max_files: usize) -> Self {
        let suffix_scheme = AppendTimestamp::default(FileLimit::MaxFiles(max_files));
        let content_limit = ContentLimit::Time(TimeFrequency::Daily);
        let compression = file_rotate::compression::Compression::OnRotate(2);
        let mode = Some(0o600);
        let f = file_rotate::FileRotate::new(path, suffix_scheme, content_limit, compression, mode);

        Self { inner: f }
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

        // let make_writer = self.destination.clone() as Arc<Mutex<dyn std::io::Write>>;
        // let make_writer = *make_writer;
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

#[cfg(test)]
mod tests {
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{
        prelude::__tracing_subscriber_SubscriberExt, registry, FmtSubscriber, Registry,
    };

    use super::*;
    #[test]
    fn test_two_loggers_bytes() {
        let buf1 = Arc::new(Mutex::new(Vec::<u8>::new()));
        let buf2 = Arc::new(Mutex::new(Vec::<u8>::new()));

        let logger1 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::Bytes(buf1))),
            ..Default::default()
        };
        let logger2 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::Bytes(buf2))),
            ..Default::default()
        };

        // let subscriber1 = logger1.subscriber::<Registry>();
        // let subscriber2 = logger2.subscriber::<Registry>();

        let r = registry();
        r.with(logger1.layer()).with(logger2.layer()).init();

        tracing::error!("testing error trace");
    }

    #[test]
    fn test_stdout_stderr() {
        let logger1 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::StdOut)),
            verbosity: 1,
            ..Default::default()
        };
        let logger2 = Logger {
            destination: Arc::new(Mutex::new(super::LogDestination::StdErr)),
            verbosity: 5,
            ..Default::default()
        };

        // let subscriber1 = logger1.subscriber::<Registry>();
        // let subscriber2 = logger2.subscriber::<Registry>();

        let r = registry();
        r.with(logger1.layer()).with(logger2.layer()).init();

        tracing::error!("testing error trace - should be visibe 2 times");
        tracing::debug!("testing debug trace - should be visibe 1 time");
    }
}
