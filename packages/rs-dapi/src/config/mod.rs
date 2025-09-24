use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};
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
    /// Port for REST gateway server
    #[serde(
        rename = "dapi_rest_gateway_port",
        deserialize_with = "from_str_or_number"
    )]
    pub rest_gateway_port: u16,
    /// Port for metrics and health endpoints
    #[serde(
        rename = "dapi_metrics_port",
        deserialize_with = "from_str_or_number"
    )]
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
            rest_gateway_port: 8080,
            metrics_port: 9090,
            bind_address: "127.0.0.1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DapiConfig {
    /// Whether to enable REST API endpoints
    #[serde(rename = "dapi_enable_rest", deserialize_with = "from_str_or_bool")]
    pub enable_rest: bool,
    /// Drive (storage layer) client configuration
    #[serde(flatten)]
    pub drive: DriveConfig,
    /// Tenderdash (consensus layer) client configuration
    #[serde(flatten)]
    pub tenderdash: TenderdashConfig,
    /// Dash Core configuration for blockchain data
    #[serde(flatten)]
    pub core: CoreConfig,
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
}

impl Default for DapiConfig {
    fn default() -> Self {
        Self {
            enable_rest: false,
            drive: DriveConfig::default(),
            tenderdash: TenderdashConfig::default(),
            core: CoreConfig::default(),
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Main application log level; TODO: not supported yet
    #[serde(rename = "dapi_logging_level")]
    pub level: String,
    /// Enable structured JSON logging for application logs
    #[serde(
        rename = "dapi_logging_json_format",
        deserialize_with = "from_str_or_bool"
    )]
    pub json_format: bool,
    /// Path to access log file. If set to non-empty value, access logging is enabled.
    /// TODO: Implement access logging
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
        Self::from_env()
            .map_err(|e| DapiError::Configuration(format!("Failed to load configuration: {}", e)))
    }

    fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    /// Load configuration from specific .env file and environment variables
    pub fn load_from_dotenv(config_path: Option<PathBuf>) -> DAPIResult<Self> {
        trace!("Loading configuration from .env file and environment");

        // Load .env file first
        if let Some(path) = config_path {
            if let Err(e) = dotenvy::from_path(&path) {
                return Err(DapiError::Configuration(format!(
                    "Cannot load config file {:?}: {}",
                    path, e
                )));
            }
            debug!("Loaded .env file from: {:?}", path);
        } else if let Err(e) = dotenvy::dotenv() {
            if e.not_found() {
                warn!("Cannot find any matching .env file");
            } else {
                return Err(DapiError::Configuration(format!(
                    "Cannot load config file: {}",
                    e
                )));
            }
        }

        // Try loading from environment with envy
        match Self::from_env() {
            Ok(config) => {
                debug!("Configuration loaded successfully from environment");
                Ok(config)
            }
            Err(e) => {
                // Fall back to manual loading if envy fails
                debug!("Falling back to manual configuration loading: {}", e);
                Self::load()
            }
        }
    }

    pub fn grpc_server_addr(&self) -> SocketAddr {
        format!(
            "{}:{}",
            self.server.bind_address, self.server.grpc_server_port
        )
        .parse()
        .expect("Invalid gRPC server address")
    }

    pub fn json_rpc_addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.bind_address, self.server.json_rpc_port)
            .parse()
            .expect("Invalid JSON-RPC address")
    }

    pub fn rest_gateway_addr(&self) -> SocketAddr {
        format!(
            "{}:{}",
            self.server.bind_address, self.server.rest_gateway_port
        )
        .parse()
        .expect("Invalid REST gateway address")
    }

    pub fn metrics_port(&self) -> u16 {
        self.server.metrics_port
    }

    pub fn metrics_enabled(&self) -> bool {
        self.server.metrics_port != 0
    }

    pub fn metrics_addr(&self) -> Option<SocketAddr> {
        if !self.metrics_enabled() {
            return None;
        }

        Some(
            format!(
                "{}:{}",
                self.server.bind_address, self.server.metrics_port
            )
            .parse()
            .expect("Invalid metrics address"),
        )
    }
}

#[cfg(test)]
mod tests;
