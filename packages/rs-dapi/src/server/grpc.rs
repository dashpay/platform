use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tracing::{info, trace};

use crate::error::DAPIResult;
use crate::logging::AccessLogLayer;
use crate::metrics::MetricsLayer;
use crate::services::platform_service::SourceKey;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Request, Response};
use dapi_grpc::core::v0::core_server::CoreServer;
use dapi_grpc::platform::v0::platform_server::PlatformServer;
use dapi_grpc::tonic::Status;
use dapi_grpc::tonic::body::Body as TonicBody;
use dapi_grpc::tonic::codegen::Bytes;
use dapi_grpc::tonic::transport::server::TcpConnectInfo;
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
/// How many requests at once may hold a message larger than the query
/// allowance on the transaction ingress. tonic reserves a receive buffer of the
/// declared size as soon as a message header is in, so without this bound a
/// client could open many streams, send only a header declaring the full
/// allowance on each, and pin that much memory per stream until the unary
/// timeout. With it the receive buffers of large broadcasts stay under
/// 8 x 34 MiB = 272 MiB; a request over the bound is refused with
/// `ResourceExhausted` before tonic reserves anything, and a broadcast of a
/// small transition never needs a slot. Node-local admission policy, not
/// consensus.
const MAX_CONCURRENT_LARGE_TRANSACTION_BODIES: usize = 8;
/// How many of those slots one source may hold at once, so a single client
/// holding stalled streams open cannot take every slot from other publishers.
/// A source is the address the shielded proof failure budget keys on (the
/// client address, or the gateway's `x-forwarded-for` entry behind Envoy; an
/// IPv6 client counts as its /48).
const MAX_CONCURRENT_LARGE_TRANSACTION_BODIES_PER_SOURCE: usize = 2;
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
            MAX_CONCURRENT_LARGE_TRANSACTION_BODIES,
            MAX_CONCURRENT_LARGE_TRANSACTION_BODIES_PER_SOURCE,
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
/// * the header of every gRPC message in the body (the five bytes tonic reads
///   before it reserves a receive buffer of the declared size) is checked as
///   soon as it is in, and a message that could not complete within the limit
///   is refused at once. Every message counts, not only the first: tonic's
///   unary path drains the messages after the first while looking for the
///   trailers, reserving for each of them under the service-wide cap;
/// * the bytes actually delivered are counted and the body is cut off when
///   they pass the limit, so a header that lies small does not help either.
///
/// A request on the allow list whose messages grow past the small limit also
/// needs one of a fixed number of large-body slots, taken when the header that
/// crosses the small limit is in and held until the body is dropped, so the
/// receive buffers tonic reserves for large messages are bounded in total and
/// not only per request. One source may hold only a few of them.
///
/// Requests to any other service pass through untouched.
#[derive(Clone)]
struct BodyLimitLayer {
    service_path_prefix: &'static str,
    default_limit: usize,
    large_limit: usize,
    large_limit_methods: &'static [&'static str],
    large_body_slots: Arc<LargeBodySlots>,
}

impl BodyLimitLayer {
    fn new(
        service_path_prefix: &'static str,
        default_limit: usize,
        large_limit: usize,
        large_limit_methods: &'static [&'static str],
        max_concurrent_large_bodies: usize,
        max_concurrent_large_bodies_per_source: usize,
    ) -> Self {
        Self {
            service_path_prefix,
            default_limit,
            large_limit,
            large_limit_methods,
            large_body_slots: Arc::new(LargeBodySlots::new(
                max_concurrent_large_bodies,
                max_concurrent_large_bodies_per_source,
            )),
        }
    }

    /// Wraps the request body of a method of the configured service in its
    /// limit, or gives it back for a method of another service.
    fn limit_body<B>(&self, parts: &Parts, body: B) -> Result<LimitedMessageBody<B>, B> {
        let path = parts.uri.path();
        if !path.starts_with(self.service_path_prefix) {
            return Err(body);
        }
        if self.large_limit_methods.contains(&path) {
            let peer = parts
                .extensions
                .get::<TcpConnectInfo>()
                .and_then(TcpConnectInfo::remote_addr)
                .map(|address| address.ip());
            let source = SourceKey::of_parts(peer, &parts.headers);
            Ok(
                LimitedMessageBody::new(body, self.large_limit).with_large_body_slots(
                    self.default_limit,
                    self.large_body_slots.clone(),
                    source,
                ),
            )
        } else {
            Ok(LimitedMessageBody::new(body, self.default_limit))
        }
    }
}

/// The large-body slots of the transaction methods: how many bodies past the
/// small limit may be open at once, in total and per source.
struct LargeBodySlots {
    max_total: usize,
    max_per_source: usize,
    held: Mutex<HeldLargeBodySlots>,
}

#[derive(Default)]
struct HeldLargeBodySlots {
    total: usize,
    by_source: HashMap<SourceKey, usize>,
}

impl LargeBodySlots {
    fn new(max_total: usize, max_per_source: usize) -> Self {
        Self {
            max_total,
            max_per_source,
            held: Mutex::default(),
        }
    }

    fn held(&self) -> MutexGuard<'_, HeldLargeBodySlots> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Takes a slot for `source`, refused when the source already holds its
    /// share or the node has none left. A request without a known source
    /// counts against the total only.
    fn claim(self: &Arc<Self>, source: Option<SourceKey>) -> Result<LargeBodySlot, Status> {
        let mut held = self.held();
        if let Some(source) = source
            && held.by_source.get(&source).copied().unwrap_or(0) >= self.max_per_source
        {
            return Err(Status::resource_exhausted(
                "too many large requests in flight from this address; retry later",
            ));
        }
        if held.total >= self.max_total {
            return Err(Status::resource_exhausted(
                "too many large requests in flight on this node; retry later",
            ));
        }
        held.total += 1;
        if let Some(source) = source {
            *held.by_source.entry(source).or_default() += 1;
        }
        Ok(LargeBodySlot {
            slots: self.clone(),
            source,
        })
    }
}

/// A large-body slot shared by the request body that claims it and the call
/// that consumes the request: tonic drops a unary request body as soon as the
/// message is decoded, before the handler runs, while the handler still holds
/// the transaction, so the slot is given back only once both are gone.
type LargeBodyLease = Arc<OnceLock<LargeBodySlot>>;

/// A held large-body slot, given back when dropped.
struct LargeBodySlot {
    slots: Arc<LargeBodySlots>,
    source: Option<SourceKey>,
}

impl Drop for LargeBodySlot {
    fn drop(&mut self) {
        let mut held = self.slots.held();
        held.total = held.total.saturating_sub(1);
        if let Some(source) = self.source
            && let Entry::Occupied(mut count) = held.by_source.entry(source)
        {
            *count.get_mut() -= 1;
            if *count.get() == 0 {
                count.remove();
            }
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
    S::Future: Send + 'static,
    ReqBody: Body<Data = Bytes> + Send + Unpin + 'static,
    ReqBody::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let (parts, body) = req.into_parts();
        let (body, lease) = match self.config.limit_body(&parts, body) {
            Ok(limited) => {
                let lease = limited.large_body_lease();
                (TonicBody::new(limited), lease)
            }
            Err(body) => (TonicBody::new(body), None),
        };
        let call = self.inner.call(Request::from_parts(parts, body));
        Box::pin(async move {
            // Keeps any large-body slot the body takes until the call ends or
            // is cancelled, not only until tonic drops the body.
            let _lease = lease;
            call.await
        })
    }
}

/// The gRPC message header: one compression flag byte and a big-endian `u32`
/// message length.
const GRPC_MESSAGE_HEADER_LEN: usize = 5;

/// Where the framing walk of [`LimitedMessageBody`] stands in the stream of
/// gRPC messages: collecting the header of the next message, or inside a
/// message body with a known number of bytes to go.
enum MessageFraming {
    Header {
        bytes: [u8; GRPC_MESSAGE_HEADER_LEN],
        filled: usize,
    },
    Body {
        remaining: usize,
    },
}

impl MessageFraming {
    const fn next_header() -> Self {
        MessageFraming::Header {
            bytes: [0; GRPC_MESSAGE_HEADER_LEN],
            filled: 0,
        }
    }
}

/// The shared slot a body must hold once its messages grow past `threshold`.
struct LargeBodyClaim {
    threshold: usize,
    slots: Arc<LargeBodySlots>,
    source: Option<SourceKey>,
    lease: LargeBodyLease,
}

/// A request body bounded by a byte limit, checked on the declared length of
/// every gRPC message as soon as its header is in and on every byte delivered.
struct LimitedMessageBody<B> {
    inner: B,
    limit: usize,
    delivered: usize,
    framing: MessageFraming,
    large_body_claim: Option<LargeBodyClaim>,
}

impl<B> LimitedMessageBody<B> {
    fn new(inner: B, limit: usize) -> Self {
        Self {
            inner,
            limit,
            delivered: 0,
            framing: MessageFraming::next_header(),
            large_body_claim: None,
        }
    }

    /// Requires one of `slots` for `source` once a message header takes the
    /// body past `threshold` bytes, held for as long as the body or a clone of
    /// its [`LimitedMessageBody::large_body_lease`] lives.
    fn with_large_body_slots(
        mut self,
        threshold: usize,
        slots: Arc<LargeBodySlots>,
        source: Option<SourceKey>,
    ) -> Self {
        self.large_body_claim = Some(LargeBodyClaim {
            threshold,
            slots,
            source,
            lease: LargeBodyLease::default(),
        });
        self
    }

    /// The lease on the large-body slot this body takes, if it may need one.
    fn large_body_lease(&self) -> Option<LargeBodyLease> {
        self.large_body_claim
            .as_ref()
            .map(|claim| claim.lease.clone())
    }

    fn over_limit(limit: usize) -> Status {
        Status::resource_exhausted(format!(
            "request body exceeds the {limit} byte limit of this method"
        ))
    }

    /// Takes a large-body slot for a body that will end at `end` bytes, if it
    /// needs one and holds none yet.
    fn claim_large_body_slot(&mut self, end: usize) -> Result<(), Status> {
        let Some(claim) = &mut self.large_body_claim else {
            return Ok(());
        };
        if end <= claim.threshold || claim.lease.get().is_some() {
            return Ok(());
        }
        let slot = claim.slots.claim(claim.source)?;
        // Only this body sets the lease and it holds none yet, so the slot is
        // stored; were it not, dropping it would give it straight back.
        let _ = claim.lease.set(slot);
        Ok(())
    }

    /// Accounts for a delivered chunk: the running total first, then the walk
    /// through the gRPC framing so that every message header is checked the
    /// moment it is complete, whether it sits inside one chunk or is split
    /// across several.
    fn check_chunk(&mut self, chunk: &Bytes) -> Result<(), Status> {
        // Bytes accounted by the framing walk, so a header completing mid-chunk
        // is judged on what was delivered up to it.
        let mut consumed = self.delivered;
        self.delivered = self.delivered.saturating_add(chunk.len());
        if self.delivered > self.limit {
            return Err(Self::over_limit(self.limit));
        }
        let mut rest: &[u8] = chunk;
        while !rest.is_empty() {
            let next = match &mut self.framing {
                MessageFraming::Header { bytes, filled } => {
                    let take = rest.len().min(GRPC_MESSAGE_HEADER_LEN - *filled);
                    bytes[*filled..*filled + take].copy_from_slice(&rest[..take]);
                    *filled += take;
                    rest = &rest[take..];
                    consumed = consumed.saturating_add(take);
                    if *filled < GRPC_MESSAGE_HEADER_LEN {
                        continue;
                    }
                    let declared =
                        u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
                    // Everything delivered up to this header plus the whole
                    // declared body has to fit: a message that cannot complete
                    // within the limit is refused before tonic reserves
                    // `declared` bytes for it.
                    let end = consumed.saturating_add(declared);
                    if end > self.limit {
                        return Err(Self::over_limit(self.limit));
                    }
                    self.claim_large_body_slot(end)?;
                    MessageFraming::Body {
                        remaining: declared,
                    }
                }
                MessageFraming::Body { remaining } => {
                    let take = rest.len().min(*remaining);
                    *remaining -= take;
                    rest = &rest[take..];
                    consumed = consumed.saturating_add(take);
                    if *remaining > 0 {
                        continue;
                    }
                    MessageFraming::next_header()
                }
            };
            self.framing = next;
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
    use dapi_grpc::tonic::client::Grpc as GrpcClient;
    use dapi_grpc::tonic::server::{Grpc as GrpcServer, NamedService, UnaryService};
    use dapi_grpc::tonic::transport::server::TcpIncoming;
    use dapi_grpc::tonic::transport::{Channel, Server};
    use dapi_grpc::tonic::{Code, Request as TonicRequest, Response as TonicResponse};
    use dapi_grpc::tonic_prost::ProstCodec;
    use http_body_util::BodyExt;
    use std::net::SocketAddr;
    use std::task::{Context, Poll};
    use tokio::net::TcpListener;
    use tokio::sync::{Notify, Semaphore};

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
            MAX_CONCURRENT_LARGE_TRANSACTION_BODIES,
            MAX_CONCURRENT_LARGE_TRANSACTION_BODIES_PER_SOURCE,
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

    /// A request body that delivers `bytes` in one DATA frame and then stays open.
    fn open_body(bytes: Vec<u8>) -> impl Body<Data = Bytes, Error = Status> + Unpin {
        http_body_util::StreamBody::new(futures::StreamExt::chain(
            futures::stream::iter([Ok(Frame::data(Bytes::from(bytes)))]),
            futures::stream::pending(),
        ))
    }

    /// The request parts of a transaction call from `peer`, forwarded for `forwarded_for`.
    fn transaction_parts(peer: Option<&str>, forwarded_for: Option<&str>) -> Parts {
        let mut request = Request::builder().uri(TRANSACTION_PATH);
        if let Some(forwarded_for) = forwarded_for {
            request = request.header("x-forwarded-for", forwarded_for);
        }
        let (mut parts, ()) = request.body(()).expect("request").into_parts();
        if let Some(peer) = peer {
            parts.extensions.insert(TcpConnectInfo {
                local_addr: None,
                remote_addr: Some(peer.parse().expect("peer address")),
            });
        }
        parts
    }

    /// A large gRPC header, sent on a transaction body that then stays open.
    fn open_large_body(
        layer: &BodyLimitLayer,
        parts: &Parts,
    ) -> LimitedMessageBody<impl Body<Data = Bytes, Error = Status> + Unpin> {
        let Ok(body) = layer.limit_body(parts, open_body(grpc_message(30 * 1024 * 1024, 0))) else {
            panic!("the transaction method is limited");
        };
        body
    }

    /// Large transaction bodies share a fixed number of slots: a header that takes the body
    /// past the query allowance needs one while the body lives, a request that finds none
    /// left is refused on that header, a body within the query allowance never needs one, and
    /// dropping a body frees its slot.
    #[tokio::test]
    async fn large_transaction_bodies_share_a_bounded_number_of_slots() {
        let layer = BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
            1,
            1,
        );
        let parts = transaction_parts(Some("203.0.113.7:4000"), None);
        let open = |bytes: Vec<u8>| {
            let Ok(body) = layer.limit_body(&parts, open_body(bytes)) else {
                panic!("the transaction method is limited");
            };
            body
        };
        let large_header = || grpc_message(30 * 1024 * 1024, 0);

        let mut first = open(large_header());
        first
            .frame()
            .await
            .expect("a frame")
            .expect("the first large body takes the slot");

        let status = open(large_header())
            .frame()
            .await
            .expect("a frame")
            .expect_err("no slot is left for a second one");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
        assert!(
            status.message().contains("too many large requests"),
            "{status:?}"
        );

        // A body ending exactly at the query allowance needs no slot, one byte more does.
        let at_threshold = MAX_PLATFORM_QUERY_BODY_BYTES - GRPC_MESSAGE_HEADER_LEN;
        open(grpc_message(at_threshold, 0))
            .frame()
            .await
            .expect("a frame")
            .expect("a body within the query allowance needs no slot");
        open(grpc_message(at_threshold + 1, 0))
            .frame()
            .await
            .expect("a frame")
            .expect_err("a body past the query allowance needs a slot");

        drop(first);
        open(large_header())
            .frame()
            .await
            .expect("a frame")
            .expect("the dropped body gave its slot back");
    }

    /// One source holds at most its share of the large-body slots while another source still
    /// gets one, the node-wide total still applies across sources and to a request with no
    /// known source, a gateway's forwarded client is the source rather than the gateway, and
    /// a dropped body gives its source's share back.
    #[tokio::test]
    async fn large_transaction_bodies_are_capped_per_source() {
        let layer = BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
            3,
            2,
        );
        let flooder = transaction_parts(Some("203.0.113.7:4000"), None);
        let refused = |status: Status, reason: &str| {
            assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
            assert!(status.message().contains(reason), "{status:?}");
        };

        let mut first = open_large_body(&layer, &flooder);
        first.frame().await.expect("a frame").expect("first slot");
        let mut second = open_large_body(&layer, &flooder);
        second.frame().await.expect("a frame").expect("second slot");
        refused(
            open_large_body(&layer, &flooder)
                .frame()
                .await
                .expect("a frame")
                .expect_err("the source has used its share"),
            "from this address",
        );

        // Behind the gateway the forwarded client is the source, not the gateway.
        let publisher = transaction_parts(Some("10.0.0.2:4000"), Some("198.51.100.9"));
        let mut third = open_large_body(&layer, &publisher);
        third
            .frame()
            .await
            .expect("a frame")
            .expect("another source still gets a slot");
        refused(
            open_large_body(&layer, &transaction_parts(Some("192.0.2.1:4000"), None))
                .frame()
                .await
                .expect("a frame")
                .expect_err("the node-wide total still applies"),
            "on this node",
        );
        refused(
            open_large_body(&layer, &transaction_parts(None, None))
                .frame()
                .await
                .expect("a frame")
                .expect_err("a request with no known source counts against the total"),
            "on this node",
        );

        drop(first);
        open_large_body(&layer, &flooder)
            .frame()
            .await
            .expect("a frame")
            .expect("the dropped body gave its source's share back");
        drop((second, third));
    }

    /// Sends the body as the given chunks, one DATA frame each.
    async fn send_chunks(path: &str, chunks: Vec<Vec<u8>>) -> Result<usize, Status> {
        use http_body_util::StreamBody;

        let frames = futures::stream::iter(
            chunks
                .into_iter()
                .map(|chunk| Ok::<_, Status>(Frame::data(Bytes::from(chunk)))),
        );
        let mut service = body_limit_layer().layer(BodyLengthService);
        let request = Request::builder()
            .uri(path)
            .body(axum::body::Body::new(StreamBody::new(frames)))
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

    /// Every message header is checked, not only the first: tonic's unary path drains the
    /// messages after the first while looking for the trailers and reserves a receive buffer
    /// for each of them, so a small first message followed by a header declaring a
    /// transaction-sized second message must be refused on those bytes, whether the second
    /// header arrives in the same DATA frame as the first message or split across frames.
    #[tokio::test]
    async fn body_limit_checks_every_message_header() {
        let first = grpc_message(0, 0);
        let second_header = grpc_message(30 * 1024 * 1024, 0);

        // Both headers in one DATA frame.
        let mut one_frame = first.clone();
        one_frame.extend_from_slice(&second_header);
        let status = send_body(QUERY_PATH, one_frame.clone())
            .await
            .expect_err("a second message over the limit is refused");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);
        assert!(
            status
                .message()
                .contains(&MAX_PLATFORM_QUERY_BODY_BYTES.to_string()),
            "{status:?}"
        );

        // The second header split across frames, two bytes then three.
        let status = send_chunks(
            QUERY_PATH,
            vec![
                first.clone(),
                second_header[..2].to_vec(),
                second_header[2..].to_vec(),
            ],
        )
        .await
        .expect_err("a second header split across frames is refused too");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);

        // A first message whose body is split across frames is walked correctly: the second
        // header is found where the declared body ends, not at a frame boundary.
        let mut body = grpc_message(7, 7);
        body.extend_from_slice(&second_header);
        let status = send_chunks(QUERY_PATH, vec![body[..3].to_vec(), body[3..].to_vec()])
            .await
            .expect_err("the second header is found after the declared body");
        assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted);

        // Two small messages are fine, in one frame or in several, and the transaction
        // ingress admits a large second declaration.
        let mut two_small = grpc_message(7, 7);
        two_small.extend_from_slice(&grpc_message(9, 9));
        assert_eq!(
            send_body(QUERY_PATH, two_small.clone())
                .await
                .expect("two small messages pass"),
            two_small.len()
        );
        assert_eq!(
            send_chunks(
                QUERY_PATH,
                two_small.chunks(4).map(<[u8]>::to_vec).collect()
            )
            .await
            .expect("two small messages split across frames pass"),
            two_small.len()
        );
        assert_eq!(
            send_body(TRANSACTION_PATH, one_frame.clone())
                .await
                .expect("the ingress admits a large second declaration"),
            one_frame.len()
        );
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

    /// A gRPC service on the Platform path whose every method takes raw bytes and answers with
    /// their length. With a gate, the handler holds the request until it gets a permit.
    #[derive(Clone, Default)]
    struct ByteCounter {
        gate: Option<Arc<Semaphore>>,
        entered: Arc<Notify>,
    }

    impl NamedService for ByteCounter {
        const NAME: &'static str = "org.dash.platform.dapi.v0.Platform";
    }

    impl UnaryService<Vec<u8>> for ByteCounter {
        type Response = u64;
        type Future =
            Pin<Box<dyn Future<Output = Result<TonicResponse<u64>, Status>> + Send + 'static>>;

        fn call(&mut self, request: TonicRequest<Vec<u8>>) -> Self::Future {
            let gate = self.gate.clone();
            let entered = self.entered.clone();
            Box::pin(async move {
                entered.notify_one();
                if let Some(gate) = gate {
                    gate.acquire().await.expect("the gate stays open").forget();
                }
                Ok(TonicResponse::new(request.into_inner().len() as u64))
            })
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

    /// Serves `service` behind `layer` on a local port, with a client connected to it.
    async fn serve(
        layer: BodyLimitLayer,
        service: ByteCounter,
    ) -> (SocketAddr, GrpcClient<Channel>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("local address");
        tokio::spawn(async move {
            Server::builder()
                .layer(layer)
                .add_service(service)
                .serve_with_incoming(TcpIncoming::from(listener))
                .await
                .expect("test server");
        });
        let channel = Channel::from_shared(format!("http://{address}"))
            .expect("endpoint")
            .connect()
            .await
            .expect("connect");
        let client = GrpcClient::new(channel)
            .max_encoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES)
            .max_decoding_message_size(MAX_PLATFORM_TRANSACTION_BODY_BYTES);
        (address, client)
    }

    /// One unary call of `path` carrying `payload`, answered with the byte count.
    async fn call_unary(
        client: &mut GrpcClient<Channel>,
        path: &str,
        payload: Vec<u8>,
    ) -> Result<u64, Status> {
        client.ready().await.expect("client ready");
        client
            .unary(
                TonicRequest::new(payload),
                path.parse().expect("path"),
                ProstCodec::<Vec<u8>, u64>::default(),
            )
            .await
            .map(|response| response.into_inner())
    }

    fn family_cap() -> usize {
        dpp::version::PlatformVersion::latest()
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes") as usize
    }

    /// Through a real tonic server and HTTP/2 client: a `getPathElements` body above the query
    /// allowance is refused by the layer (ResourceExhausted, naming the limit) even though it
    /// is far below tonic's service-wide decode cap, and a broadcast carrying a family-cap
    /// transition reaches the service intact. The service behind the layer is a plain gRPC
    /// unary handler that answers with the byte count it received, so the assertion is on what
    /// crossed the layer, not on Platform semantics.
    #[tokio::test]
    async fn real_server_applies_the_per_method_body_limit() {
        // One large-body slot, so the second family-cap call below only passes if the server
        // gives the slot back once the first call is done.
        let layer = BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
            1,
            1,
        );
        let (address, mut client) = serve(layer, ByteCounter::default()).await;
        let mut call =
            async |path: &str, payload: Vec<u8>| call_unary(&mut client, path, payload).await;

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
        let family_cap = family_cap();
        for _ in 0..2 {
            assert_eq!(
                call(TRANSACTION_PATH, vec![0x5Au8; family_cap])
                    .await
                    .expect("a family-cap transition reaches the service"),
                family_cap as u64
            );
        }

        // Through the same server, a raw request body carrying an empty first message and
        // then the header of a 30 MiB second message, with the stream left open: the unary
        // handler reads the first message and drains the rest while looking for the trailers,
        // so without the per-message header check tonic would reserve 30 MiB and wait for the
        // body. The layer refuses the request on those ten bytes and the response comes back
        // at once, without the stream ever closing.
        {
            use dapi_grpc::tonic::body::Body as GrpcBody;
            use http_body_util::StreamBody;
            use tower::ServiceExt;

            let mut open_ended = grpc_message(0, 0);
            open_ended.extend_from_slice(&grpc_message(30 * 1024 * 1024, 0));
            let frames = futures::StreamExt::chain(
                futures::stream::iter([Ok::<_, Status>(Frame::data(Bytes::from(open_ended)))]),
                futures::stream::pending(),
            );
            let request = Request::builder()
                .method("POST")
                .uri(format!("http://{address}{QUERY_PATH}"))
                .header("content-type", "application/grpc")
                .header("te", "trailers")
                .body(GrpcBody::new(StreamBody::new(frames)))
                .expect("request");
            let mut raw = Channel::from_shared(format!("http://{address}"))
                .expect("endpoint")
                .connect()
                .await
                .expect("connect");
            let response = tokio::time::timeout(
                Duration::from_secs(5),
                raw.ready().await.expect("channel ready").call(request),
            )
            .await
            .expect("the request is answered while the stream is still open")
            .expect("transport");
            let status = Status::from_header_map(response.headers())
                .expect("a trailers-only response carries the status");
            assert_eq!(status.code(), Code::ResourceExhausted, "{status:?}");
            assert!(
                status
                    .message()
                    .contains(&MAX_PLATFORM_QUERY_BODY_BYTES.to_string()),
                "{status:?}"
            );
        }
    }

    /// Through a real tonic server: tonic drops a unary request body once the message is
    /// decoded, before the handler runs, so a large broadcast whose handler is still working
    /// (waiting on Tenderdash, say) keeps its large-body slot. With one slot, a second large
    /// broadcast is refused while the first handler is blocked, and passes once it is done.
    #[tokio::test]
    async fn real_server_holds_the_large_body_slot_until_the_call_ends() {
        let layer = BodyLimitLayer::new(
            PLATFORM_SERVICE_PATH_PREFIX,
            MAX_PLATFORM_QUERY_BODY_BYTES,
            MAX_PLATFORM_TRANSACTION_BODY_BYTES,
            PLATFORM_TRANSACTION_METHODS,
            1,
            1,
        );
        let gate = Arc::new(Semaphore::new(0));
        let service = ByteCounter {
            gate: Some(gate.clone()),
            ..ByteCounter::default()
        };
        let entered = service.entered.clone();
        let (_, client) = serve(layer, service).await;
        let family_cap = family_cap();

        let mut first_client = client.clone();
        let first = tokio::spawn(async move {
            call_unary(
                &mut first_client,
                TRANSACTION_PATH,
                vec![0x5Au8; family_cap],
            )
            .await
        });
        entered.notified().await;

        let mut second_client = client.clone();
        let status = tokio::time::timeout(
            Duration::from_secs(5),
            call_unary(
                &mut second_client,
                TRANSACTION_PATH,
                vec![0x5Au8; family_cap],
            ),
        )
        .await
        .expect("the second broadcast is refused instead of waiting in the handler")
        .expect_err("no slot is left while the first handler runs");
        assert_eq!(status.code(), Code::ResourceExhausted, "{status:?}");
        assert!(
            status.message().contains("too many large requests"),
            "{status:?}"
        );

        gate.add_permits(2);
        assert_eq!(
            first
                .await
                .expect("the first call task")
                .expect("the first broadcast completes"),
            family_cap as u64
        );
        let mut third_client = client.clone();
        assert_eq!(
            call_unary(
                &mut third_client,
                TRANSACTION_PATH,
                vec![0x5Au8; family_cap]
            )
            .await
            .expect("the finished call gave its slot back"),
            family_cap as u64
        );
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
