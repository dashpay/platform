use super::Config;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper function to clean up all DAPI environment variables
fn cleanup_env_vars() {
    let env_vars = [
        "DAPI_GRPC_SERVER_PORT",
        "DAPI_GRPC_STREAMS_PORT",
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
    // Note: These will fail if no servers are running, which is expected in unit tests
    DriveClient::new(&config.dapi.drive.uri)
        .await
        .expect_err("DriveClient should fail if no server is running");
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
DAPI_GRPC_STREAMS_PORT=4006
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
    assert_eq!(config.grpc_server_addr().to_string(), "127.0.0.1:3005");
    assert_eq!(config.json_rpc_addr().to_string(), "127.0.0.1:3004");
    assert_eq!(config.metrics_addr().unwrap().to_string(), "127.0.0.1:9090");
}

#[test]
fn test_config_socket_addresses_custom_bind() {
    let mut config = Config::default();
    config.server.bind_address = "0.0.0.0".to_string();
    config.server.grpc_server_port = 4000;

    // Test that custom bind address and port work
    assert_eq!(config.grpc_server_addr().to_string(), "0.0.0.0:4000");
}

#[test]
fn test_metrics_disabled_when_port_zero() {
    let mut config = Config::default();
    config.server.metrics_port = 0;

    assert!(!config.metrics_enabled());
    assert!(config.metrics_addr().is_none());
}
