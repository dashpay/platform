//! Structured logging utilities for debugging
//!
//! This module provides logging functionality that can be enabled/disabled
//! at compile time for debugging verification operations.

#![allow(dead_code)]

use wasm_bindgen::prelude::*;

/// Log levels for structured logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Detailed trace information
    Trace,
    /// Debug information
    Debug,
    /// Informational messages
    Info,
    /// Warning messages
    Warn,
    /// Error messages
    Error,
}

/// Structured log entry
pub struct LogEntry {
    level: LogLevel,
    module: &'static str,
    message: String,
    context: Option<JsValue>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, module: &'static str, message: String) -> Self {
        Self {
            level,
            module,
            message,
            context: None,
        }
    }

    /// Add context to the log entry
    pub fn with_context(mut self, context: JsValue) -> Self {
        self.context = Some(context);
        self
    }

    /// Output the log entry to the console
    pub fn log(self) {
        #[cfg(feature = "debug_logs")]
        {
            let prefix = format!("[{}] {}: ", self.module, self.level_str());

            match self.level {
                LogLevel::Trace | LogLevel::Debug => {
                    if let Some(ctx) = self.context {
                        web_sys::console::debug_3(&prefix.into(), &self.message.into(), &ctx);
                    } else {
                        web_sys::console::debug_2(&prefix.into(), &self.message.into());
                    }
                }
                LogLevel::Info => {
                    if let Some(ctx) = self.context {
                        web_sys::console::info_3(&prefix.into(), &self.message.into(), &ctx);
                    } else {
                        web_sys::console::info_2(&prefix.into(), &self.message.into());
                    }
                }
                LogLevel::Warn => {
                    if let Some(ctx) = self.context {
                        web_sys::console::warn_3(&prefix.into(), &self.message.into(), &ctx);
                    } else {
                        web_sys::console::warn_2(&prefix.into(), &self.message.into());
                    }
                }
                LogLevel::Error => {
                    if let Some(ctx) = self.context {
                        web_sys::console::error_3(&prefix.into(), &self.message.into(), &ctx);
                    } else {
                        web_sys::console::error_2(&prefix.into(), &self.message.into());
                    }
                }
            }
        }
    }

    fn level_str(&self) -> &'static str {
        match self.level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Log a trace message
#[inline]
pub fn trace(module: &'static str, message: impl Into<String>) {
    LogEntry::new(LogLevel::Trace, module, message.into()).log();
}

/// Log a debug message
#[inline]
pub fn debug(module: &'static str, message: impl Into<String>) {
    LogEntry::new(LogLevel::Debug, module, message.into()).log();
}

/// Log an info message
#[inline]
pub fn info(module: &'static str, message: impl Into<String>) {
    LogEntry::new(LogLevel::Info, module, message.into()).log();
}

/// Log a warning message
#[inline]
pub fn warn(module: &'static str, message: impl Into<String>) {
    LogEntry::new(LogLevel::Warn, module, message.into()).log();
}

/// Log an error message
#[inline]
pub fn error(module: &'static str, message: impl Into<String>) {
    LogEntry::new(LogLevel::Error, module, message.into()).log();
}

/// Log a message with context
#[inline]
pub fn log_with_context(
    level: LogLevel,
    module: &'static str,
    message: impl Into<String>,
    context: JsValue,
) {
    LogEntry::new(level, module, message.into())
        .with_context(context)
        .log();
}

/// Macro for conditional logging
#[macro_export]
macro_rules! log_debug {
    ($module:expr, $msg:expr) => {
        $crate::utils::logging::debug($module, $msg)
    };
    ($module:expr, $msg:expr, $ctx:expr) => {
        $crate::utils::logging::log_with_context(
            $crate::utils::logging::LogLevel::Debug,
            $module,
            $msg,
            $ctx,
        )
    };
}

/// Macro for error logging
#[macro_export]
macro_rules! log_error {
    ($module:expr, $msg:expr) => {
        $crate::utils::logging::error($module, $msg)
    };
    ($module:expr, $msg:expr, $ctx:expr) => {
        $crate::utils::logging::log_with_context(
            $crate::utils::logging::LogLevel::Error,
            $module,
            $msg,
            $ctx,
        )
    };
}

/// Performance logging helper
pub struct PerfLogger {
    module: &'static str,
    operation: String,
    #[cfg(all(target_arch = "wasm32", feature = "debug_logs"))]
    start_time: f64,
}

impl PerfLogger {
    /// Start performance logging
    pub fn new(module: &'static str, operation: impl Into<String>) -> Self {
        let operation_str = operation.into();

        #[cfg(all(target_arch = "wasm32", feature = "debug_logs"))]
        {
            let start_time = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);

            debug(module, format!("Starting: {}", &operation_str));

            Self {
                module,
                operation: operation_str,
                start_time,
            }
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "debug_logs")))]
        Self {
            module,
            operation: operation_str,
        }
    }

    /// Complete performance logging
    pub fn complete(self) {
        #[cfg(all(target_arch = "wasm32", feature = "debug_logs"))]
        {
            let end_time = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);

            let duration = end_time - self.start_time;
            debug(
                self.module,
                format!("Completed: {} (took {:.2}ms)", self.operation, duration),
            );
        }
    }
}
