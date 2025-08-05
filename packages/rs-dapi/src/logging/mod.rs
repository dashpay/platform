//! Logging infrastructure for rs-dapi
//!
//! This module provides structured logging with access logging in standard formats,
//! and log rotation support.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

use crate::config::LoggingConfig;

pub mod access_log;
pub mod middleware;

pub use access_log::{AccessLogEntry, AccessLogger};
pub use middleware::AccessLogLayer;

/// Initialize logging subsystem with given configuration
/// Returns Some(AccessLogger) if access logging is configured with a non-empty path, None otherwise
pub async fn init_logging(
    config: &LoggingConfig,
    cli_config: &LoggingCliConfig,
) -> Result<Option<AccessLogger>, String> {
    // Set up the main application logger
    setup_application_logging(config, cli_config)?;

    // Set up access logging if configured with a non-empty path
    let access_logger = if let Some(ref path) = config.access_log_path {
        if !path.trim().is_empty() {
            Some(
                AccessLogger::new(path.clone())
                    .await
                    .map_err(|e| format!("Failed to create access logger: {}", e))?,
            )
        } else {
            None
        }
    } else {
        None
    };

    Ok(access_logger)
}

fn setup_application_logging(
    config: &LoggingConfig,
    cli_config: &LoggingCliConfig,
) -> Result<(), String> {
    use tracing_subscriber::{filter::EnvFilter, fmt};

    // Determine log level based on verbose flags
    let env_filter = if cli_config.debug || cli_config.verbose > 0 {
        match cli_config.verbose.max(if cli_config.debug { 2 } else { 0 }) {
            1 => "rs_dapi=debug,tower_http::trace=debug,info", // -v: debug from rs-dapi, info from others
            2 => "rs_dapi=trace,tower_http::trace=debug,info", // -vv or --debug: trace from rs-dapi, debug from others
            3 => "rs_dapi=trace,tower_http::trace=trace,h2=info,tower=info,hyper_util=info,debug", // -vvv
            4 => "rs_dapi=trace,tower_http::trace=trace,debug", // -vvvv
            _ => "rs_dapi=trace,trace",                         // -vvvvv+
        }
    } else {
        // Use RUST_LOG if set, otherwise default
        &std::env::var("RUST_LOG").unwrap_or_else(|_| "rs_dapi=info,warn".to_string())
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(env_filter))
        .map_err(|e| format!("Invalid log filter: {}", e))?;

    let registry = Registry::default().with(env_filter);

    if config.json_format {
        // JSON structured logging
        let fmt_layer = fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(false)
            .with_ansi(cli_config.color.unwrap_or(false));

        registry.with(fmt_layer).init();
    } else {
        // Human-readable logging
        let fmt_layer = fmt::layer().with_ansi(cli_config.color.unwrap_or(true));

        registry.with(fmt_layer).init();
    }

    Ok(())
}

// CLI configuration structure for compatibility
pub struct LoggingCliConfig {
    pub verbose: u8,
    pub debug: bool,
    pub color: Option<bool>,
}
