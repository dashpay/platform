mod config;
mod destination;
mod error;
mod format;
mod level;
mod logger;

pub use config::LogConfig;
pub use config::LogConfigs;
pub use destination::LogDestination;
pub use error::Error;
pub use format::LogFormat;
pub use level::LogLevel;
pub use logger::LogBuilder;
pub use logger::Loggers;

/// Helper that initializes logging in unit tests
///
///
/// For verbosity, see drive-abci --help or use 0 or 5
pub fn init_for_tests(level: LogLevel) {
    let mut logger_builder = LogBuilder::new();
    let config = LogConfig {
        destination: LogDestination::StdOut,
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

#[cfg(test)]
mod tests {
    use super::*;

    use itertools::Itertools;
    use std::{cmp::Ordering, fs};
    use tempfile::TempDir;

    /// Test that multiple loggers can work independently, with different log levels.
    ///
    /// Note that, due to limitation of [tracing::subscriber::set_global_default()], we can only have one test.
    #[test]
    fn test_logging() {
        let logger_stdout = LogConfig {
            destination: LogDestination::StdOut,
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            ..Default::default()
        };

        let logger_stderr = LogConfig {
            destination: LogDestination::StdErr,
            level: LogLevel::Debug,
            ..Default::default()
        };

        let logger_v0 = LogConfig {
            destination: LogDestination::Bytes,
            level: LogLevel::Info,
            ..Default::default()
        };

        let logger_v4 = LogConfig {
            destination: LogDestination::Bytes,
            level: LogLevel::Debug,
            format: LogFormat::Json,
            ..Default::default()
        };

        let dir = TempDir::new().unwrap();

        let file_v0_path = dir.path().join("log.v0");
        let logger_file_v0 = LogConfig {
            destination: LogDestination::File(file_v0_path),
            level: LogLevel::Info,
            max_files: 4,
            ..Default::default()
        };

        let file_v4_path = dir.path().join("log.v4");
        let logger_file_v4 = LogConfig {
            destination: LogDestination::File(file_v4_path.clone()),
            level: LogLevel::Debug,
            max_files: 0, // no rotation
            ..Default::default()
        };

        let loggers = LogBuilder::new()
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

        let dispatch = loggers.as_subscriber().expect("subscriber failed");
        let _guard = tracing::dispatcher::set_default(&dispatch);

        const TEST_STRING_DEBUG: &str = "testing debug trace";
        const TEST_STRING_ERROR: &str = "testing error trace";
        tracing::error!(TEST_STRING_ERROR);
        tracing::debug!(TEST_STRING_DEBUG);

        loggers.flush().unwrap();
        loggers.rotate().unwrap();

        // CHECK ASSERTIONS

        let result_verb_0 = loggers
            .get("v0")
            .expect("should return logger")
            .destination
            .lock()
            .unwrap()
            .read_as_string();

        let result_verb_4 = loggers
            .get("v4")
            .expect("should return logger")
            .destination
            .lock()
            .unwrap()
            .read_as_string();

        let result_dir_verb_0 = loggers
            .get("file_v0")
            .expect("should return logger")
            .destination
            .lock()
            .unwrap()
            .read_as_string();

        let result_file_verb_4 = fs::read_to_string(&file_v4_path)
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
        let temp_dir = TempDir::new().unwrap();
        let filepath = temp_dir.path().join("drive-abci.log");
        let config = LogConfig {
            destination: LogDestination::File(filepath),
            level: LogLevel::Trace,
            format: LogFormat::Pretty,
            max_files: 3,
            ..Default::default()
        };

        let loggers = LogBuilder::new()
            .with_config("rotate", &config)
            .expect("configure log builder")
            .build();
        let logger = loggers.get("rotate").expect("get logger");

        for i in 0..config.max_files + 2 {
            logger
                .destination
                .lock()
                .unwrap()
                .to_write()
                .write_all(format!("file {}\n", i).as_bytes())
                .unwrap();

            loggers.rotate().expect("rotate logs");

            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        let mut counter = 0;
        temp_dir.path().read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let path = path.to_string_lossy();
            println!("{}", path);
            assert!(path.contains("drive-abci.log"));
            counter += 1;
        });
        assert_eq!(counter, config.max_files + 1);
    }

    // TODO: Not passing on Mac OS
    /// Test rotation of File destination.
    ///
    /// Given that we move the File and then Rotate it, we expect the file to be recreated in new location.
    #[ignore]
    #[test]
    fn test_file_rotate() {
        const ITERATIONS: usize = 4;

        let temp_dir = TempDir::new().unwrap();
        let filepath = temp_dir.path().join("drive-abci.log");
        let config = LogConfig {
            destination: LogDestination::File(filepath.clone()),
            level: LogLevel::Trace,
            format: LogFormat::Pretty,
            max_files: 0,
            ..Default::default()
        };

        let loggers = LogBuilder::new()
            .with_config("rotate", &config)
            .expect("configure log builder")
            .build();
        let logger = loggers.get("rotate").expect("get logger");

        for i in 0..ITERATIONS {
            let guard = logger.destination.lock().unwrap();
            guard
                .to_write()
                .write_all(format!("file {}, before rotate\n", i).as_bytes())
                .unwrap();

            fs::rename(
                &filepath,
                temp_dir.path().join(format!("drive-abci.log.{}", i)),
            )
            .unwrap();
            // rotate() locks, so we need to drop guard here
            drop(guard);

            loggers.rotate().expect("rotate logs");
            let guard = logger.destination.lock().unwrap();
            guard
                .to_write()
                .write_all(format!("file {}, after rotate\n", i + 1).as_bytes())
                .unwrap();
            guard.to_write().flush().unwrap();

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Close all files, so that we can read them
        drop(loggers);

        let mut counter = 0;
        temp_dir
            .path()
            .read_dir()
            .unwrap()
            .sorted_by(|a, b| {
                let a = a.as_ref().unwrap().path();
                let b = b.as_ref().unwrap().path();
                if a.eq(&b) {
                    return Ordering::Equal;
                }
                if a.ends_with("drive-abci.log") {
                    return Ordering::Greater;
                }
                if b.ends_with("drive-abci.log") {
                    return Ordering::Greater;
                }

                a.cmp(&b)
            })
            .for_each(|entry| {
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

                counter += 1;
            });
        assert_eq!(counter, ITERATIONS + 1);
    }
}
