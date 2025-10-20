use super::{Config, ServerConfig};
use serial_test::serial;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper function to clean up all DAPI environment variables
fn cleanup_env_vars() {
    let env_vars = [
        "DAPI_GRPC_SERVER_PORT",
        "DAPI_JSON_RPC_PORT",
        "DAPI_METRICS_PORT",
        "DAPI_BIND_ADDRESS",
        "DAPI_DRIVE_URI",
        "DAPI_TENDERDASH_URI",
        "DAPI_TENDERDASH_WEBSOCKET_URI",
        "DAPI_CORE_ZMQ_URL",
        "DAPI_STATE_TRANSITION_WAIT_TIMEOUT",
    ];

    for var in &env_vars {
        remove_env_var(var);
    }
}

fn set_env_var(key: &str, value: &str) {
    // SAFETY: manipulating process environment is inherently unsafe when multiple
    // threads are running. Tests using these helpers are serialized to avoid races.
    unsafe {
        std::env::set_var(key, value);
    }
}

fn remove_env_var(key: &str) {
    // SAFETY: see set_env_var comment; tests are serialized.
    unsafe {
        std::env::remove_var(key);
    }
}

#[test]
fn test_default_config_uses_uris() {
    let config = Config::default();

    // Test that default config uses proper URIs
    assert_eq!(config.dapi.drive.uri, "http://127.0.0.1:6000");
    assert_eq!(config.dapi.tenderdash.uri, "http://127.0.0.1:26657");
}

#[test]
#[serial]
fn test_config_load_with_uri_env_vars() {
    // Set environment variables
    set_env_var("DAPI_DRIVE_URI", "http://custom-drive:8000");
    set_env_var("DAPI_TENDERDASH_URI", "http://custom-tenderdash:9000");

    let config = Config::load().expect("Config should load successfully");

    // Test that environment variables override defaults
    assert_eq!(config.dapi.drive.uri, "http://custom-drive:8000");
    assert_eq!(config.dapi.tenderdash.uri, "http://custom-tenderdash:9000");

    // Clean up
    remove_env_var("DAPI_DRIVE_URI");
    remove_env_var("DAPI_TENDERDASH_URI");
}

#[tokio::test]
async fn test_clients_can_be_created_with_uris() {
    use crate::clients::{DriveClient, TenderdashClient};

    let config = Config::default();

    // Test that clients can be created with URIs from config
    // These create lazy connections that may operate in degraded mode until upstreams respond
    DriveClient::new(&config.dapi.drive.uri)
        .await
        .expect("DriveClient should be constructed even if no server is running");
    TenderdashClient::new(
        &config.dapi.tenderdash.uri,
        &config.dapi.tenderdash.websocket_uri,
    )
    .await
    .expect_err("TenderdashClient should fail if no server is running");
}

#[test]
#[serial]
fn test_config_load_from_dotenv_file() {
    // Clean up any existing environment variables first
    cleanup_env_vars();

    // Create a temporary .env file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let env_content = r#"
# Test configuration
DAPI_GRPC_SERVER_PORT=4005
DAPI_JSON_RPC_PORT=4004
DAPI_METRICS_PORT=9091
DAPI_BIND_ADDRESS=0.0.0.0
DAPI_DRIVE_URI=http://test-drive:7000
DAPI_TENDERDASH_URI=http://test-tenderdash:8000
DAPI_TENDERDASH_WEBSOCKET_URI=ws://test-tenderdash:8000/websocket
DAPI_CORE_ZMQ_URL=tcp://test-core:30000
DAPI_STATE_TRANSITION_WAIT_TIMEOUT=45000
"#;

    fs::write(temp_file.path(), env_content).expect("Failed to write temp file");

    // Load config from the temp file
    let config = Config::load_from_dotenv(Some(temp_file.path().to_path_buf()))
        .expect("Config should load from dotenv file");

    // Verify all values were loaded correctly
    assert_eq!(config.server.grpc_server_port, 4005);
    assert_eq!(config.server.json_rpc_port, 4004);
    assert_eq!(config.server.metrics_port, 9091);
    assert_eq!(config.server.bind_address, "0.0.0.0");
    assert_eq!(config.dapi.drive.uri, "http://test-drive:7000");
    assert_eq!(config.dapi.tenderdash.uri, "http://test-tenderdash:8000");
    assert_eq!(
        config.dapi.tenderdash.websocket_uri,
        "ws://test-tenderdash:8000/websocket"
    );
    assert_eq!(config.dapi.core.zmq_url, "tcp://test-core:30000");
    assert_eq!(config.dapi.state_transition_wait_timeout, 45000);

    // Cleanup
    cleanup_env_vars();
}

#[test]
#[serial]
fn test_config_load_from_dotenv_file_partial() {
    // Clean up any existing environment variables first
    cleanup_env_vars();

    // Create a temporary .env file with only some values
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let env_content = r#"
# Partial test configuration
DAPI_GRPC_SERVER_PORT=5005
DAPI_DRIVE_URI=http://partial-drive:8000
"#;

    fs::write(temp_file.path(), env_content).expect("Failed to write temp file");

    // Load config from the temp file
    let config = Config::load_from_dotenv(Some(temp_file.path().to_path_buf()))
        .expect("Config should load from dotenv file");

    // Verify specified values were loaded
    assert_eq!(config.server.grpc_server_port, 5005);
    assert_eq!(config.dapi.drive.uri, "http://partial-drive:8000");

    // Verify defaults are used for unspecified values
    assert_eq!(config.dapi.tenderdash.uri, "http://127.0.0.1:26657"); // default
    assert_eq!(config.dapi.state_transition_wait_timeout, 30000); // default

    // Cleanup
    cleanup_env_vars();
}

#[test]
fn test_config_load_from_nonexistent_dotenv_file() {
    let nonexistent_path = PathBuf::from("/nonexistent/path/to/.env");

    // Should return an error for nonexistent file
    let result = Config::load_from_dotenv(Some(nonexistent_path));
    assert!(result.is_err());

    // Error message should mention the file path
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Cannot load config file"));
}

#[test]
fn test_server_config_address_with_port_ipv4_literal() {
    let mut server = ServerConfig::default();
    server.bind_address = "0.0.0.0".to_string();

    let addr = server
        .address_with_port(1234)
        .expect("IPv4 bind address should resolve");

    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    assert_eq!(addr.port(), 1234);
}

#[test]
fn test_server_config_address_with_port_ipv6_literal() {
    let mut server = ServerConfig::default();
    server.bind_address = "::1".to_string();

    let addr = server
        .address_with_port(4321)
        .expect("IPv6 bind address should resolve");

    assert_eq!(addr.ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
    assert_eq!(addr.port(), 4321);
}

#[test]
fn test_server_config_address_with_port_hostname() {
    let mut server = ServerConfig::default();
    server.bind_address = "localhost".to_string();

    let addr = server
        .address_with_port(8080)
        .expect("Hostname bind address should resolve");

    assert!(addr.ip().is_loopback());
    assert_eq!(addr.port(), 8080);
}

#[test]
fn test_server_config_rejects_port_in_bind_address() {
    let mut server = ServerConfig::default();
    server.bind_address = "127.0.0.1:9000".to_string();

    let err = server
        .address_with_port(5000)
        .expect_err("Port in bind address should be rejected");

    assert!(
        err.to_string()
            .contains("Bind address '127.0.0.1:9000' must not include a port")
    );
}

#[test]
fn test_server_config_invalid_bind_address() {
    let mut server = ServerConfig::default();
    server.bind_address = "invalid host".to_string();

    let err = server
        .address_with_port(6000)
        .expect_err("Invalid bind address should fail");

    assert!(
        err.to_string()
            .contains("Invalid bind address 'invalid host'")
    );
}

#[test]
#[serial]
fn test_config_load_from_dotenv_with_env_override() {
    // Clean up any existing environment variables first
    cleanup_env_vars();

    // Create a temporary .env file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let env_content = r#"
DAPI_GRPC_SERVER_PORT=6005
DAPI_DRIVE_URI=http://dotenv-drive:9000
"#;

    fs::write(temp_file.path(), env_content).expect("Failed to write temp file");

    // Set environment variables that should override .env file
    set_env_var("DAPI_GRPC_SERVER_PORT", "7005");
    set_env_var("DAPI_TENDERDASH_URI", "http://env-tenderdash:10000");

    // Load config from the temp file
    let config = Config::load_from_dotenv(Some(temp_file.path().to_path_buf()))
        .expect("Config should load from dotenv file");

    // Environment variables should override .env file values
    assert_eq!(config.server.grpc_server_port, 7005); // from env, not .env file
    assert_eq!(config.dapi.tenderdash.uri, "http://env-tenderdash:10000"); // from env

    // Values only in .env file should still be loaded
    assert_eq!(config.dapi.drive.uri, "http://dotenv-drive:9000"); // from .env file

    // Clean up environment variables
    cleanup_env_vars();
}

#[test]
#[serial]
fn test_config_load_with_cli_overrides() {
    // Ensure we start from a clean environment
    cleanup_env_vars();

    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let env_content = r#"
DAPI_GRPC_SERVER_PORT=6005
DAPI_DRIVE_URI=http://dotenv-drive:9000
"#;

    fs::write(temp_file.path(), env_content).expect("Failed to write temp file");

    set_env_var("DAPI_GRPC_SERVER_PORT", "7005");

    let overrides = [
        ("DAPI_GRPC_SERVER_PORT", "8005"),
        ("DAPI_TENDERDASH_URI", "http://cli-tenderdash:11000"),
    ];

    let config = Config::load_with_overrides(Some(temp_file.path().to_path_buf()), overrides)
        .expect("Config should load with CLI overrides");

    assert_eq!(config.server.grpc_server_port, 8005); // CLI override wins
    assert_eq!(config.dapi.tenderdash.uri, "http://cli-tenderdash:11000"); // CLI override
    assert_eq!(config.dapi.drive.uri, "http://dotenv-drive:9000"); // .env retains value for unset keys

    cleanup_env_vars();
}

#[test]
#[serial]
fn test_config_load_from_dotenv_invalid_values() {
    // Clean up any existing environment variables first
    cleanup_env_vars();

    // Create a temporary .env file with invalid port value
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let env_content = r#"
DAPI_GRPC_SERVER_PORT=not_a_number
DAPI_DRIVE_URI=http://test-drive:8000
"#;

    fs::write(temp_file.path(), env_content).expect("Failed to write temp file");

    // Loading should fail due to invalid port value
    let result = Config::load_from_dotenv(Some(temp_file.path().to_path_buf()));

    // Should either return error or fallback gracefully (depending on implementation)
    // The current implementation should fallback to manual loading which would fail
    match result {
        Ok(config) => {
            // If it succeeds, the invalid port should fallback to default
            assert_eq!(config.server.grpc_server_port, 3005); // default
            assert_eq!(config.dapi.drive.uri, "http://test-drive:8000"); // valid value should load
        }
        Err(_) => {
            // Error is also acceptable for invalid configuration
        }
    }

    // Cleanup
    cleanup_env_vars();
}

#[test]
fn test_config_socket_addresses() {
    let config = Config::default();

    // Test that socket addresses are properly formatted
    assert_eq!(
        config
            .grpc_server_addr()
            .expect("gRPC address should parse")
            .to_string(),
        "127.0.0.1:3005"
    );
    assert_eq!(
        config
            .json_rpc_addr()
            .expect("JSON-RPC address should parse")
            .to_string(),
        "127.0.0.1:3004"
    );
    assert_eq!(
        config
            .metrics_addr()
            .expect("metrics address should parse")
            .expect("metrics address should be present")
            .to_string(),
        "127.0.0.1:9090"
    );
}

#[test]
fn test_config_socket_addresses_custom_bind() {
    let mut config = Config::default();
    config.server.bind_address = "0.0.0.0".to_string();
    config.server.grpc_server_port = 4000;

    // Test that custom bind address and port work
    assert_eq!(
        config
            .grpc_server_addr()
            .expect("custom gRPC address should parse")
            .to_string(),
        "0.0.0.0:4000"
    );
}

#[test]
fn test_metrics_disabled_when_port_zero() {
    let mut config = Config::default();
    config.server.metrics_port = 0;

    assert!(!config.metrics_enabled());
    assert!(
        config
            .metrics_addr()
            .expect("metrics address check should succeed")
            .is_none()
    );
}

#[test]
fn test_validate_default_config_succeeds() {
    let config = Config::default();
    config
        .validate()
        .expect("Default configuration should be valid");
}

#[test]
fn test_validate_fails_on_invalid_bind_address() {
    let mut config = Config::default();
    config.server.bind_address = "invalid-address".to_string();

    let error = config
        .validate()
        .expect_err("Invalid bind address should fail validation");

    assert!(
        error
            .to_string()
            .contains("Invalid bind address 'invalid-address'"),
        "unexpected error message: {}",
        error
    );
}
