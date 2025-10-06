use axum::{Router, response::Json, routing::get};
use serde_json::Value;
use tokio::net::TcpListener;
use tracing::info;

use crate::error::DAPIResult;
use crate::logging::middleware::AccessLogLayer;

use super::DapiServer;

impl DapiServer {
    /// Launch the health and Prometheus metrics server if configured.
    /// Binds Axum routes and wraps them with access logging when available.
    /// Returns early when metrics are disabled.
    pub(super) async fn start_metrics_server(&self) -> DAPIResult<()> {
        let Some(addr) = self.config.metrics_addr() else {
            info!("Metrics server disabled; skipping startup");
            return Ok(());
        };

        info!("Starting metrics server (health + Prometheus) on {}", addr);

        let mut app = Router::new()
            .route("/health", get(handle_health))
            .route("/health/ready", get(handle_ready))
            .route("/health/live", get(handle_live))
            .route("/metrics", get(handle_metrics));

        if let Some(ref access_logger) = self.access_logger {
            app = app.layer(AccessLogLayer::new(access_logger.clone()));
        }

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Report overall health along with build metadata for readiness probes.
async fn handle_health() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().timestamp(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Indicate the server is ready to serve traffic with a timestamp payload.
async fn handle_ready() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().timestamp(),
    }))
}

/// Indicate the server process is alive with a timestamp payload.
async fn handle_live() -> Json<Value> {
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().timestamp(),
    }))
}

/// Expose Prometheus-formatted metrics gathered from the registry.
async fn handle_metrics() -> axum::response::Response {
    let (body, content_type) = crate::metrics::gather_prometheus();
    axum::response::Response::builder()
        .status(200)
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| axum::response::Response::new(axum::body::Body::from("")))
}
