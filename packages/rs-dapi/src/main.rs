use rs_dapi::DAPIResult;
use tracing::{error, info, trace};

use rs_dapi::config::Config;
use rs_dapi::server::DapiServer;

#[tokio::main]
async fn main() -> DAPIResult<()> {
    // Initialize tracing; by default, we log rs_dapi at debug level, others at info
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "rs_dapi=debug,info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting rs-dapi server...");

    trace!("Loading configuration...");
    // Load configuration
    let config = Config::load()?;
    trace!("Configuration loaded successfully");

    trace!("Creating DAPI server instance...");
    // Create and start the server
    let server = DapiServer::new(std::sync::Arc::new(config)).await?;

    info!("rs-dapi server starting on configured ports");

    trace!("Starting server main loop...");
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    info!("rs-dapi server shutdown complete");
    Ok(())
}
