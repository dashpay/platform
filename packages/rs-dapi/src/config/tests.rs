use super::Config;

#[test]
fn test_default_config_uses_uris() {
    let config = Config::default();

    // Test that default config uses proper URIs
    assert_eq!(config.dapi.drive.uri, "http://127.0.0.1:6000");
    assert_eq!(config.dapi.tenderdash.uri, "http://127.0.0.1:26657");
}

#[test]
fn test_config_load_with_uri_env_vars() {
    // Set environment variables
    std::env::set_var("DAPI_DRIVE_URI", "http://custom-drive:8000");
    std::env::set_var("DAPI_TENDERDASH_URI", "http://custom-tenderdash:9000");

    let config = Config::load().expect("Config should load successfully");

    // Test that environment variables override defaults
    assert_eq!(config.dapi.drive.uri, "http://custom-drive:8000");
    assert_eq!(config.dapi.tenderdash.uri, "http://custom-tenderdash:9000");

    // Clean up
    std::env::remove_var("DAPI_DRIVE_URI");
    std::env::remove_var("DAPI_TENDERDASH_URI");
}

#[test]
fn test_clients_can_be_created_with_uris() {
    use crate::clients::{DriveClient, TenderdashClient};

    let config = Config::default();

    // Test that clients can be created with URIs from config
    let _drive_client = DriveClient::new(&config.dapi.drive.uri);
    let _tenderdash_client = TenderdashClient::new(&config.dapi.tenderdash.uri);

    // Test passes if no panic occurs during client creation
}
