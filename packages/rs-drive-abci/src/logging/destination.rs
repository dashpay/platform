use super::error::Error;
use crate::logging::config::LogConfig;
use file_rotate::suffix::{AppendTimestamp, FileLimit};
use file_rotate::{ContentLimit, FileRotate, TimeFrequency};
use reopen::Reopen;
use std::fmt::{Debug, Display};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::str::from_utf8;

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::sync::{Arc, Mutex};
use std::{fmt, fs, path};

/// Log destination configuration that will be converted to LogDestinationWriter
#[derive(Default, Serialize, Clone, Debug)]
pub enum LogDestination {
    /// Standard error
    StdErr,
    #[default]
    /// Standard out
    StdOut,
    /// File
    File(PathBuf),
    /// Blob of bytes for testing
    #[cfg(test)]
    Bytes,
}

impl Display for LogDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogDestination::StdErr => write!(f, "stderr"),
            LogDestination::StdOut => write!(f, "stdout"),
            LogDestination::File(path) => write!(f, "{}", path.to_string_lossy()),
            #[cfg(test)]
            LogDestination::Bytes => write!(f, "bytes"),
        }
    }
}

/// Creates log destination from string
impl From<&str> for LogDestination {
    fn from(value: &str) -> Self {
        match value {
            "stdout" => LogDestination::StdOut,
            "stderr" => LogDestination::StdErr,
            #[cfg(test)]
            "bytes" => LogDestination::Bytes,
            file_path => LogDestination::File(PathBuf::from(file_path)),
        }
    }
}

struct LogDestinationVisitor;

impl Visitor<'_> for LogDestinationVisitor {
    type Value = LogDestination;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("stdout, stderr, or absolute path to a file log destination")
    }

    fn visit_str<E>(self, value: &str) -> Result<LogDestination, E>
    where
        E: de::Error,
    {
        let destination = LogDestination::from(value);

        Ok(destination)
    }
}

impl<'de> Deserialize<'de> for LogDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LogDestinationVisitor)
    }
}

/// Writer wraps Arc<Mutex<...>> data structure to implement std::io::Write on it.
///
/// Implementation of std::io::Write is required by [tracing_subscriber] crate.
pub(super) struct Writer<T>(Arc<Mutex<T>>)
where
    T: Write;

impl<T> Write for Writer<T>
where
    T: Write,
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

impl<T> Writer<T>
where
    T: Write,
{
    /// Create writer
    pub(super) fn new(write: Arc<Mutex<T>>) -> Self {
        Self(write)
    }
}

impl<T: Write> From<T> for Writer<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

impl<T: Write> Clone for Writer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Log destination represents actual destination (implementing std::io::Write) where logs are sent
pub(super) enum LogDestinationWriter {
    /// Standard error
    StdErr,
    /// Standard out
    StdOut,
    /// File
    File(Writer<Reopen<File>>),
    /// Rotated file
    RotationWriter(Writer<FileRotate<AppendTimestamp>>),
    #[cfg(test)]
    // Just some bytes, for testing
    Bytes(Writer<Vec<u8>>),
}

impl LogDestinationWriter {
    /// Convert this log destination to std::io::Write implementation
    pub fn to_write(&self) -> Box<dyn Write> {
        match self {
            LogDestinationWriter::StdErr => Box::new(std::io::stderr()) as Box<dyn Write>,
            LogDestinationWriter::StdOut => Box::new(std::io::stdout()) as Box<dyn Write>,
            LogDestinationWriter::File(f) => Box::new(f.clone()) as Box<dyn Write>,
            LogDestinationWriter::RotationWriter(w) => Box::new(w.clone()) as Box<dyn Write>,
            #[cfg(test)]
            LogDestinationWriter::Bytes(w) => Box::new(w.clone()) as Box<dyn Write>,
        }
    }

    /// Return human-readable name of selected log destination
    pub fn name(&self) -> String {
        let s = match self {
            LogDestinationWriter::StdOut => "stdout",
            LogDestinationWriter::StdErr => "stderr",
            LogDestinationWriter::File(_) => "file",
            LogDestinationWriter::RotationWriter(_) => "RotationWriter",
            #[cfg(test)]
            LogDestinationWriter::Bytes(_) => "ByteBuffer",
        };

        String::from(s)
    }

    /// Rotate log file
    pub fn rotate(&self) -> Result<(), Error> {
        match self {
            LogDestinationWriter::RotationWriter(ref writer) => {
                let mut file_rotate_guard = writer.0.lock().expect("logging lock poisoned");

                file_rotate_guard.rotate().map_err(Error::FileRotate)?;
            }
            LogDestinationWriter::File(ref f) => {
                let mut file_reopen_guard = f.0.lock().expect("logging lock poisoned");

                file_reopen_guard
                    .flush()
                    .map_err(Error::FileRotate)
                    .map(|_| {
                        file_reopen_guard.handle().reopen();
                    })?
            }
            _ => {}
        };

        Ok(())
    }

    /// Reads data written into destination
    ///
    /// Only [LogDestinationWriter::Bytes] and [LogDestinationWriter::RotationWriter] are supported.
    ///
    /// Contract: LogDestinationWriter::RotationWriter was rotated
    #[cfg(test)]
    pub fn read_as_string(&self) -> String {
        match self {
            LogDestinationWriter::Bytes(b) => {
                let guard = b.0.lock().unwrap();
                let b = guard.clone();

                from_utf8(b.as_slice()).unwrap().to_string()
            }
            LogDestinationWriter::RotationWriter(w) => {
                let paths = w.0.lock().unwrap().log_paths();
                let path = paths.first().expect("exactly one path excepted");
                fs::read_to_string(path).unwrap()
            }
            _ => todo!(),
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
        let path = PathBuf::from(&config.destination.to_string());

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
        let path = PathBuf::from(&config.destination.to_string());

        let opened_path = path.clone();
        let open_fn = move || {
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(mode)
                .open(&opened_path)
        };

        Reopen::new(Box::new(open_fn)).map_err(|e| Error::FileCreate(path, e))
    }
}

impl TryFrom<&LogConfig> for LogDestinationWriter {
    type Error = Error;

    fn try_from(value: &LogConfig) -> Result<Self, Self::Error> {
        let destination = match &value.destination {
            LogDestination::StdOut => LogDestinationWriter::StdOut,
            LogDestination::StdErr => LogDestinationWriter::StdErr,
            #[cfg(test)]
            LogDestination::Bytes => LogDestinationWriter::Bytes(Vec::<u8>::new().into()),
            LogDestination::File(path_string) => {
                let path = PathBuf::from(path_string);

                validate_log_path(path)?;

                if value.max_files > 0 {
                    let file: FileRotate<AppendTimestamp> = FileRotate::try_from(value)?;
                    LogDestinationWriter::RotationWriter(file.into())
                } else {
                    let file: Reopen<File> = value.try_into()?;
                    LogDestinationWriter::File(file.into())
                }
            }
        };

        Ok(destination)
    }
}

impl Debug for LogDestinationWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

/// Whenever we want to write to log destination, we delegate to the Writer implementation
impl Write for LogDestinationWriter {
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

/// Verify log file path.
///
/// Ensure that the log file path is correct, that is:
/// - it points to a file, not a directory
/// - if the log file exists, it is writable for the current user
/// - parent directory of the file exists and is writable for current user
/// - path is absolute
fn validate_log_path<T: AsRef<Path>>(log_file_path: T) -> Result<(), Error> {
    let log_file_path = log_file_path.as_ref();

    if !log_file_path.is_absolute() {
        return Err(Error::FilePath(
            log_file_path.to_owned(),
            "log file path must be absolute".to_string(),
        ));
    }

    if log_file_path.exists() {
        // Make sure log file is writable
        if log_file_path.is_dir() {
            return Err(Error::FilePath(
                log_file_path.to_owned(),
                "log file path must point to file".to_string(),
            ));
        }

        let md = fs::metadata(log_file_path).map_err(|e| {
            Error::FilePath(
                log_file_path.to_owned(),
                format!("cannot read log file metadata: {}", e),
            )
        })?;

        if md.permissions().readonly() {
            return Err(Error::FilePath(
                log_file_path.to_owned(),
                "log file is readonly".to_string(),
            ));
        }
    } else if log_file_path.ends_with(String::from(path::MAIN_SEPARATOR)) {
        // If file doesn't exist we need to do at least simple validation
        return Err(Error::FilePath(
            log_file_path.to_owned(),
            "log file path must point to file".to_string(),
        ));
    }

    // Make sure parent directly is writable so log rotation can work
    let parent_dir = log_file_path
        .parent()
        .expect("absolute log file path will always have parent");

    let md = fs::metadata(parent_dir).map_err(|e| {
        Error::FilePath(
            log_file_path.to_owned(),
            format!("cannot read parent directory: {}", e),
        )
    })?;

    let permissions = md.permissions();
    if permissions.readonly() {
        return Err(Error::FilePath(
            log_file_path.to_owned(),
            "parent directory is readonly".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::fs::OpenOptions;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_validate_log_path_file_exists_but_readonly() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("log.txt");
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .unwrap();
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&file_path, perms).unwrap();

        assert!(
            matches!(validate_log_path(&file_path), Err(Error::FilePath(_, message)) if message == "log file is readonly")
        );
    }

    #[test]
    fn test_validate_log_path_parent_directory_not_writable() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("log.txt");
        let mut perms = fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(dir.path(), perms).unwrap();

        assert!(
            matches!(validate_log_path(file_path), Err(Error::FilePath(_, message)) if message == "parent directory is readonly")
        );
    }

    #[test]
    fn test_validate_log_path_points_to_directory() {
        let dir = tempdir().unwrap();

        assert!(
            matches!(validate_log_path(dir.path()), Err(Error::FilePath(_, message)) if message == "log file path must point to file")
        );
    }

    #[test]
    fn test_validate_log_path_not_absolute() {
        let relative_path = Path::new("log.txt");

        assert!(
            matches!(validate_log_path(relative_path), Err(Error::FilePath(_, message)) if message == "log file path must be absolute")
        );
    }

    #[test]
    fn test_validate_log_path_file_exists_and_writable() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("log.txt");
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .unwrap();

        assert!(validate_log_path(&file_path).is_ok());
    }
}
