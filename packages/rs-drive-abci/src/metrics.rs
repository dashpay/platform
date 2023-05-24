//! # Metrics Module
//!
//! This module provides a singleton implementation for managing metrics.
//! It exposes an `instance()` function to get the current instance of the `Metrics` struct.

use std::{
    sync::{Arc, Once},
    time::Instant,
};

use lazy_static::lazy_static;
use metrics::{
    counter, describe_counter, describe_histogram, histogram, register_histogram, Histogram,
    HistogramFn, Key, Label,
};
use metrics_exporter_prometheus::PrometheusBuilder;

/// Default Prometheus port (29090)
pub const DEFAULT_PROMETHEUS_PORT: u16 = 29090;

/// Error returned by metrics subsystem
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Prometheus server failed
    #[error("prometheus server: {0}")]
    ServerFailed(#[from] metrics_exporter_prometheus::BuildError),
    /// Listen address invalid
    #[error("invalid listen address {0}: {1}")]
    InvalidListenAddress(url::Url, String),
}

/// Measure execution time and record as a metric.
///
/// `HistogramTiming` contains a metric key and a start time, and is designed to be used
/// with the Drop trait for automatic timing measurements.
///
/// When a `HistogramTiming` instance is dropped, [HistogramTiming::Drop()] method calculates and records the elapsed time
/// since the start time.
pub struct HistogramTiming {
    key: metrics::Key,
    start: Instant,
}

impl HistogramTiming {
    /// Creates a new `HistogramTiming` instance.
    ///
    /// # Arguments
    ///
    /// * `metric` - The metric key for the histogram.
    ///
    /// # Returns
    ///
    /// A new `HistogramTiming` instance with the given metric key and the current time as the start time.
    #[inline]
    fn new(metric: metrics::Key) -> Self {
        Self {
            key: metric,
            start: Instant::now(),
        }
    }
}

impl Drop for HistogramTiming {
    /// Implements the Drop trait for `HistogramTiming`.
    ///
    /// When a `HistogramTiming` instance is dropped, this method calculates and records the elapsed time
    /// since the start time.
    #[inline]
    fn drop(&mut self) {
        let stop = self.start.elapsed();
        let key = self.key.name().to_string();

        let labels: Vec<Label> = self.key.labels().map(|a| a.clone()).collect();
        histogram!(key, stop.as_secs_f64(), labels);
    }
}

struct HistogramWrapper(Arc<Histogram>);
impl HistogramFn for HistogramWrapper {
    fn record(&self, value: f64) {
        self.0.record(value)
    }
}

/// `Prometheus` is a struct that represents a Prometheus exporter server.
///
//
/// # Examples
///
/// ```
/// use drive_abci::metrics::Prometheus;
/// use url::Url;
///
/// let listen_address = Url::parse("http://127.0.0.1:9090").unwrap();
/// let prometheus = Prometheus::new(listen_address).unwrap();
/// ```
pub struct Prometheus {}

impl Prometheus {
    /// Creates and starts a new Prometheus server.
    ///
    /// # Arguments
    ///
    /// * `listen_address` - A `url::Url` representing the address the server should listen on.
    ///   The URL scheme must be "http". Any other scheme will result in an `Error::InvalidListenAddress`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use drive_abci::metrics::Prometheus;
    /// use url::Url;
    ///
    /// let listen_address = Url::parse("http://127.0.0.1:9090").unwrap();
    /// let prometheus = Prometheus::new(listen_address).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an `Error::InvalidListenAddress` if the provided `listen_address` has an unsupported scheme.
    ///
    /// # Default Port
    ///
    /// If the port number is not specified, it defaults to [DEFAULT_PROMETHEUS_PORT].
    pub fn new(listen_address: url::Url) -> Result<Self, Error> {
        if listen_address.scheme() != "http" {
            return Err(Error::InvalidListenAddress(
                listen_address.clone(),
                format!("unsupported scheme {}", listen_address.scheme()),
            ));
        }

        let saddr = listen_address
            .socket_addrs(|| Some(DEFAULT_PROMETHEUS_PORT))
            .map_err(|e| Error::InvalidListenAddress(listen_address.clone(), e.to_string()))?;
        if saddr.len() > 1 {
            tracing::warn!(
                "too many listen addresses resolved from {}: {:?}",
                listen_address,
                saddr
            )
        }
        let saddr = saddr.first().ok_or(Error::InvalidListenAddress(
            listen_address,
            "failed to resolve listen address".to_string(),
        ))?;

        let builder = PrometheusBuilder::new().with_http_listener(*saddr);
        builder.install()?;

        Ok(Self {})
    }

    fn register_metrics() {
        static START: Once = Once::new();

        START.call_once(|| {
            describe_counter!(
                "abci_last_finalized_height",
                "Last finalized height of platform chain (eg. Tenderdash)"
            );
            describe_histogram!(
                "abci_request_duration_seconds",
                metrics::Unit::Seconds,
                "Processing time of ABCI requests"
            );
        });
    }
}
/// Sets the last finalized height metric to the provided height value.
///
/// # Examples
///
/// ```
/// use drive_abci::metrics::abci_last_platform_height;
///
/// let height = 42;
/// abci_last_platform_height(height);
/// ```
pub fn abci_last_platform_height(height: u64) {
    counter!("abci_last_finalized_height", height);
}

/// Returns a `HistogramTimer` for measuring the duration of ABCI requests.
///
/// This function creates and registers a `HistogramVec` with the given `request_name`.
/// The `HistogramVec` is used to track the duration of ABCI requests in seconds.
///
/// # Arguments
///
/// * `request_name` - A string slice that holds the name of the ABCI request method.
///
/// # Example
///
/// ```
/// use drive_abci::metrics::abci_request_duration;
///
/// let timer = abci_request_duration("check_tx");
/// // ... perform some work ...
/// drop(timer);
/// ```
///
/// # Returns
///
/// A `HistogramTimer` instance for the specified `request_name`.
pub fn abci_request_duration(endpoint: &str) -> HistogramTiming {
    let labels = vec![Label::new("endpoint", endpoint.to_string())];
    HistogramTiming::new(
        metrics::Key::from_name("abci_request_duration_seconds").with_extra_labels(labels),
    )
}
