/*!
 * Platform service integration tests
 *
 * These tests validate the platform service gRPC endpoints using mock clients.
 * Each test uses the shared setup infrastructure for consistent test execution.
 */

use super::setup;
use dapi_grpc::platform::v0::{get_status_request, get_status_response, GetStatusRequest};
use dapi_grpc::tonic;
use tracing::info;

/// Test the basic getStatus endpoint functionality
#[tokio::test]
async fn test_get_status_endpoint() {
    let server = setup::setup().await;

    // Create the request
    let request = tonic::Request::new(GetStatusRequest {
        version: Some(get_status_request::Version::V0(
            get_status_request::GetStatusRequestV0 {},
        )),
    });

    // Call the getStatus endpoint
    let response = server.client.clone().get_status(request).await;
    assert!(response.is_ok(), "getStatus should succeed");

    let status_response = response.unwrap().into_inner();

    // Validate the response structure
    assert!(
        status_response.version.is_some(),
        "Response should have version field"
    );

    if let Some(get_status_response::Version::V0(v0)) = status_response.version {
        assert!(v0.time.is_some(), "Response should have time field");

        if let Some(time) = v0.time {
            assert!(time.local > 0, "Local time should be set");
            info!("✓ getStatus endpoint working correctly");
            info!("  - Local time: {}", time.local);
            info!("  - Block time: {:?}", time.block);
            info!("  - Genesis time: {:?}", time.genesis);
            info!("  - Epoch time: {:?}", time.epoch);
        }
    }

    // Server will be automatically cleaned up when `server` is dropped
}

/// Test that mock clients provide the expected test data
#[tokio::test]
async fn test_mock_data_injection() {
    let server = setup::setup().await;

    let request = tonic::Request::new(GetStatusRequest {
        version: Some(get_status_request::Version::V0(
            get_status_request::GetStatusRequestV0 {},
        )),
    });

    let response = server.client.clone().get_status(request).await.unwrap();
    let status_response = response.into_inner();

    // Verify we're getting the expected mock data
    if let Some(get_status_response::Version::V0(v0)) = status_response.version {
        // Check version info comes from mock clients
        if let Some(version) = v0.version {
            if let Some(software) = version.software {
                assert_eq!(
                    software.drive,
                    Some("1.1.1".to_string()),
                    "Should get mock Drive version"
                );
                assert_eq!(
                    software.tenderdash,
                    Some("0.11.0".to_string()),
                    "Should get mock Tenderdash version"
                );
            }

            if let Some(protocol) = version.protocol {
                if let Some(drive) = protocol.drive {
                    assert_eq!(
                        drive.current, 1,
                        "Should get mock Drive protocol current version"
                    );
                    assert_eq!(
                        drive.latest, 2,
                        "Should get mock Drive protocol latest version"
                    );
                }
            }
        }

        // Check chain info comes from mock clients
        if let Some(chain) = v0.chain {
            assert_eq!(
                chain.core_chain_locked_height,
                Some(1000),
                "Should get mock core chain locked height"
            );
        }

        // Check network info comes from mock clients
        if let Some(network) = v0.network {
            assert_eq!(network.peers_count, 8, "Should get mock peers count");
            assert!(network.listening, "Should get mock listening status");
        }
    }

    info!("✓ Mock clients are providing expected test data");
    // Server will be automatically cleaned up when `server` is dropped
}

/// Test server lifecycle management
#[tokio::test]
async fn test_server_lifecycle() {
    let server = setup::setup().await;

    // Server should be ready immediately after setup
    let addr = server.addr;
    info!("✓ Server started successfully on {}", addr);

    // Server should be responsive
    let request = tonic::Request::new(GetStatusRequest {
        version: Some(get_status_request::Version::V0(
            get_status_request::GetStatusRequestV0 {},
        )),
    });

    let response = server.client.clone().get_status(request).await;
    assert!(response.is_ok(), "Server should be responsive");

    info!("✓ Server is responsive and will be cleaned up automatically");
    // Server will be automatically cleaned up when `server` is dropped
}
