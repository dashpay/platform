use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tracing::{info, trace};

use crate::error::DAPIResult;
use crate::logging::AccessLogLayer;
use crate::metrics::MetricsLayer;
use axum::http::{HeaderMap, Request, Response};
use dapi_grpc::core::v0::core_server::CoreServer;
use dapi_grpc::platform::v0::platform_server::PlatformServer;
use dapi_grpc::tonic::Status;
use tower::layer::util::{Identity, Stack};
use tower::util::Either;
use tower::{Layer, Service};

use super::DapiServer;

/// Timeouts for regular requests - sync with envoy config if changed there
const UNARY_TIMEOUT_SECS: u64 = 15;
/// Timeouts for streaming requests - sync with envoy config if changed there
const STREAMING_TIMEOUT_SECS: u64 = 600;
/// Safety margin to ensure we respond before client-side gRPC deadlines fire
const GRPC_REQUEST_TIME_SAFETY_MARGIN: Duration = Duration::from_millis(50);

impl DapiServer {
    /// Start the unified gRPC server that exposes both Platform and Core services.
    /// Configures timeouts, message limits, optional access logging, and then awaits completion.
    /// Returns when the server stops serving.
    pub(super) async fn start_unified_grpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.grpc_server_addr()?;
        info!(
            "Starting unified gRPC server on {} (Core + Platform services)",
            addr
        );

        let platform_service = self.platform_service.clone();
        let core_service = self.core_service.clone();

        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

        let builder = dapi_grpc::tonic::transport::Server::builder()
            .tcp_keepalive(Some(Duration::from_secs(25)))
            .timeout(Duration::from_secs(
                STREAMING_TIMEOUT_SECS.max(UNARY_TIMEOUT_SECS) + 5,
            )); // failsafe timeout - we handle timeouts in the timeout_layer

        // Create timeout layer with different timeouts for unary vs streaming
        let timeout_layer = TimeoutLayer::new(
            Duration::from_secs(UNARY_TIMEOUT_SECS),
            Duration::from_secs(STREAMING_TIMEOUT_SECS),
        );

        let metrics_layer = MetricsLayer::new();
        let access_layer = if let Some(ref access_logger) = self.access_logger {
            Either::Left(AccessLogLayer::new(access_logger.clone()))
        } else {
            Either::Right(Identity::new())
        };

        // Stack layers (execution order: metrics -> access log -> timeout)
        let combined_layer = Stack::new(Stack::new(timeout_layer, access_layer), metrics_layer);
        let mut builder = builder.layer(combined_layer);

        builder
            .add_service(
                PlatformServer::new(platform_service)
                    .max_decoding_message_size(MAX_DECODING_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .add_service(
                CoreServer::new(core_service)
                    .max_decoding_message_size(MAX_DECODING_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .serve(addr)
            .await?;

        Ok(())
    }
}

/// Middleware layer to apply different timeouts based on gRPC method type.
///
/// Streaming methods (subscriptions) get longer timeouts to support long-lived connections,
/// while unary methods get shorter timeouts to prevent resource exhaustion.
#[derive(Clone)]
struct TimeoutLayer {
    unary_timeout: Duration,
    streaming_timeout: Duration,
}

impl TimeoutLayer {
    fn new(unary_timeout: Duration, streaming_timeout: Duration) -> Self {
        Self {
            unary_timeout,
            streaming_timeout,
        }
    }

    /// Determine the appropriate timeout for a given gRPC method path.
    fn timeout_for_method(&self, path: &str) -> Duration {
        // All known streaming methods in Core service (all use "stream" return type)
        const STREAMING_METHODS: &[&str] = &[
            "/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks",
            "/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs",
            "/org.dash.platform.dapi.v0.Core/subscribeToMasternodeList",
            "/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult",
            "/org.dash.platform.dapi.v0.Platform/SubscribePlatformEvents",
        ];

        // Check if this is a known streaming method
        if STREAMING_METHODS.contains(&path) {
            tracing::trace!(
                path,
                "Detected streaming gRPC method, applying streaming timeout"
            );
            self.streaming_timeout
        } else {
            self.unary_timeout
        }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimeoutService {
            inner,
            config: self.clone(),
        }
    }
}

/// Service wrapper that applies per-method timeouts.
#[derive(Clone)]
struct TimeoutService<S> {
    inner: S,
    config: TimeoutLayer,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for TimeoutService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let path = req.uri().path().to_owned();
        let default_timeout = self.config.timeout_for_method(&path);
        let timeout_from_header = parse_grpc_timeout_header(req.headers());
        let effective_timeout = timeout_from_header
            .and_then(|d| d.checked_sub(GRPC_REQUEST_TIME_SAFETY_MARGIN))
            .unwrap_or(default_timeout)
            .min(default_timeout);

        if timeout_from_header.is_some() {
            trace!(
                path,
                header_timeout = timeout_from_header.unwrap_or_default().as_secs_f32(),
                timeout = effective_timeout.as_secs_f32(),
                "Applying gRPC timeout from header"
            );
        } else {
            tracing::trace!(
                path,
                timeout = effective_timeout.as_secs_f32(),
                "Applying default gRPC timeout"
            );
        }
        let timeout_duration = effective_timeout;
        let timeout_secs = timeout_duration.as_secs_f64();
        let fut = tower::timeout::Timeout::new(self.inner.clone(), timeout_duration).call(req);

        Box::pin(async move {
            fut.await.map_err(|err| {
                if err.is::<tower::timeout::error::Elapsed>() {
                    // timeout from TimeoutLayer
                    Status::deadline_exceeded(format!(
                        "request timed out after {:.3}s: {err}",
                        timeout_secs
                    ))
                    .into()
                } else {
                    err
                }
            })
        })
    }
}

/// Parse inbound grpc-timeout header into Duration (RFC 8681 style units)
fn parse_grpc_timeout_header(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get("grpc-timeout")?;
    let as_str = value.to_str().ok()?;
    if as_str.is_empty() {
        return None;
    }
    let (num_part, unit_part) = as_str.split_at(as_str.len().saturating_sub(1));
    let amount: u64 = num_part.parse().ok()?;
    match unit_part {
        "H" => Some(Duration::from_secs(amount.saturating_mul(60 * 60))),
        "M" => Some(Duration::from_secs(amount.saturating_mul(60))),
        "S" => Some(Duration::from_secs(amount)),
        "m" => Some(Duration::from_millis(amount)),
        "u" => Some(Duration::from_micros(amount)),
        "n" => Some(Duration::from_nanos(amount)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll};

    #[derive(Clone)]
    struct SlowService;

    impl Service<Request<()>> for SlowService {
        type Response = Response<()>;
        type Error = Box<dyn std::error::Error + Send + Sync>;
        type Future =
            Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<()>) -> Self::Future {
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(Response::new(()))
            })
        }
    }

    #[tokio::test]
    async fn timeout_service_returns_deadline_exceeded_status() {
        let timeout_layer = TimeoutLayer::new(Duration::from_millis(5), Duration::from_secs(1));
        let mut service = timeout_layer.layer(SlowService);

        let request = Request::builder().uri("/test").body(()).unwrap();

        let err = service
            .call(request)
            .await
            .expect_err("expected timeout error");

        let status = err
            .downcast::<Status>()
            .expect("expected tonic status error");

        assert_eq!(status.code(), dapi_grpc::tonic::Code::DeadlineExceeded);
        assert!(
            status.message().contains("0.005"),
            "status message should include timeout value, got '{}'",
            status.message()
        );
    }
}
