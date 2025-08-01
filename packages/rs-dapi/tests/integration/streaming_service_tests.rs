/*!
 * Streaming service integration tests
 *
 * These tests validate the streaming service and core service components using mock clients.
 * Each test uses the shared TestEnvironment for consistent test execution.
 */

use rs_dapi::config::Config;

use super::setup;

#[tokio::test]
async fn test_streaming_service_integration() {
    let env = setup::TestEnvironment::with_streaming_components().await;
    let streaming_service = env.create_streaming_service();

    // Test that we can create the service successfully
    assert!(
        streaming_service
            .subscriber_manager
            .subscription_count()
            .await
            == 0
    );

    // Test that streaming service initialization works
    // Note: We can't actually start the streaming service in a test without a real ZMQ connection
    // but we can verify the structure is correct
    assert!(!env.config.dapi.core.zmq_url.is_empty());
}

#[tokio::test]
async fn test_config_loading() {
    let config = Config::default();

    // Test default configuration values
    assert_eq!(config.server.grpc_server_port, 3005);
    assert_eq!(config.dapi.core.zmq_url, "tcp://127.0.0.1:29998");
    assert_eq!(config.server.bind_address, "127.0.0.1");
}

#[tokio::test]
async fn test_server_creation() {
    let config = Config::default();

    // Test that we can create a DapiServer successfully
    let server_result = rs_dapi::server::DapiServer::new(config.into()).await;
    assert!(server_result.is_ok());
}
