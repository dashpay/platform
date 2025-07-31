// Integration tests for rs-dapi streaming service

use rs_dapi::clients::mock::{MockDriveClient, MockTenderdashClient};
use rs_dapi::config::Config;
use rs_dapi::services::CoreServiceImpl;
use std::sync::Arc;

#[tokio::test]
async fn test_streaming_service_integration() {
    let config = Arc::new(Config::default());
    let drive_client = Arc::new(MockDriveClient::new());
    let tenderdash_client = Arc::new(MockTenderdashClient::new());

    // Create core service with streaming service
    let core_service = CoreServiceImpl::new(drive_client, tenderdash_client, config);

    // Test that we can create the service successfully
    assert!(
        core_service
            .streaming_service
            .subscriber_manager
            .subscription_count()
            .await
            == 0
    );

    // Test that streaming service initialization works
    // Note: We can't actually start the streaming service in a test without a real ZMQ connection
    // but we can verify the structure is correct
    assert!(!core_service.config.dapi.core.zmq_url.is_empty());
}

#[tokio::test]
async fn test_config_loading() {
    let config = Config::default();
    
    // Test default configuration values
    assert_eq!(config.server.grpc_api_port, 3005);
    assert_eq!(config.server.grpc_streams_port, 3006);
    assert_eq!(config.dapi.core.zmq_url, "tcp://127.0.0.1:29998");
    assert_eq!(config.server.bind_address, "127.0.0.1");
}

#[tokio::test]
async fn test_server_creation() {
    let config = Config::default();
    
    // Test that we can create a DapiServer successfully
    let server_result = rs_dapi::server::DapiServer::new(config).await;
    assert!(server_result.is_ok());
}
