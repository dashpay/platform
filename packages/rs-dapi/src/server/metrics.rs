use axum::{Router, extract::State, http::StatusCode, response::Json, routing::get};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};
use tracing::info;

use crate::error::DAPIResult;
use crate::logging::middleware::AccessLogLayer;

use super::DapiServer;
use super::state::MetricsAppState;

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

        let app_state = MetricsAppState {
            platform_service: self.platform_service.as_ref().clone(),
            core_service: self.core_service.as_ref().clone(),
        };

        let mut app = Router::new()
            .route("/health", get(handle_health))
            .route("/metrics", get(handle_metrics))
            .with_state(app_state);

        if let Some(ref access_logger) = self.access_logger {
            app = app.layer(AccessLogLayer::new(access_logger.clone()));
        }

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Run health checks against upstream dependencies and expose consolidated status.
async fn handle_health(State(state): State<MetricsAppState>) -> impl axum::response::IntoResponse {
    const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

    let platform_service = state.platform_service.clone();
    let core_client = state.core_service.core_client.clone();

    let platform_result = timeout(HEALTH_CHECK_TIMEOUT, async move {
        platform_service
            .build_status_response_with_health()
            .await
            .map(|(_, health)| health)
    });

    let core_result = timeout(HEALTH_CHECK_TIMEOUT, async move {
        core_client.get_block_count().await
    });

    let (platform_result, core_result) = tokio::join!(platform_result, core_result);

    let (platform_ok, platform_payload) = match platform_result {
        Ok(Ok(health)) => {
            let is_healthy = health.is_healthy();
            let payload = PlatformChecks {
                status: if is_healthy {
                    "ok".into()
                } else {
                    "degraded".into()
                },
                error: None,
                drive: Some(ComponentCheck::from_option(health.drive_error.clone())),
                tenderdash_status: Some(ComponentCheck::from_option(
                    health.tenderdash_status_error.clone(),
                )),
                tenderdash_net_info: Some(ComponentCheck::from_option(
                    health.tenderdash_netinfo_error.clone(),
                )),
            };
            (is_healthy, payload)
        }
        Ok(Err(err)) => (
            false,
            PlatformChecks {
                status: "error".into(),
                error: Some(err.to_string()),
                drive: None,
                tenderdash_status: None,
                tenderdash_net_info: None,
            },
        ),
        Err(_) => (
            false,
            PlatformChecks {
                status: "error".into(),
                error: Some("timeout".into()),
                drive: None,
                tenderdash_status: None,
                tenderdash_net_info: None,
            },
        ),
    };

    let (core_ok, core_payload) = match core_result {
        Ok(Ok(height)) => (
            true,
            CoreRpcCheck {
                status: "ok".into(),
                latest_block_height: Some(height),
                error: None,
            },
        ),
        Ok(Err(err)) => (
            false,
            CoreRpcCheck {
                status: "error".into(),
                latest_block_height: None,
                error: Some(err.to_string()),
            },
        ),
        Err(_) => (
            false,
            CoreRpcCheck {
                status: "error".into(),
                latest_block_height: None,
                error: Some("timeout".into()),
            },
        ),
    };

    let overall_status = match (platform_ok, core_ok) {
        (true, true) => "ok",
        (false, false) => "error",
        _ => "degraded",
    };

    let http_status = if overall_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = HealthResponse {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        version: env!("CARGO_PKG_VERSION"),
        checks: Checks {
            platform: platform_payload,
            core_rpc: core_payload,
        },
    };

    (http_status, Json(body))
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

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: i64,
    version: &'static str,
    checks: Checks,
}

#[derive(Serialize)]
struct Checks {
    platform: PlatformChecks,
    #[serde(rename = "coreRpc")]
    core_rpc: CoreRpcCheck,
}

#[derive(Serialize)]
struct PlatformChecks {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drive: Option<ComponentCheck>,
    #[serde(rename = "tenderdashStatus", skip_serializing_if = "Option::is_none")]
    tenderdash_status: Option<ComponentCheck>,
    #[serde(rename = "tenderdashNetInfo", skip_serializing_if = "Option::is_none")]
    tenderdash_net_info: Option<ComponentCheck>,
}

#[derive(Serialize)]
struct CoreRpcCheck {
    status: String,
    #[serde(rename = "latestBlockHeight", skip_serializing_if = "Option::is_none")]
    latest_block_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct ComponentCheck {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ComponentCheck {
    fn from_option(error: Option<String>) -> Self {
        match error {
            Some(err) => Self {
                status: "error".into(),
                error: Some(err),
            },
            None => Self {
                status: "ok".into(),
                error: None,
            },
        }
    }
}
