/*!
 * Shared setup utilities for integration tests
 *
 * This module provides:
 * - Centralized test configuration and initialization
 * - Server startup and teardown helpers
 * - Common test infrastructure
 */

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic;
use rs_dapi::clients::mock::{MockDriveClient, MockTenderdashClient};
use rs_dapi::clients::traits::{DriveClientTrait, TenderdashClientTrait};
use rs_dapi::config::Config;
use rs_dapi::services::PlatformServiceImpl;
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use tracing::{debug, error, info};

/// Test server guard that automatically cleans up when dropped
pub struct TestServerGuard {
    pub addr: std::net::SocketAddr,
    pub client: PlatformClient<tonic::transport::Channel>,
    server_handle: tokio::task::JoinHandle<()>,
}

impl Drop for TestServerGuard {
    fn drop(&mut self) {
        self.server_handle.abort();
        debug!("Test server on {} cleaned up", self.addr);
    }
}

/// Initialize tracing for tests - call this once at the beginning of each test
pub fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "rs_dapi=debug".to_string());

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .init();
    });
}

/// Main setup function - configures logging and starts a test server
/// This is the only function tests should call to get a ready-to-use test environment
pub async fn setup() -> TestServerGuard {
    init_tracing();
    start_test_server().await
}

/// Start a test server with mock clients on an available port
async fn start_test_server() -> TestServerGuard {
    // Find an available port
    let port = find_available_port().await;

    // Create mock clients
    let drive_client: Arc<dyn DriveClientTrait> = Arc::new(MockDriveClient::new());
    let tenderdash_client: Arc<dyn TenderdashClientTrait> = Arc::new(MockTenderdashClient::new());

    // Create config with test-specific settings
    let mut config = Config::default();
    config.server.grpc_api_port = port;

    // Create platform service with mock clients
    let platform_service = Arc::new(PlatformServiceImpl::new(
        drive_client,
        tenderdash_client,
        config.clone(),
    ));

    let addr = config.grpc_api_addr();

    // Start the server in a background task
    let server_handle = tokio::spawn(async move {
        use dapi_grpc::platform::v0::platform_server::PlatformServer;
        use dapi_grpc::tonic::transport::Server;

        info!("Starting test server on {}", addr);

        let platform_service_clone = platform_service.clone();

        let result = Server::builder()
            .add_service(PlatformServer::new((*platform_service_clone).clone()))
            .serve(addr)
            .await;

        match result {
            Ok(_) => info!("Server completed successfully"),
            Err(e) => {
                error!("Server error: {} (Error details: {:?})", e, e);
                error!("Server failed to bind to address: {}", addr);
            }
        }
    });

    // Wait for the server to be ready and create a client
    let client = wait_for_server_ready_and_connect(addr).await;

    TestServerGuard {
        addr,
        client,
        server_handle,
    }
}

/// Find an available port starting from 3000
async fn find_available_port() -> u16 {
    use tokio::net::TcpListener;

    for port in 3000..4000 {
        if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            drop(listener);
            return port;
        }
    }
    panic!("Could not find an available port");
}

/// Wait for server to be ready and return a connected client
async fn wait_for_server_ready_and_connect(
    addr: std::net::SocketAddr,
) -> PlatformClient<tonic::transport::Channel> {
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    loop {
        // Try to make an actual gRPC connection
        match timeout(
            Duration::from_millis(100),
            PlatformClient::connect(format!("http://{}", addr)),
        )
        .await
        {
            Ok(Ok(client)) => {
                info!("Server is ready on {}", addr);
                return client;
            }
            Ok(Err(e)) => {
                debug!("gRPC connection failed: {}, retrying...", e);
            }
            Err(_) => {
                debug!("Connection attempt timed out, retrying...");
            }
        }

        if start_time.elapsed() > timeout_duration {
            panic!("Server failed to start within 5 seconds on {}", addr);
        }
        sleep(Duration::from_millis(10)).await;
    }
}
