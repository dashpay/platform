use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, Encoder, IntCounterVec, TextEncoder};

/// Enum for all metric names used in rs-dapi
#[derive(Copy, Clone, Debug)]
pub enum Metric {
    /// Cache events counter: labels [method, outcome]
    CacheEvent,
}

impl Metric {
    pub const fn name(self) -> &'static str {
        match self {
            Metric::CacheEvent => "rsdapi_cache_events_total",
        }
    }

    pub const fn help(self) -> &'static str {
        match self {
            Metric::CacheEvent => "Cache events by method and outcome (hit|miss)",
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
}

impl Label {
    pub const fn name(self) -> &'static str {
        match self {
            Label::Method => "method",
            Label::Outcome => "outcome",
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
