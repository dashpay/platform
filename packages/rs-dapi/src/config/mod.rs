use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, path::PathBuf};
use tracing::{debug, trace, warn};

use crate::{DAPIResult, DapiError};

mod utils;
use utils::{from_str_or_bool, from_str_or_number};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Server configuration for ports and network binding
    #[serde(flatten)]
    pub server: ServerConfig,
    /// DAPI-specific configuration for blockchain integration
    #[serde(flatten)]
    pub dapi: DapiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Port for the unified gRPC server (all services: Core, Platform, Streams)
    #[serde(
        rename = "dapi_grpc_server_port",
        deserialize_with = "from_str_or_number"
    )]
    pub grpc_server_port: u16,
    /// Port for JSON-RPC API server
    #[serde(rename = "dapi_json_rpc_port", deserialize_with = "from_str_or_number")]
    pub json_rpc_port: u16,
    /// Port for metrics and health endpoints
    #[serde(rename = "dapi_metrics_port", deserialize_with = "from_str_or_number")]
    pub metrics_port: u16,
    /// IP address to bind all servers to
    #[serde(rename = "dapi_bind_address")]
    pub bind_address: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            grpc_server_port: 3005,
            json_rpc_port: 3004,
            metrics_port: 9090,
            bind_address: "127.0.0.1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DapiConfig {
    /// Drive (storage layer) client configuration
    #[serde(flatten)]
    pub drive: DriveConfig,
    /// Tenderdash (consensus layer) client configuration
    #[serde(flatten)]
    pub tenderdash: TenderdashConfig,
    /// Dash Core configuration for blockchain data
    #[serde(flatten)]
    pub core: CoreConfig,
    /// Memory budget for cached Platform API responses (bytes)
    #[serde(
        rename = "dapi_platform_cache_bytes",
        deserialize_with = "from_str_or_number"
    )]
    pub platform_cache_bytes: u64,
    /// Timeout for waiting for state transition results (in milliseconds)
    #[serde(
        rename = "dapi_state_transition_wait_timeout",
        deserialize_with = "from_str_or_number"
    )]
    pub state_transition_wait_timeout: u64,
    /// Logging configuration
    #[serde(flatten)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DriveConfig {
    /// URI for connecting to the Drive service
    #[serde(rename = "dapi_drive_uri")]
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TenderdashConfig {
    /// URI for connecting to the Tenderdash consensus service (HTTP RPC)
    #[serde(rename = "dapi_tenderdash_uri")]
    pub uri: String,
    /// WebSocket URI for real-time events from Tenderdash
    #[serde(rename = "dapi_tenderdash_websocket_uri")]
    pub websocket_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// ZMQ URI for receiving real-time blockchain events from Dash Core
    #[serde(rename = "dapi_core_zmq_url")]
    pub zmq_url: String,
    /// JSON-RPC URL for Dash Core RPC (e.g., http://127.0.0.1:9998)
    #[serde(rename = "dapi_core_rpc_url")]
    pub rpc_url: String,
    /// Dash Core RPC username
    #[serde(rename = "dapi_core_rpc_user")]
    pub rpc_user: String,
    /// Dash Core RPC password
    #[serde(rename = "dapi_core_rpc_pass")]
    pub rpc_pass: String,
    /// Memory budget for cached Core RPC responses (bytes)
    #[serde(
        rename = "dapi_core_cache_bytes",
        deserialize_with = "from_str_or_number"
    )]
    pub cache_bytes: u64,
}

impl Default for DapiConfig {
    fn default() -> Self {
        Self {
            drive: DriveConfig::default(),
            tenderdash: TenderdashConfig::default(),
            core: CoreConfig::default(),
            platform_cache_bytes: 2 * 1024 * 1024,
            state_transition_wait_timeout: 30000, // 30 seconds default
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for DriveConfig {
    fn default() -> Self {
        Self {
            uri: "http://127.0.0.1:6000".to_string(),
        }
    }
}

impl Default for TenderdashConfig {
    fn default() -> Self {
        Self {
            uri: "http://127.0.0.1:26657".to_string(),
            websocket_uri: "ws://127.0.0.1:26657/websocket".to_string(),
        }
    }
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            zmq_url: "tcp://127.0.0.1:29998".to_string(),
            rpc_url: "http://127.0.0.1:9998".to_string(),
            rpc_user: String::new(),
            rpc_pass: String::new(),
            cache_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Main application log level or explicit RUST_LOG filter string
    #[serde(rename = "dapi_logging_level")]
    pub level: String,
    /// Enable structured JSON logging for application logs
    #[serde(
        rename = "dapi_logging_json_format",
        deserialize_with = "from_str_or_bool"
    )]
    pub json_format: bool,
    /// Path to access log file. If set to non-empty value, access logging is enabled.
    #[serde(rename = "dapi_logging_access_log_path")]
    pub access_log_path: Option<String>,
    /// Access log format. Currently supports "combined" (Apache Common Log Format)
    #[serde(rename = "dapi_logging_access_log_format")]
    pub access_log_format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            access_log_path: None,
            access_log_format: "combined".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables and .env file
    pub fn load() -> DAPIResult<Self> {
        let config = Self::from_env().map_err(|e| {
            DapiError::Configuration(format!("Failed to load configuration: {}", e))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Populate configuration from environment variables using `envy`.
    fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    /// Load configuration from specific .env file and environment variables
    pub fn load_from_dotenv(config_path: Option<PathBuf>) -> DAPIResult<Self> {
        Self::load_with_overrides(config_path, std::iter::empty::<(String, String)>())
    }

    /// Load configuration applying defaults, .env, environment variables, and CLI overrides (in that order).
    pub fn load_with_overrides<I, K, V>(
        config_path: Option<PathBuf>,
        cli_overrides: I,
    ) -> DAPIResult<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        trace!("Loading configuration from .env file, environment, and CLI overrides");

        // Collect configuration values from layered sources
        let mut layered: HashMap<String, String> = HashMap::new();

        if let Some(path) = config_path {
            match dotenvy::from_path_iter(&path) {
                Ok(iter) => {
                    for entry in iter {
                        let (key, value) = entry.map_err(|e| {
                            DapiError::Configuration(format!(
                                "Cannot parse config file {:?}: {}",
                                path, e
                            ))
                        })?;
                        layered.insert(key, value);
                    }
                    debug!("Loaded .env file from: {:?}", path);
                }
                Err(e) => {
                    return Err(DapiError::Configuration(format!(
                        "Cannot load config file {:?}: {}",
                        path, e
                    )));
                }
            }
        } else {
            match dotenvy::dotenv_iter() {
                Ok(iter) => {
                    for entry in iter {
                        let (key, value) = entry.map_err(|e| {
                            DapiError::Configuration(format!(
                                "Cannot parse config file entry: {}",
                                e
                            ))
                        })?;
                        layered.insert(key, value);
                    }
                    debug!("Loaded .env file from default location");
                }
                Err(e) => {
                    if e.not_found() {
                        warn!("Cannot find any matching .env file");
                    } else {
                        return Err(DapiError::Configuration(format!(
                            "Cannot load config file: {}",
                            e
                        )));
                    }
                }
            }
        }

        // Environment variables override .env contents
        layered.extend(env::vars());

        // CLI overrides have the highest priority
        for (key, value) in cli_overrides.into_iter() {
            layered.insert(key.into(), value.into());
        }

        match envy::from_iter::<_, Self>(layered) {
            Ok(config) => {
                debug!("Configuration loaded successfully from layered sources");
                config.validate()?;
                Ok(config)
            }
            Err(e) => Err(DapiError::Configuration(format!(
                "Failed to load configuration: {}",
                e
            ))),
        }
    }

    /// Build the socket address for the unified gRPC endpoint.
    pub fn grpc_server_addr(&self) -> DAPIResult<SocketAddr> {
        format!(
            "{}:{}",
            self.server.bind_address, self.server.grpc_server_port
        )
        .parse()
        .map_err(|e| {
            DapiError::Configuration(format!(
                "Invalid gRPC server address '{}:{}': {}",
                self.server.bind_address, self.server.grpc_server_port, e
            ))
        })
    }

    /// Build the socket address for the JSON-RPC endpoint.
    pub fn json_rpc_addr(&self) -> DAPIResult<SocketAddr> {
        format!("{}:{}", self.server.bind_address, self.server.json_rpc_port)
            .parse()
            .map_err(|e| {
                DapiError::Configuration(format!(
                    "Invalid JSON-RPC address '{}:{}': {}",
                    self.server.bind_address, self.server.json_rpc_port, e
                ))
            })
    }

    /// Return the configured metrics listener port.
    pub fn metrics_port(&self) -> u16 {
        self.server.metrics_port
    }

    /// Determine whether metrics should be exposed (port non-zero).
    pub fn metrics_enabled(&self) -> bool {
        self.server.metrics_port != 0
    }

    /// Build the metrics socket address if metrics are enabled.
    pub fn metrics_addr(&self) -> DAPIResult<Option<SocketAddr>> {
        if !self.metrics_enabled() {
            return Ok(None);
        }

        format!("{}:{}", self.server.bind_address, self.server.metrics_port)
            .parse()
            .map(Some)
            .map_err(|e| {
                DapiError::Configuration(format!(
                    "Invalid metrics address '{}:{}': {}",
                    self.server.bind_address, self.server.metrics_port, e
                ))
            })
    }

    /// Validate configuration to ensure dependent subsystems can start successfully.
    pub fn validate(&self) -> DAPIResult<()> {
        self.grpc_server_addr()?;
        self.json_rpc_addr()?;
        self.metrics_addr()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
