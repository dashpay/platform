use once_cell::sync::Lazy;
use prometheus::{
    Encoder, IntCounter, IntCounterVec, IntGauge, TextEncoder, register_int_counter,
    register_int_counter_vec, register_int_gauge,
};

/// Enum for all metric names used in rs-dapi
#[derive(Copy, Clone, Debug)]
pub enum Metric {
    /// Cache events counter: labels [method, outcome]
    CacheEvent,
    /// Cache memory usage gauge
    CacheMemoryUsage,
    /// Cache memory capacity gauge
    CacheMemoryCapacity,
    /// Cache entries gauge
    CacheEntries,
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
    Method,
    Outcome,
    Op,
}

impl Label {
    /// Return the label key used in Prometheus metrics.
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

pub static CACHE_MEMORY_USAGE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        Metric::CacheMemoryUsage.name(),
        Metric::CacheMemoryUsage.help()
    )
    .expect("create gauge")
});

pub static CACHE_MEMORY_CAPACITY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        Metric::CacheMemoryCapacity.name(),
        Metric::CacheMemoryCapacity.help()
    )
    .expect("create gauge")
});

pub static CACHE_ENTRIES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(Metric::CacheEntries.name(), Metric::CacheEntries.help())
        .expect("create gauge")
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

pub static WORKERS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(Metric::WorkersActive.name(), Metric::WorkersActive.help())
        .expect("create gauge")
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

#[inline]
fn clamp_to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[inline]
pub fn cache_memory_usage_bytes(bytes: u64) {
    CACHE_MEMORY_USAGE.set(clamp_to_i64(bytes));
}

#[inline]
pub fn cache_memory_capacity_bytes(bytes: u64) {
    CACHE_MEMORY_CAPACITY.set(clamp_to_i64(bytes));
}

#[inline]
pub fn cache_entries(entries: usize) {
    CACHE_ENTRIES.set(clamp_to_i64(entries as u64));
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

#[inline]
pub fn workers_active_inc() {
    WORKERS_ACTIVE.inc();
}

#[inline]
pub fn workers_active_dec() {
    WORKERS_ACTIVE.dec();
}
