/*!
 * Shared setup utilities for integration tests
 *
 * This module provides centralized test configuration and initialization to avoid
 * code duplication across different test modules. It offers:
 *
 * - Centralized test configuration and initialization
 * - Server startup and teardown helpers
 * - Common test infrastructure for different testing scenarios
 * - Multiple setup functions for different test needs:
 *   - `setup()` - Full gRPC server setup for platform service tests
 *   - `setup_streaming_components()` - Service components for streaming tests
 *   - `setup_config()` - Basic config for configuration tests
 *   - `setup_server_config()` - Arc<Config> for server creation tests
 */

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic;
use rs_dapi::clients::mock::{MockDriveClient, MockTenderdashClient};
use rs_dapi::clients::traits::{DriveClientTrait, TenderdashClientTrait};
use rs_dapi::config::Config;
use rs_dapi::services::{CoreServiceImpl, PlatformServiceImpl, StreamingServiceImpl};
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use tracing::{debug, error, info};

/// Centralized test environment that provides all necessary components for integration tests
/// and automatically cleans up when dropped
pub struct TestEnvironment {
    pub addr: std::net::SocketAddr,
    pub client: PlatformClient<tonic::transport::Channel>,
    pub config: Arc<Config>,
    pub drive_client: Arc<dyn DriveClientTrait>,
    pub tenderdash_client: Arc<dyn TenderdashClientTrait>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if let Some(handle) = &self.server_handle {
            handle.abort();
            debug!("Test server on {} cleaned up", self.addr);
        }
    }
}

/// Initialize tracing for tests - call this once at the beginning of each test
fn init_tracing() {
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

impl TestEnvironment {
    /// Create a new test environment with full gRPC server setup
    /// This is the main setup function for platform service tests
    pub async fn new() -> Self {
        init_tracing();
        Self::with_grpc_server().await
    }

    /// Create test environment with streaming components only (no gRPC server)
    /// This is suitable for streaming service tests
    pub async fn with_streaming_components() -> Self {
        init_tracing();

        let config = Arc::new(Config::default());
        let drive_client: Arc<dyn DriveClientTrait> = Arc::new(MockDriveClient::new());
        let tenderdash_client: Arc<dyn TenderdashClientTrait> =
            Arc::new(MockTenderdashClient::new());

        // For streaming-only tests, we don't need a real client or server
        // We'll use a placeholder address and create a mock client connection
        let dummy_addr = "127.0.0.1:0".parse().unwrap();

        // Create a minimal mock connection for consistency - this won't actually be used
        // in streaming tests, but we need it to satisfy the struct fields
        let dummy_endpoint = tonic::transport::Endpoint::from_static("http://127.0.0.1:0");
        let dummy_channel = dummy_endpoint.connect_lazy();
        let dummy_client = PlatformClient::new(dummy_channel);

        Self {
            addr: dummy_addr,
            client: dummy_client,
            config,
            drive_client,
            tenderdash_client,
            server_handle: None, // No server for streaming-only tests
        }
    }

    /// Create a streaming service from the components
    pub fn create_streaming_service(&self) -> Arc<StreamingServiceImpl> {
        Arc::new(
            StreamingServiceImpl::new(
                self.drive_client.clone(),
                self.tenderdash_client.clone(),
                self.config.clone(),
            )
            .unwrap(),
        )
    }

    /// Create a core service from the streaming service
    pub fn create_core_service(&self) -> CoreServiceImpl {
        let streaming_service = self.create_streaming_service();
        CoreServiceImpl::new(streaming_service, self.config.clone())
    }

    /// Internal method to create test environment with full gRPC server
    async fn with_grpc_server() -> Self {
        // Find an available port
        let port = Self::find_available_port().await;

        // Create mock clients
        let drive_client: Arc<dyn DriveClientTrait> = Arc::new(MockDriveClient::new());
        let tenderdash_client: Arc<dyn TenderdashClientTrait> =
            Arc::new(MockTenderdashClient::new());

        // Create config with test-specific settings
        let mut config = Config::default();
        config.server.grpc_server_port = port;
        let config = Arc::new(config);

        // Create platform service with mock clients
        let platform_service = Arc::new(PlatformServiceImpl::new(
            drive_client.clone(),
            tenderdash_client.clone(),
            config.clone(),
        ));

        let addr = config.grpc_server_addr();

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
        let client = Self::wait_for_server_ready_and_connect(addr).await;

        Self {
            addr,
            client,
            config,
            drive_client,
            tenderdash_client,
            server_handle: Some(server_handle),
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
}

// Convenience functions for backward compatibility and easier usage

/// Main setup function - configures logging and starts a test server
/// This is the main function platform service tests should call
///
/// # Usage
/// ```rust
/// let env = setup::setup().await;
/// // Use env.client, env.addr, env.config, etc.
/// ```
pub async fn setup() -> TestEnvironment {
    TestEnvironment::new().await
}
