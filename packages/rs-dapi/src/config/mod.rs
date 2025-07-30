use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration for ports and network binding
    pub server: ServerConfig,
    /// DAPI-specific configuration for blockchain integration
    pub dapi: DapiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Port for the main gRPC API server
    pub grpc_api_port: u16,
    /// Port for gRPC streaming endpoints
    pub grpc_streams_port: u16,
    /// Port for JSON-RPC API server
    pub json_rpc_port: u16,
    /// Port for REST gateway server
    pub rest_gateway_port: u16,
    /// Port for health check endpoints
    pub health_check_port: u16,
    /// IP address to bind all servers to
    pub bind_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapiConfig {
    /// Whether to enable REST API endpoints
    pub enable_rest: bool,
    /// Drive (storage layer) client configuration
    pub drive: DriveConfig,
    /// Tenderdash (consensus layer) client configuration
    pub tenderdash: TenderdashConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveConfig {
    /// URI for connecting to the Drive service
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenderdashConfig {
    /// URI for connecting to the Tenderdash consensus service
    pub uri: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                grpc_api_port: 3005,
                grpc_streams_port: 3006,
                json_rpc_port: 3004,
                rest_gateway_port: 8080,
                health_check_port: 9090,
                bind_address: "127.0.0.1".to_string(),
            },
            dapi: DapiConfig {
                enable_rest: false,
                drive: DriveConfig {
                    uri: "http://127.0.0.1:6000".to_string(),
                },
                tenderdash: TenderdashConfig {
                    uri: "http://127.0.0.1:26657".to_string(),
                },
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(port) = std::env::var("DAPI_GRPC_SERVER_PORT") {
            config.server.grpc_api_port = port.parse()?;
        }
        if let Ok(port) = std::env::var("DAPI_GRPC_STREAMS_PORT") {
            config.server.grpc_streams_port = port.parse()?;
        }
        if let Ok(port) = std::env::var("DAPI_JSON_RPC_PORT") {
            config.server.json_rpc_port = port.parse()?;
        }
        if let Ok(port) = std::env::var("DAPI_REST_GATEWAY_PORT") {
            config.server.rest_gateway_port = port.parse()?;
        }
        if let Ok(port) = std::env::var("DAPI_HEALTH_CHECK_PORT") {
            config.server.health_check_port = port.parse()?;
        }
        if let Ok(addr) = std::env::var("DAPI_BIND_ADDRESS") {
            config.server.bind_address = addr;
        }
        if let Ok(enable_rest) = std::env::var("DAPI_ENABLE_REST") {
            config.dapi.enable_rest = enable_rest.parse().unwrap_or(false);
        }
        if let Ok(drive_uri) = std::env::var("DAPI_DRIVE_URI") {
            config.dapi.drive.uri = drive_uri;
        }
        if let Ok(tenderdash_uri) = std::env::var("DAPI_TENDERDASH_URI") {
            config.dapi.tenderdash.uri = tenderdash_uri;
        }

        Ok(config)
    }

    pub fn grpc_api_addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.bind_address, self.server.grpc_api_port)
            .parse()
            .expect("Invalid gRPC API address")
    }

    pub fn grpc_streams_addr(&self) -> SocketAddr {
        format!(
            "{}:{}",
            self.server.bind_address, self.server.grpc_streams_port
        )
        .parse()
        .expect("Invalid gRPC streams address")
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

    pub fn health_check_addr(&self) -> SocketAddr {
        format!(
            "{}:{}",
            self.server.bind_address, self.server.health_check_port
        )
        .parse()
        .expect("Invalid health check address")
    }
}

#[cfg(test)]
mod tests;
