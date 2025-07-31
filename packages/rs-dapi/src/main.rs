use anyhow::Result;
use rs_dapi::DAPIResult;
use tracing::{error, info};

use rs_dapi::config::Config;
use rs_dapi::server::DapiServer;

#[tokio::main]
async fn main() -> DAPIResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting rs-dapi server...");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: {:?}", config);

    // Create and start the server
    let server = DapiServer::new(std::sync::Arc::new(config)).await?;

    info!("rs-dapi server starting on configured ports");

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}
