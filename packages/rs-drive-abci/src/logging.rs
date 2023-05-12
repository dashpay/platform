use std::default;
use std::fmt::Debug;
use std::io::BufWriter;
use std::io::Stderr;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use file_rotate::suffix::AppendTimestamp;
use file_rotate::suffix::FileLimit;
use file_rotate::ContentLimit;
use file_rotate::TimeFrequency;
use serde::Deserialize;
use tracing::Subscriber;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::ArcWriter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::fmt::writer::MutexGuardWriter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

#[derive(Default, Debug)]
pub enum LogDestination<'a> {
    #[default]
    StdErr,
    StdOut,
    RotationWriter(Arc<Mutex<RotationWriter>>),
    ByteBuffer(Mutex<&'a mut [u8]>),
}

struct RotationWriter {
    inner: file_rotate::FileRotate<AppendTimestamp>,
}

impl Debug for RotationWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RotationWriter")
    }
}

// impl std::io::Write for &mut RotationWriter {
//     #[inline]
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         self.inner.write(buf)
//     }

//     #[inline]
//     fn flush(&mut self) -> std::io::Result<()> {
//         self.inner.flush()
//     }

//     #[inline]
//     fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
//         self.inner.write_vectored(bufs)
//     }

//     #[inline]
//     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
//         self.inner.write_all(buf)
//     }

//     #[inline]
//     fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
//         self.inner.write_fmt(fmt)
//     }
// }
impl RotationWriter {
    fn new(path: &Path, max_files: usize) -> Self {
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
pub struct Logger<'writer> {
    /// Destination of logs; either absolute path to dir where log files will be stored, `stdout` or `stderr`
    pub destination: LogDestination<'writer>,

    /// Log verbosity level, number; see [super::Cli::verbose].
    pub verbosity: u8,

    /// Whether to use colored output
    pub color: Option<bool>,

    /// Max number of files to keep; only applies to file destination
    pub max_files: usize,
}

impl<'a> Default for Logger<'a> {
    fn default() -> Self {
        Self {
            destination: LogDestination::StdErr,
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

impl<'a> Logger<'a> {
    pub fn subscriber(&'static self) {
        let builder = fmt::fmt();

        let ansi = self.color.unwrap_or(match self.destination {
            LogDestination::StdOut => atty::is(atty::Stream::Stdout),
            LogDestination::StdErr => atty::is(atty::Stream::Stderr),
            _ => false,
        });
        let self_arc = Arc::new(self);

        let fmt_layer = fmt::layer().with_ansi(ansi).with_writer(self);

        let registry = tracing_subscriber::registry();
        let subscriber = registry.with(fmt_layer).with(self.env_filter());
    }

    fn env_filter(&self) -> EnvFilter {
        match self.verbosity {
            0 => EnvFilter::builder()
                .with_default_directive(
                    "error,tenderdash_abci=warn,drive_abci=warn"
                        .parse()
                        .unwrap(),
                )
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
//impl<'a, W> MakeWriter<'a> for Mutex<W>
impl<'writer> MakeWriter<'writer> for &Logger<'writer> {
    type Writer = Box<dyn std::io::Write + 'writer>;
    fn make_writer(&'writer self) -> Self::Writer {
        let writer = match &self.destination {
            LogDestination::StdErr => Box::new(std::io::stderr()) as Self::Writer,
            LogDestination::StdOut => Box::new(std::io::stdout()) as Self::Writer,
            LogDestination::RotationWriter(w) => {
                let mut w = Arc::clone(w);
                let mut w = w.lock().unwrap().inner;
                Box::new(&mut w) as Self::Writer
            }
            // LogDestination::ByteBuffer(buf) => {
            //     let b = buf.lock().expect("logging mux is poisoned").as_mut();
            //     Box::new(b)
            // }
            _ => todo!(),
        };

        writer
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_two_loggers() {
        let mut buf1 = Vec::<u8>::new();
        let mut buf2 = Vec::<u8>::new();

        let logger1 = Logger {
            destination: super::LogDestination::ByteBuffer(Mutex::new(&mut buf1)),
            ..Default::default()
        };
        let logger2 = Logger {
            destination: super::LogDestination::ByteBuffer(Mutex::new(&mut buf2)),
            ..Default::default()
        };
    }
}
