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
use dapi_grpc::tonic::body::Body as TonicBody;
use dapi_grpc::tonic::codegen::Bytes;
use http_body::{Body, Frame};
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

/// Coarse DoS backstop on the *encoded* request body of every Platform method
/// that does not carry a state transition. Must stay strictly above every
/// per-method app-layer budget plus protobuf envelope overhead, so that
/// oversized-but-parseable requests reach the app-layer validators and get
/// their precise errors instead of dying here with a generic status. The
/// largest legitimate query payload is `getPathElements` (MAX_PATH_QUERY_BYTES,
/// 64 KiB of raw components, ~65 KiB encoded); every other query sits far
/// below it. Enforced on the raw body before Prost materialises anything.
const MAX_PLATFORM_QUERY_BODY_BYTES: usize = 128 * 1024; // 128 KiB
// The body cap sits above the largest query's raw component budget plus framing, or that
// query could never reach its own validator.
const _: () = assert!(
    MAX_PLATFORM_QUERY_BODY_BYTES > crate::services::platform_service::MAX_PATH_QUERY_BYTES
);
/// The one Platform method that legitimately carries a large body: a
/// broadcast of a contract-code capable state transition
/// (`max_contract_code_state_transition_size`, 32 MiB from protocol
/// version 17) plus protobuf framing. This is also tonic's service-wide
/// decode cap, so it is the ceiling for every method and the body limit
/// below is what keeps the queries on the smaller allowance.
const MAX_PLATFORM_TRANSACTION_BODY_BYTES: usize = 34 * 1024 * 1024; // 34 MiB
/// The Platform methods allowed the transaction body allowance.
const PLATFORM_TRANSACTION_METHODS: &[&str] =
    &["/org.dash.platform.dapi.v0.Platform/broadcastStateTransition"];
/// The gRPC service the body limits apply to; every other service (Core) keeps
/// its own service-wide decode cap and is not touched by the layer.
const PLATFORM_SERVICE_PATH_PREFIX: &str = "/org.dash.platform.dapi.v0.Platform/";
/// Same principle for Core: sized above the largest app-layer budget
/// (raw transaction wire cap, 400 KB) plus envelope overhead.
const MAX_CORE_DECODING_BYTES: usize = 512 * 1024; // 512 KiB
const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

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

        // Per-method request body cap on the Platform service, ahead of
        // tonic's service-wide decode cap: only the transaction ingress gets
        // the large allowance.
        let body_limit_layer = BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
        );

        // Stack layers (execution order: metrics -> access log -> timeout -> body limit)
        let combined_layer = Stack::new(
            Stack::new(Stack::new(body_limit_layer, timeout_layer), access_layer),
            metrics_layer,
        );
        let mut builder = builder.layer(combined_layer);

        builder
            .add_service(
                PlatformServer::new(platform_service)
                    .max_decoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .add_service(
                CoreServer::new(core_service)
                    .max_decoding_message_size(MAX_CORE_DECODING_BYTES)
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
            "/org.dash.platform.dapi.v0.Platform/subscribePlatformEvents",
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

/// Middleware layer that caps the request body per gRPC method of one service
/// before tonic decodes it.
///
/// Tonic's `max_decoding_message_size` is one number per service, so raising
/// it for the state transition ingress would hand every query the same
/// allowance. This layer wraps the request body of the methods of the
/// configured service: a method on the allow list may send up to the large
/// limit, every other method of that service is cut off at the small one.
/// Two checks enforce the limit, both before Prost materialises a single
/// field:
///
/// * the gRPC message header (the five bytes tonic reads first) declares the
///   message length, and a declared length over the limit is refused at once,
///   before tonic reserves a receive buffer of that size;
/// * the bytes actually delivered are counted and the body is cut off when
///   they pass the limit, so a header that lies small does not help either.
///
/// Requests to any other service pass through untouched.
#[derive(Clone)]
struct BodyLimitLayer {
    service_path_prefix: &'static str,
    default_limit: usize,
    large_limit: usize,
    large_limit_methods: &'static [&'static str],
}

impl BodyLimitLayer {
    fn new(
        service_path_prefix: &'static str,
        default_limit: usize,
        large_limit: usize,
        large_limit_methods: &'static [&'static str],
    ) -> Self {
        Self {
            service_path_prefix,
            default_limit,
            large_limit,
            large_limit_methods,
        }
    }

    /// The body limit for a gRPC method path, `None` for a method of another
    /// service.
    fn limit_for_method(&self, path: &str) -> Option<usize> {
        if !path.starts_with(self.service_path_prefix) {
            return None;
        }
        if self.large_limit_methods.contains(&path) {
            Some(self.large_limit)
        } else {
            Some(self.default_limit)
        }
    }
}

impl<S> Layer<S> for BodyLimitLayer {
    type Service = BodyLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodyLimitService {
            inner,
            config: self.clone(),
        }
    }
}

/// Service wrapper that applies per-method request body limits.
#[derive(Clone)]
struct BodyLimitService<S> {
    inner: S,
    config: BodyLimitLayer,
}

impl<S, ReqBody> Service<Request<ReqBody>> for BodyLimitService<S>
where
    S: Service<Request<TonicBody>>,
    ReqBody: Body<Data = Bytes> + Send + Unpin + 'static,
    ReqBody::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let limit = self.config.limit_for_method(req.uri().path());
        let (parts, body) = req.into_parts();
        let body = match limit {
            Some(limit) => TonicBody::new(LimitedMessageBody::new(body, limit)),
            None => TonicBody::new(body),
        };
        self.inner.call(Request::from_parts(parts, body))
    }
}

/// The gRPC message header: one compression flag byte and a big-endian `u32`
/// message length.
const GRPC_MESSAGE_HEADER_LEN: usize = 5;

/// A request body bounded by a byte limit, checked on the declared length of
/// the first gRPC message as soon as its header is in and on every byte
/// delivered after that.
struct LimitedMessageBody<B> {
    inner: B,
    limit: usize,
    delivered: usize,
    /// The first bytes of the body until the message header is complete;
    /// `None` once it has been checked.
    header: Option<Vec<u8>>,
}

impl<B> LimitedMessageBody<B> {
    fn new(inner: B, limit: usize) -> Self {
        Self {
            inner,
            limit,
            delivered: 0,
            header: Some(Vec::with_capacity(GRPC_MESSAGE_HEADER_LEN)),
        }
    }

    fn over_limit(limit: usize) -> Status {
        Status::resource_exhausted(format!(
            "request body exceeds the {limit} byte limit of this method"
        ))
    }

    /// Accounts for a delivered chunk: the declared length once the header is
    /// complete, then the running total.
    fn check_chunk(&mut self, chunk: &Bytes) -> Result<(), Status> {
        self.delivered = self.delivered.saturating_add(chunk.len());
        if self.delivered > self.limit {
            return Err(Self::over_limit(self.limit));
        }
        if let Some(header) = self.header.as_mut() {
            let missing = GRPC_MESSAGE_HEADER_LEN - header.len();
            header.extend_from_slice(&chunk[..chunk.len().min(missing)]);
            if header.len() == GRPC_MESSAGE_HEADER_LEN {
                let declared =
                    u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
                self.header = None;
                if declared.saturating_add(GRPC_MESSAGE_HEADER_LEN) > self.limit {
                    return Err(Self::over_limit(self.limit));
                }
            }
        }
        Ok(())
    }
}

impl<B> Body for LimitedMessageBody<B>
where
    B: Body<Data = Bytes> + Unpin,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Data = Bytes;
    type Error = Status;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = &mut *self;
        let frame = match std::task::ready!(Pin::new(&mut this.inner).poll_frame(cx)) {
            None => return Poll::Ready(None),
            Some(Err(error)) => {
                return Poll::Ready(Some(Err(Status::from_error(error.into()))));
            }
            Some(Ok(frame)) => frame,
        };
        if let Some(chunk) = frame.data_ref()
            && let Err(status) = this.check_chunk(chunk)
        {
            return Poll::Ready(Some(Err(status)));
        }
        Poll::Ready(Some(Ok(frame)))
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
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
    use http_body_util::BodyExt;
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

    /// A service that drains the request body and answers with its length, so a test can see
    /// whether the limit layer let the bytes through.
    #[derive(Clone)]
    struct BodyLengthService;

    impl Service<Request<TonicBody>> for BodyLengthService {
        type Response = Response<usize>;
        type Error = Box<dyn std::error::Error + Send + Sync>;
        type Future =
            Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request<TonicBody>) -> Self::Future {
            Box::pin(async move {
                let collected = req.into_body().collect().await?;
                Ok(Response::new(collected.to_bytes().len()))
            })
        }
    }

    const QUERY_PATH: &str = "/org.dash.platform.dapi.v0.Platform/getPathElements";
    const TRANSACTION_PATH: &str = "/org.dash.platform.dapi.v0.Platform/broadcastStateTransition";
    const CORE_PATH: &str = "/org.dash.platform.dapi.v0.Core/broadcastTransaction";

    fn body_limit_layer() -> BodyLimitLayer {
        BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
        )
    }

    /// A gRPC message body of `payload_len` bytes with a header declaring `declared_len`.
    fn grpc_message(declared_len: usize, payload_len: usize) -> Vec<u8> {
        let mut body = vec![0u8];
        body.extend_from_slice(&(declared_len as u32).to_be_bytes());
        body.extend(std::iter::repeat_n(0x5Au8, payload_len));
        body
    }

    async fn send_body(path: &str, body: Vec<u8>) -> Result<usize, Status> {
        let mut service = body_limit_layer().layer(BodyLengthService);
        let request = Request::builder()
            .uri(path)
            .body(axum::body::Body::from(body))
            .expect("request");
        service
            .call(request)
            .await
            .map(|response| response.into_body())
            .map_err(|error| {
                *error
                    .downcast::<Status>()
                    .expect("the limit layer reports a tonic status")
            })
    }

    /// A well-formed gRPC message whose total body is `body_len` bytes.
    async fn send(path: &str, body_len: usize) -> Result<usize, Status> {
        let payload_len = body_len - GRPC_MESSAGE_HEADER_LEN;
        send_body(path, grpc_message(payload_len, payload_len)).await
    }

    /// A query at its limit passes intact and one byte over is refused before the service
    /// sees it, while the transaction ingress accepts a body far above the query limit.
    #[tokio::test]
    async fn body_limit_is_per_method() {
        assert_eq!(
            send(QUERY_PATH, MAX_PLATFORM_QUERY_BODY_BYTES)
                .await
                .expect("a query at the limit passes"),
            MAX_PLATFORM_QUERY_BODY_BYTES
        );
        let status = send(QUERY_PATH, MAX_PLATFORM_QUERY_BODY_BYTES + 1)
            .await
            .expect_err("a query over the limit is refused");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
        assert!(
            status
                .message()
                .contains(&MAX_PLATFORM_QUERY_BODY_BYTES.to_string()),
            "the status names the limit: {}",
            status.message()
        );

        let family_cap = dpp::version::PlatformVersion::latest()
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes")
            as usize;
        assert_eq!(
            send(TRANSACTION_PATH, family_cap)
                .await
                .expect("a family-cap transaction passes the ingress"),
            family_cap
        );
        let status = send(TRANSACTION_PATH, MAX_PLATFORM_TRANSACTION_BODY_BYTES + 1)
            .await
            .expect_err("the ingress has a ceiling too");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
    }

    /// The declared message length is checked as soon as the header is in: a query that
    /// announces a transaction-sized message is refused on its first bytes, before tonic
    /// would reserve a receive buffer of that size, and a header that lies small is still
    /// caught by the delivered-byte count.
    #[tokio::test]
    async fn body_limit_checks_the_declared_message_length_first() {
        // Declares 30 MiB, sends 100 bytes: refused at once by the header check.
        let status = send_body(QUERY_PATH, grpc_message(30 * 1024 * 1024, 100))
            .await
            .expect_err("a declared length over the limit is refused");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
        assert!(
            status
                .message()
                .contains(&MAX_PLATFORM_QUERY_BODY_BYTES.to_string()),
            "{status:?}"
        );

        // The same declaration is fine for the transaction ingress.
        assert_eq!(
            send_body(TRANSACTION_PATH, grpc_message(30 * 1024 * 1024, 100))
                .await
                .expect("the ingress admits a large declared length"),
            100 + GRPC_MESSAGE_HEADER_LEN
        );

        // Declares 10 bytes but streams well past the limit: caught by the byte count.
        let status = send_body(QUERY_PATH, grpc_message(10, MAX_PLATFORM_QUERY_BODY_BYTES))
            .await
            .expect_err("a body over the limit is refused whatever it declares");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
    }

    /// The layer is scoped to the Platform service: a Core request above the Platform query
    /// allowance passes through untouched, so Core keeps its own service-wide cap.
    #[tokio::test]
    async fn body_limit_leaves_core_methods_alone() {
        let core_body_len = MAX_CORE_DECODING_BYTES - 1;
        assert!(core_body_len > MAX_PLATFORM_QUERY_BODY_BYTES);
        assert_eq!(
            send(CORE_PATH, core_body_len)
                .await
                .expect("a Core request is not bounded by the Platform limits"),
            core_body_len
        );
    }

    /// The transaction allowance must fit the largest state transition of every registered
    /// protocol version plus framing.
    #[test]
    fn body_limits_cover_the_app_layer_budgets() {
        const FRAMING_HEADROOM_BYTES: usize = 1024 * 1024;
        for platform_version in dpp::version::PLATFORM_VERSIONS {
            let limits = &platform_version.system_limits;
            let largest_family_cap = limits
                .max_contract_code_state_transition_size
                .unwrap_or(0)
                .max(limits.max_state_transition_size)
                as usize;
            assert!(
                MAX_PLATFORM_TRANSACTION_BODY_BYTES >= largest_family_cap + FRAMING_HEADROOM_BYTES,
                "protocol version {} admits a {largest_family_cap} byte state transition the \
                 ingress could not receive",
                platform_version.protocol_version
            );
        }
    }

    /// Through a real tonic server and HTTP/2 client: a `getPathElements` body above the query
    /// allowance is refused by the layer (ResourceExhausted, naming the limit) even though it
    /// is far below tonic's service-wide decode cap, and a broadcast carrying a family-cap
    /// transition reaches the service intact. The service behind the layer is a plain gRPC
    /// unary handler that answers with the byte count it received, so the assertion is on what
    /// crossed the layer, not on Platform semantics.
    #[tokio::test]
    async fn real_server_applies_the_per_method_body_limit() {
        use dapi_grpc::tonic::client::Grpc as GrpcClient;
        use dapi_grpc::tonic::server::{Grpc as GrpcServer, NamedService, UnaryService};
        use dapi_grpc::tonic::transport::server::TcpIncoming;
        use dapi_grpc::tonic::transport::{Channel, Server};
        use dapi_grpc::tonic::{Code, Request as TonicRequest, Response as TonicResponse};
        use dapi_grpc::tonic_prost::ProstCodec;
        use tokio::net::TcpListener;

        /// A gRPC service whose every method takes raw bytes and returns their length.
        #[derive(Clone)]
        struct ByteCounter;

        impl NamedService for ByteCounter {
            const NAME: &'static str = "org.dash.platform.dapi.v0.Platform";
        }

        impl UnaryService<Vec<u8>> for ByteCounter {
            type Response = u64;
            type Future = std::future::Ready<Result<TonicResponse<u64>, Status>>;

            fn call(&mut self, request: TonicRequest<Vec<u8>>) -> Self::Future {
                std::future::ready(Ok(TonicResponse::new(request.into_inner().len() as u64)))
            }
        }

        impl Service<Request<TonicBody>> for ByteCounter {
            type Response = Response<TonicBody>;
            type Error = std::convert::Infallible;
            type Future =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, req: Request<TonicBody>) -> Self::Future {
                let service = self.clone();
                Box::pin(async move {
                    let mut grpc = GrpcServer::new(ProstCodec::default())
                        .max_decoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES);
                    Ok(grpc.unary(service, req).await)
                })
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("local address");
        let server = tokio::spawn(async move {
            Server::builder()
                .layer(body_limit_layer())
                .add_service(ByteCounter)
                .serve_with_incoming(TcpIncoming::from(listener))
                .await
                .expect("test server");
        });

        let channel = Channel::from_shared(format!("http://{address}"))
            .expect("endpoint")
            .connect()
            .await
            .expect("connect");
        let mut client = GrpcClient::new(channel)
            .max_encoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES)
            .max_decoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES);
        let mut call = async |path: &str, payload: Vec<u8>| {
            client.ready().await.expect("client ready");
            client
                .unary(
                    TonicRequest::new(payload),
                    path.parse().expect("path"),
                    ProstCodec::<Vec<u8>, u64>::default(),
                )
                .await
                .map(|response| response.into_inner())
        };

        // A query body above the query allowance but below the service-wide cap is refused by
        // the layer, and the status names the limit.
        let status = call(QUERY_PATH, vec![0x5Au8; MAX_PLATFORM_QUERY_BODY_BYTES + 1])
            .await
            .expect_err("the oversized query is refused");
        assert_eq!(status.code(), Code::ResourceExhausted, "{status:?}");
        assert!(
            status
                .message()
                .contains(&MAX_PLATFORM_QUERY_BODY_BYTES.to_string()),
            "{status:?}"
        );

        // A query under its allowance and a family-cap transition both reach the service.
        assert_eq!(
            call(QUERY_PATH, vec![0x5Au8; 64 * 1024])
                .await
                .expect("a query under the allowance passes"),
            64 * 1024
        );
        let family_cap = dpp::version::PlatformVersion::latest()
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes")
            as usize;
        assert_eq!(
            call(TRANSACTION_PATH, vec![0x5Au8; family_cap])
                .await
                .expect("a family-cap transition reaches the service"),
            family_cap as u64
        );

        server.abort();
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
