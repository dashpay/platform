use axum::http::{Extensions, Request, Response};
use once_cell::sync::Lazy;
use prometheus::{
    Encoder, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, TextEncoder,
    register_histogram_vec, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec,
};
use std::any::type_name_of_val;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

use crate::logging::middleware::{
    detect_protocol_type, extract_grpc_status, http_status_to_grpc_status,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MethodLabel(&'static str);

impl MethodLabel {
    pub fn from_type_name(name: &'static str) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl fmt::Display for MethodLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn method_label<T>(value: &T) -> MethodLabel {
    MethodLabel::from_type_name(type_name_of_val(value))
}

pub fn attach_method_label(extensions: &mut Extensions, method: MethodLabel) {
    extensions.insert(method);
}

/// Enum for all metric names used in rs-dapi
#[derive(Copy, Clone, Debug)]
pub enum Metric {
    /// Cache events counter: labels [cache, method, outcome]
    CacheEvent,
    /// Cache memory usage gauge
    CacheMemoryUsage,
    /// Cache memory capacity gauge
    CacheMemoryCapacity,
    /// Cache entries gauge
    CacheEntries,
    /// Requests counter: labels [protocol, endpoint, status]
    RequestCount,
    /// Request duration histogram: labels [protocol, endpoint, status]
    RequestDuration,
    /// Platform events: active sessions gauge
    PlatformEventsActiveSessions,
    /// Platform events: commands processed, labels [op]
    PlatformEventsCommands,
    /// Platform events: forwarded events counter
    PlatformEventsForwardedEvents,
    /// Platform events: forwarded acks counter
    PlatformEventsForwardedAcks,
    /// Platform events: forwarded errors counter
    PlatformEventsForwardedErrors,
    /// Platform events: upstream streams started counter
    PlatformEventsUpstreamStreams,
    /// Active worker tasks gauge
    WorkersActive,
}

impl Metric {
    /// Return the Prometheus metric name associated with this enum variant.
    pub const fn name(self) -> &'static str {
        match self {
            Metric::CacheEvent => "rsdapi_cache_events_total",
            Metric::CacheMemoryUsage => "rsdapi_cache_memory_usage_bytes",
            Metric::CacheMemoryCapacity => "rsdapi_cache_memory_capacity_bytes",
            Metric::CacheEntries => "rsdapi_cache_entries",
            Metric::RequestCount => "rsdapi_requests_total",
            Metric::RequestDuration => "rsdapi_request_duration_seconds",
            Metric::PlatformEventsActiveSessions => "rsdapi_platform_events_active_sessions",
            Metric::PlatformEventsCommands => "rsdapi_platform_events_commands_total",
            Metric::PlatformEventsForwardedEvents => {
                "rsdapi_platform_events_forwarded_events_total"
            }
            Metric::PlatformEventsForwardedAcks => "rsdapi_platform_events_forwarded_acks_total",
            Metric::PlatformEventsForwardedErrors => {
                "rsdapi_platform_events_forwarded_errors_total"
            }
            Metric::PlatformEventsUpstreamStreams => {
                "rsdapi_platform_events_upstream_streams_total"
            }
            Metric::WorkersActive => "rsdapi_workers_active_tasks",
        }
    }

    /// Return the human-readable help string for the Prometheus metric.
    pub const fn help(self) -> &'static str {
        match self {
            Metric::CacheEvent => "Cache events by method and outcome (hit|miss)",
            Metric::CacheMemoryUsage => "Approximate cache memory usage in bytes",
            Metric::CacheMemoryCapacity => "Configured cache memory capacity in bytes",
            Metric::CacheEntries => "Number of items currently stored in the cache",
            Metric::RequestCount => "Requests received by protocol, endpoint, and status",
            Metric::RequestDuration => {
                "Request latency in seconds by protocol, endpoint, and status"
            }
            Metric::PlatformEventsActiveSessions => {
                "Current number of active Platform events sessions"
            }
            Metric::PlatformEventsCommands => "Platform events commands processed by operation",
            Metric::PlatformEventsForwardedEvents => "Platform events forwarded to clients",
            Metric::PlatformEventsForwardedAcks => "Platform acks forwarded to clients",
            Metric::PlatformEventsForwardedErrors => "Platform errors forwarded to clients",
            Metric::PlatformEventsUpstreamStreams => {
                "Upstream subscribePlatformEvents streams started"
            }
            Metric::WorkersActive => "Current number of active background worker tasks",
        }
    }
}

/// Outcome label values for cache events
#[derive(Copy, Clone, Debug)]
pub enum Outcome {
    Hit,
    Miss,
}

impl Outcome {
    /// Convert the outcome into a label-friendly string literal.
    pub const fn as_str(self) -> &'static str {
        match self {
            Outcome::Hit => "hit",
            Outcome::Miss => "miss",
        }
    }
}

/// Label keys used across metrics
#[derive(Copy, Clone, Debug)]
pub enum Label {
    Cache,
    Method,
    Outcome,
    Protocol,
    // TODO: ensure we have a limited set of endpoints, so that cardinality is controlled and we don't overload Prometheus
    Endpoint,
    Status,
    Op,
}

impl Label {
    /// Return the label key used in Prometheus metrics.
    pub const fn name(self) -> &'static str {
        match self {
            Label::Cache => "cache",
            Label::Method => "method",
            Label::Outcome => "outcome",
            Label::Protocol => "protocol",
            Label::Endpoint => "endpoint",
            Label::Status => "status",
            Label::Op => "op",
        }
    }
}

pub static CACHE_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Metric::CacheEvent.name(),
        Metric::CacheEvent.help(),
        &[
            Label::Cache.name(),
            Label::Method.name(),
            Label::Outcome.name()
        ]
    )
    .expect("create counter")
});

pub static CACHE_MEMORY_USAGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        Metric::CacheMemoryUsage.name(),
        Metric::CacheMemoryUsage.help(),
        &[Label::Cache.name()]
    )
    .expect("create gauge")
});

pub static CACHE_MEMORY_CAPACITY: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        Metric::CacheMemoryCapacity.name(),
        Metric::CacheMemoryCapacity.help(),
        &[Label::Cache.name()]
    )
    .expect("create gauge")
});

pub static CACHE_ENTRIES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        Metric::CacheEntries.name(),
        Metric::CacheEntries.help(),
        &[Label::Cache.name()]
    )
    .expect("create gauge")
});

pub static PLATFORM_EVENTS_ACTIVE_SESSIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        Metric::PlatformEventsActiveSessions.name(),
        Metric::PlatformEventsActiveSessions.help()
    )
    .expect("create gauge")
});

pub static REQUEST_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Metric::RequestCount.name(),
        Metric::RequestCount.help(),
        &[
            Label::Protocol.name(),
            Label::Endpoint.name(),
            Label::Status.name()
        ]
    )
    .expect("create counter vec")
});

pub static REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        Metric::RequestDuration.name(),
        Metric::RequestDuration.help(),
        &[
            Label::Protocol.name(),
            Label::Endpoint.name(),
            Label::Status.name()
        ]
    )
    .expect("create histogram vec")
});

pub static PLATFORM_EVENTS_COMMANDS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Metric::PlatformEventsCommands.name(),
        Metric::PlatformEventsCommands.help(),
        &[Label::Op.name()]
    )
    .expect("create counter vec")
});

pub static PLATFORM_EVENTS_FORWARDED_EVENTS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        Metric::PlatformEventsForwardedEvents.name(),
        Metric::PlatformEventsForwardedEvents.help()
    )
    .expect("create counter")
});

pub static PLATFORM_EVENTS_FORWARDED_ACKS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        Metric::PlatformEventsForwardedAcks.name(),
        Metric::PlatformEventsForwardedAcks.help()
    )
    .expect("create counter")
});

pub static PLATFORM_EVENTS_FORWARDED_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        Metric::PlatformEventsForwardedErrors.name(),
        Metric::PlatformEventsForwardedErrors.help()
    )
    .expect("create counter")
});

pub static PLATFORM_EVENTS_UPSTREAM_STREAMS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        Metric::PlatformEventsUpstreamStreams.name(),
        Metric::PlatformEventsUpstreamStreams.help()
    )
    .expect("create counter")
});

pub static WORKERS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(Metric::WorkersActive.name(), Metric::WorkersActive.help())
        .expect("create gauge")
});

/// Root typed accessor for metrics
pub struct Metrics;

impl Metrics {
    /// Increment cache events counter with explicit outcome
    #[inline]
    pub fn cache_events_inc(cache: &str, method: &MethodLabel, outcome: Outcome) {
        CACHE_EVENTS
            .with_label_values(&[cache, method.as_str(), outcome.as_str()])
            .inc();
    }

    /// Mark cache hit for method
    #[inline]
    pub fn cache_events_hit(cache: &str, method: &MethodLabel) {
        Self::cache_events_inc(cache, method, Outcome::Hit);
    }

    /// Mark cache miss for method
    #[inline]
    pub fn cache_events_miss(cache: &str, method: &MethodLabel) {
        Self::cache_events_inc(cache, method, Outcome::Miss);
    }
}

#[inline]
pub fn record_cache_event(cache: &str, method: &MethodLabel, outcome: Outcome) {
    CACHE_EVENTS
        .with_label_values(&[cache, method.as_str(), outcome.as_str()])
        .inc();
}

#[inline]
pub fn cache_hit(cache: &str, method: &MethodLabel) {
    record_cache_event(cache, method, Outcome::Hit);
}

#[inline]
pub fn cache_miss(cache: &str, method: &MethodLabel) {
    record_cache_event(cache, method, Outcome::Miss);
}

#[inline]
fn clamp_to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[inline]
pub fn cache_memory_usage_bytes(cache: &str, bytes: u64) {
    CACHE_MEMORY_USAGE
        .with_label_values(&[cache])
        .set(clamp_to_i64(bytes));
}

#[inline]
pub fn cache_memory_capacity_bytes(cache: &str, bytes: u64) {
    CACHE_MEMORY_CAPACITY
        .with_label_values(&[cache])
        .set(clamp_to_i64(bytes));
}

#[inline]
pub fn cache_entries(cache: &str, entries: usize) {
    CACHE_ENTRIES
        .with_label_values(&[cache])
        .set(clamp_to_i64(entries as u64));
}

#[inline]
pub fn requests_inc(protocol: &str, endpoint: &str, status: &str) {
    REQUEST_COUNTER
        .with_label_values(&[protocol, endpoint, status])
        .inc();
}

#[inline]
pub fn request_duration_observe(protocol: &str, endpoint: &str, status: &str, seconds: f64) {
    REQUEST_DURATION_SECONDS
        .with_label_values(&[protocol, endpoint, status])
        .observe(seconds);
}

/// Gather Prometheus metrics into an encoded buffer and its corresponding content type.
pub fn gather_prometheus() -> (Vec<u8>, String) {
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .unwrap_or_default();
    let content_type = encoder.format_type().to_string();
    (buffer, content_type)
}

// ---- Request metrics middleware ----

#[derive(Clone, Default)]
pub struct MetricsLayer;

impl MetricsLayer {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, service: S) -> Self::Service {
        MetricsService { inner: service }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for MetricsService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start_time = Instant::now();
        let protocol_type = detect_protocol_type(&req);
        let path = req.uri().path().to_string();
        let request_method_hint = req.extensions().get::<MethodLabel>().cloned();

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let result = inner.call(req).await;
            match result {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let status = response.status().as_u16();
                    let method_hint = response.extensions().get::<MethodLabel>();
                    let endpoint_label = endpoint_label(
                        &protocol_type,
                        &path,
                        method_hint.or(request_method_hint.as_ref()),
                    );

                    let status_code = if protocol_type == "gRPC" {
                        extract_grpc_status(&response, status)
                    } else {
                        http_status_to_grpc_status(status)
                    };
                    let status_label = status_code.to_string();

                    requests_inc(
                        protocol_type.as_str(),
                        endpoint_label.as_str(),
                        status_label.as_str(),
                    );
                    request_duration_observe(
                        protocol_type.as_str(),
                        endpoint_label.as_str(),
                        status_label.as_str(),
                        duration.as_secs_f64(),
                    );

                    Ok(response)
                }
                Err(err) => {
                    let duration = start_time.elapsed();
                    let endpoint_label =
                        endpoint_label(&protocol_type, &path, request_method_hint.as_ref());
                    let status_label = http_status_to_grpc_status(500).to_string();

                    requests_inc(
                        protocol_type.as_str(),
                        endpoint_label.as_str(),
                        status_label.as_str(),
                    );
                    request_duration_observe(
                        protocol_type.as_str(),
                        endpoint_label.as_str(),
                        status_label.as_str(),
                        duration.as_secs_f64(),
                    );

                    Err(err)
                }
            }
        })
    }
}

#[inline]
fn endpoint_label(protocol: &str, path: &str, method_hint: Option<&MethodLabel>) -> String {
    match protocol {
        "gRPC" => known_grpc_endpoint(path).to_string(),
        "JSON-RPC" => method_hint
            .map(MethodLabel::as_str)
            .unwrap_or("jsonrpc_unknown")
            .to_string(),
        _ => "http_unknown".to_string(),
    }
}

macro_rules! match_grpc_methods {
    ($path:expr, $service:literal, [$($method:literal),+ $(,)?]) => {{
        match $path {
            $(concat!("/", $service, "/", $method) => {
                Some(concat!($service, "/", $method))
            },)+
            _ => None,
        }
    }};
}

/// Return a label only for methods compiled into the two public Tonic services.
/// Every other syntactically valid or malformed path shares one finite bucket.
// KEEP IN SYNC with packages/dapi-grpc/protos/{core,platform}/v0/*.proto —
// an rpc missing here still serves fine but its metrics degrade to the
// `grpc_unknown` bucket. Enforced by
// `known_grpc_endpoint_covers_every_served_rpc`, which walks dapi-grpc's
// FILE_DESCRIPTOR_SET, so forgetting to extend this list fails CI.
fn known_grpc_endpoint(path: &str) -> &'static str {
    match_grpc_methods!(
        path,
        "org.dash.platform.dapi.v0.Core",
        [
            "getBlockchainStatus",
            "getMasternodeStatus",
            "getBlock",
            "getBestBlockHeight",
            "getEstimatedTransactionFee",
            "broadcastTransaction",
            "getTransaction",
            "subscribeToBlockHeadersWithChainLocks",
            "subscribeToTransactionsWithProofs",
            "subscribeToMasternodeList",
        ]
    )
    .or_else(|| {
        match_grpc_methods!(
            path,
            "org.dash.platform.dapi.v0.Platform",
            [
                "broadcastStateTransition",
                "getIdentity",
                "getIdentityKeys",
                "getIdentitiesContractKeys",
                "getIdentityNonce",
                "getIdentityContractNonce",
                "getIdentityBalance",
                "getIdentitiesBalances",
                "getIdentityBalanceAndRevision",
                "getEvonodesProposedEpochBlocksByIds",
                "getEvonodesProposedEpochBlocksByRange",
                "getDataContract",
                "getDataContractHistory",
                "getDataContracts",
                "getChainedDocuments",
                "getDocumentHistory",
                "getDocuments",
                "getIdentityByPublicKeyHash",
                "getIdentityByNonUniquePublicKeyHash",
                "waitForStateTransitionResult",
                "getConsensusParams",
                "getProtocolVersionUpgradeState",
                "getProtocolVersionUpgradeVoteStatus",
                "getEpochsInfo",
                "getFinalizedEpochInfos",
                "getContestedResources",
                "getContestedResourceVoteState",
                "getContestedResourceVotersForIdentity",
                "getContestedResourceIdentityVotes",
                "getVotePollsByEndDate",
                "getPrefundedSpecializedBalance",
                "getTotalCreditsInPlatform",
                "getPathElements",
                "getStatus",
                "getCurrentQuorumsInfo",
                "getIdentityTokenBalances",
                "getIdentitiesTokenBalances",
                "getIdentityTokenInfos",
                "getIdentitiesTokenInfos",
                "getTokenStatuses",
                "getTokenDirectPurchasePrices",
                "getTokenContractInfo",
                "getTokenPreProgrammedDistributions",
                "getTokenPerpetualDistributionLastClaim",
                "getTokenTotalSupply",
                "getGroupInfo",
                "getGroupInfos",
                "getGroupActions",
                "getGroupActionSigners",
                "getAddressInfo",
                "getAddressesInfos",
                "getAddressesTrunkState",
                "getAddressesBranchState",
                "getRecentAddressBalanceChanges",
                "getRecentCompactedAddressBalanceChanges",
                "getShieldedEncryptedNotes",
                "getShieldedAnchors",
                "getMostRecentShieldedAnchor",
                "getShieldedPoolState",
                "getShieldedNotesCount",
                "getShieldedNullifiers",
            ]
        )
    })
    .unwrap_or("grpc_unknown")
}

// ---- Platform events (proxy) helpers ----

#[inline]
pub fn platform_events_active_sessions_inc() {
    PLATFORM_EVENTS_ACTIVE_SESSIONS.inc();
}

#[inline]
pub fn platform_events_active_sessions_dec() {
    PLATFORM_EVENTS_ACTIVE_SESSIONS.dec();
}

#[inline]
pub fn platform_events_command(op: &'static str) {
    // `&'static str` keeps this label bounded by construction — request-derived
    // strings cannot reach the registry (same rationale as `MethodLabel`).
    PLATFORM_EVENTS_COMMANDS.with_label_values(&[op]).inc();
}

#[inline]
pub fn platform_events_forwarded_event() {
    PLATFORM_EVENTS_FORWARDED_EVENTS.inc();
}

#[inline]
pub fn platform_events_forwarded_ack() {
    PLATFORM_EVENTS_FORWARDED_ACKS.inc();
}

#[inline]
pub fn platform_events_forwarded_error() {
    PLATFORM_EVENTS_FORWARDED_ERRORS.inc();
}

#[inline]
pub fn platform_events_upstream_stream_started() {
    PLATFORM_EVENTS_UPSTREAM_STREAMS.inc();
}

#[inline]
pub fn workers_active_inc() {
    WORKERS_ACTIVE.inc();
}

#[inline]
pub fn workers_active_dec() {
    WORKERS_ACTIVE.dec();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_to_i64_within_range() {
        assert_eq!(clamp_to_i64(0), 0);
        assert_eq!(clamp_to_i64(1000), 1000);
        assert_eq!(clamp_to_i64(i64::MAX as u64), i64::MAX);
    }

    #[test]
    fn clamp_to_i64_above_range() {
        assert_eq!(clamp_to_i64(u64::MAX), i64::MAX);
        assert_eq!(clamp_to_i64(i64::MAX as u64 + 1), i64::MAX);
    }

    // Smoke test: verifies metric gathering does not panic and returns a valid content type
    #[test]
    fn gather_prometheus_returns_non_empty() {
        let (buffer, content_type) = gather_prometheus();
        // Buffer may be empty if no metrics have been touched yet, but should not panic
        assert!(!content_type.is_empty());
        let _ = buffer; // just ensure it doesn't panic
    }

    // Smoke test: verifies metric registration and label cardinality.
    // Intentionally only checks that calls do not panic; asserting recorded
    // values would couple tests to the global Prometheus registry state.
    #[test]
    fn cache_metrics_functions_do_not_panic() {
        let label = MethodLabel::from_type_name("test_method");
        cache_hit("test_cache", &label);
        cache_miss("test_cache", &label);
        cache_memory_usage_bytes("test_cache", 1024);
        cache_memory_capacity_bytes("test_cache", 2048);
        cache_entries("test_cache", 10);
    }

    // Smoke test: verifies metric registration and label cardinality.
    // Intentionally only checks that calls do not panic; asserting recorded
    // values would couple tests to the global Prometheus registry state.
    #[test]
    fn request_metrics_functions_do_not_panic() {
        requests_inc("gRPC", "test_endpoint", "0");
        request_duration_observe("gRPC", "test_endpoint", "0", 0.1);
    }

    // Smoke test: verifies metric registration and label cardinality.
    // Intentionally only checks that calls do not panic; asserting recorded
    // values would couple tests to the global Prometheus registry state.
    #[test]
    fn platform_events_metrics_do_not_panic() {
        platform_events_active_sessions_inc();
        platform_events_active_sessions_dec();
        platform_events_command("subscribe");
        platform_events_forwarded_event();
        platform_events_forwarded_ack();
        platform_events_forwarded_error();
        platform_events_upstream_stream_started();
    }

    // Smoke test: verifies metric registration and label cardinality.
    // Intentionally only checks that calls do not panic; asserting recorded
    // values would couple tests to the global Prometheus registry state.
    #[test]
    fn workers_active_metrics_do_not_panic() {
        workers_active_inc();
        workers_active_dec();
    }

    // -- MethodLabel tests --

    #[test]
    fn method_label_from_type_name() {
        let label = MethodLabel::from_type_name("GetStatusRequest");
        assert_eq!(label.as_str(), "GetStatusRequest");
        assert_eq!(format!("{}", label), "GetStatusRequest");
    }

    #[test]
    fn method_label_function() {
        let value = 42_u32;
        let label = method_label(&value);
        assert_eq!(label.as_str(), "u32");
    }

    #[test]
    fn attach_method_label_inserts_into_extensions() {
        let mut extensions = axum::http::Extensions::new();
        let label = MethodLabel::from_type_name("my_method");
        attach_method_label(&mut extensions, label);
        let retrieved = extensions
            .get::<MethodLabel>()
            .expect("MethodLabel should be in extensions");
        assert_eq!(retrieved.as_str(), "my_method");
    }

    // -- Metric enum tests --

    #[test]
    fn metric_names_are_prefixed() {
        assert!(Metric::CacheEvent.name().starts_with("rsdapi_"));
        assert!(Metric::RequestCount.name().starts_with("rsdapi_"));
        assert!(Metric::WorkersActive.name().starts_with("rsdapi_"));
    }

    #[test]
    fn metric_help_strings_are_nonempty() {
        assert!(!Metric::CacheEvent.help().is_empty());
        assert!(!Metric::RequestDuration.help().is_empty());
        assert!(!Metric::PlatformEventsActiveSessions.help().is_empty());
    }

    // -- Outcome enum tests --

    #[test]
    fn outcome_as_str() {
        assert_eq!(Outcome::Hit.as_str(), "hit");
        assert_eq!(Outcome::Miss.as_str(), "miss");
    }

    // -- Label enum tests --

    #[test]
    fn label_names() {
        assert_eq!(Label::Cache.name(), "cache");
        assert_eq!(Label::Method.name(), "method");
        assert_eq!(Label::Outcome.name(), "outcome");
        assert_eq!(Label::Protocol.name(), "protocol");
        assert_eq!(Label::Endpoint.name(), "endpoint");
        assert_eq!(Label::Status.name(), "status");
        assert_eq!(Label::Op.name(), "op");
    }

    // -- Metrics struct tests --

    #[test]
    fn metrics_struct_cache_events() {
        let label = MethodLabel::from_type_name("test");
        Metrics::cache_events_hit("struct_cache", &label);
        Metrics::cache_events_miss("struct_cache", &label);
        Metrics::cache_events_inc("struct_cache", &label, Outcome::Hit);
    }

    // -- endpoint_label tests --

    #[test]
    fn endpoint_label_grpc_with_hint() {
        let hint = MethodLabel::from_type_name("GetIdentity");
        let result = endpoint_label(
            "gRPC",
            "/org.dash.platform.dapi.v0.Platform/getIdentity",
            Some(&hint),
        );
        assert_eq!(result, "org.dash.platform.dapi.v0.Platform/getIdentity");
    }

    #[test]
    fn endpoint_label_grpc_without_hint_allowlists_path() {
        let result = endpoint_label(
            "gRPC",
            "/org.dash.platform.dapi.v0.Platform/getStatus",
            None,
        );
        assert_eq!(result, "org.dash.platform.dapi.v0.Platform/getStatus");

        let result = endpoint_label(
            "gRPC",
            "/org.dash.platform.dapi.v0.Core/getBlockchainStatus",
            None,
        );
        assert_eq!(result, "org.dash.platform.dapi.v0.Core/getBlockchainStatus");
    }

    #[test]
    fn endpoint_label_grpc_unknown_paths_share_one_bucket() {
        for path in [
            "/",
            "/org.dash.platform.dapi.v0.Core/UnknownMethod0001",
            "/org.dash.platform.dapi.v0.Platform/UnknownMethod0002",
            "/org.dash.platform.dapi.v0.Platform0003/getStatus",
        ] {
            assert_eq!(endpoint_label("gRPC", path, None), "grpc_unknown");
        }

        for suffix in 0..10_000 {
            let path = format!("/org.dash.platform.dapi.v0.Core/UnknownMethod{suffix:08}");
            assert_eq!(endpoint_label("gRPC", &path, None), "grpc_unknown");
        }
    }

    #[test]
    fn endpoint_label_jsonrpc_with_hint() {
        let hint = MethodLabel::from_type_name("getStatus");
        let result = endpoint_label("JSON-RPC", "/", Some(&hint));
        assert_eq!(result, "getStatus");
    }

    #[test]
    fn endpoint_label_jsonrpc_without_hint() {
        let result = endpoint_label("JSON-RPC", "/rpc", None);
        assert_eq!(result, "jsonrpc_unknown");
    }

    /// Walks dapi-grpc's FILE_DESCRIPTOR_SET and asserts every rpc of the two
    /// services this server exposes resolves to a real label. Adding an rpc
    /// to the protos without extending `known_grpc_endpoint` fails here
    /// instead of silently degrading the new method's metrics to
    /// `grpc_unknown`.
    #[test]
    fn known_grpc_endpoint_covers_every_served_rpc() {
        use dapi_grpc::Message;
        use prost_types::FileDescriptorSet;

        const SERVED_SERVICES: [&str; 2] = [
            "org.dash.platform.dapi.v0.Core",
            "org.dash.platform.dapi.v0.Platform",
        ];

        let mut seen_services = 0;
        for descriptor_bytes in [
            dapi_grpc::core::v0::FILE_DESCRIPTOR_SET,
            dapi_grpc::platform::v0::FILE_DESCRIPTOR_SET,
        ] {
            let set = FileDescriptorSet::decode(descriptor_bytes)
                .expect("dapi-grpc descriptor set should decode");
            for file in &set.file {
                for service in &file.service {
                    let service_full = format!("{}.{}", file.package(), service.name());
                    if !SERVED_SERVICES.contains(&service_full.as_str()) {
                        continue;
                    }
                    seen_services += 1;
                    assert!(
                        !service.method.is_empty(),
                        "descriptor for {service_full} lists no methods"
                    );
                    for method in &service.method {
                        let path = format!("/{}/{}", service_full, method.name());
                        assert_ne!(
                            known_grpc_endpoint(&path),
                            "grpc_unknown",
                            "{path} is served but missing from the known_grpc_endpoint \
                             allowlist — add it so its metrics don't degrade to grpc_unknown"
                        );
                    }
                }
            }
        }
        assert_eq!(
            seen_services, 2,
            "expected to find both served services in the descriptor sets"
        );
    }

    #[test]
    fn endpoint_label_http_paths_share_one_bucket() {
        for path in [
            "/health",
            "/org.dash.platform.dapi.v0.Platform0001",
            "/org.dash.platform.dapi.v0.Core0002",
        ] {
            assert_eq!(endpoint_label("HTTP", path, None), "http_unknown");
        }

        for suffix in 0..10_000 {
            let path = format!("/org.dash.platform.dapi.v0.Platform{suffix:08}");
            assert_eq!(endpoint_label("HTTP", &path, None), "http_unknown");
        }
    }

    // -- MetricsLayer tests --

    // Smoke test: verifies MetricsLayer can be constructed without panic
    #[test]
    fn metrics_layer_new() {
        let layer = MetricsLayer::new();
        // Just ensure it constructs without panic
        let _ = layer;
    }
}
