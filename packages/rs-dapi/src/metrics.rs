use once_cell::sync::Lazy;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, Encoder, IntCounter,
    IntCounterVec, IntGauge, TextEncoder,
};

/// Enum for all metric names used in rs-dapi
#[derive(Copy, Clone, Debug)]
pub enum Metric {
    /// Cache events counter: labels [method, outcome]
    CacheEvent,
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
}

impl Metric {
    pub const fn name(self) -> &'static str {
        match self {
            Metric::CacheEvent => "rsdapi_cache_events_total",
            Metric::PlatformEventsActiveSessions => "rsdapi_platform_events_active_sessions",
            Metric::PlatformEventsCommands => "rsdapi_platform_events_commands_total",
            Metric::PlatformEventsForwardedEvents => "rsdapi_platform_events_forwarded_events_total",
            Metric::PlatformEventsForwardedAcks => "rsdapi_platform_events_forwarded_acks_total",
            Metric::PlatformEventsForwardedErrors => "rsdapi_platform_events_forwarded_errors_total",
            Metric::PlatformEventsUpstreamStreams => "rsdapi_platform_events_upstream_streams_total",
        }
    }

    pub const fn help(self) -> &'static str {
        match self {
            Metric::CacheEvent => "Cache events by method and outcome (hit|miss)",
            Metric::PlatformEventsActiveSessions => "Current number of active Platform events sessions",
            Metric::PlatformEventsCommands => "Platform events commands processed by operation",
            Metric::PlatformEventsForwardedEvents => "Platform events forwarded to clients",
            Metric::PlatformEventsForwardedAcks => "Platform acks forwarded to clients",
            Metric::PlatformEventsForwardedErrors => "Platform errors forwarded to clients",
            Metric::PlatformEventsUpstreamStreams => "Upstream subscribePlatformEvents streams started",
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
    Method,
    Outcome,
    Op,
}

impl Label {
    pub const fn name(self) -> &'static str {
        match self {
            Label::Method => "method",
            Label::Outcome => "outcome",
            Label::Op => "op",
        }
    }
}

pub static CACHE_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Metric::CacheEvent.name(),
        Metric::CacheEvent.help(),
        &[Label::Method.name(), Label::Outcome.name()]
    )
    .expect("create counter")
});

pub static PLATFORM_EVENTS_ACTIVE_SESSIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        Metric::PlatformEventsActiveSessions.name(),
        Metric::PlatformEventsActiveSessions.help()
    )
    .expect("create gauge")
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

/// Root typed accessor for metrics
pub struct Metrics;

impl Metrics {
    /// Increment cache events counter with explicit outcome
    #[inline]
    pub fn cache_events_inc(method: &str, outcome: Outcome) {
        CACHE_EVENTS
            .with_label_values(&[method, outcome.as_str()])
            .inc();
    }

    /// Mark cache hit for method
    #[inline]
    pub fn cache_events_hit(method: &str) {
        Self::cache_events_inc(method, Outcome::Hit);
    }

    /// Mark cache miss for method
    #[inline]
    pub fn cache_events_miss(method: &str) {
        Self::cache_events_inc(method, Outcome::Miss);
    }
}

#[inline]
pub fn record_cache_event(method: &str, outcome: Outcome) {
    CACHE_EVENTS
        .with_label_values(&[method, outcome.as_str()])
        .inc();
}

#[inline]
pub fn cache_hit(method: &str) {
    record_cache_event(method, Outcome::Hit);
}

#[inline]
pub fn cache_miss(method: &str) {
    record_cache_event(method, Outcome::Miss);
}

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
pub fn platform_events_command(op: &str) {
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
