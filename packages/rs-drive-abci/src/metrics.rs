//! # Metrics Module
//!
//! This module provides a singleton implementation for managing metrics.

use std::time::Duration;
use std::{sync::Once, time::Instant};

use dapi_grpc::tonic::Code;
use metrics::{counter, describe_counter, describe_histogram, histogram, Label};
use metrics_exporter_prometheus::PrometheusBuilder;

/// Default Prometheus port (29090)
pub const DEFAULT_PROMETHEUS_PORT: u16 = 29090;

const COUNTER_LAST_BLOCK_TIME: &str = "abci_last_block_time_seconds";
const COUNTER_LAST_HEIGHT: &str = "abci_last_finalized_height";
const HISTOGRAM_FINALIZED_ROUND: &str = "abci_finalized_round";
const HISTOGRAM_ABCI_REQUEST_DURATION: &str = "abci_request_duration_seconds";
/// State transition processing duration metric
const HISTOGRAM_STATE_TRANSITION_PROCESSING_DURATION: &str =
    "state_transition_processing_duration_seconds";
const LABEL_ENDPOINT: &str = "endpoint";
/// Metrics label to specify ABCI response code
pub const LABEL_ABCI_RESPONSE_CODE: &str = "response_code";
const HISTOGRAM_QUERY_DURATION: &str = "abci_query_duration";
/// Metrics label to specify state transition name
pub const LABEL_STATE_TRANSITION_NAME: &str = "st_name";
/// State transition execution code
const LABEL_STATE_TRANSITION_EXECUTION_CODE: &str = "st_exec_code";
/// Metrics label to specify check tx mode: 0 - first time check, 1 - recheck
pub const LABEL_CHECK_TX_MODE: &str = "check_tx_mode";

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
    skip: bool,
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
            skip: false,
        }
    }

    /// Returns the elapsed time since the metric was started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Add label to the histrgram
    pub fn add_label(&mut self, label: Label) {
        self.key = self.key.with_extra_labels(vec![label]);
    }

    /// Cancel timing measurement and discard the metric.
    pub fn cancel(mut self) {
        self.skip = true;

        drop(self);
    }
}

impl Drop for HistogramTiming {
    /// Implements the Drop trait for `HistogramTiming`.
    ///
    /// When a `HistogramTiming` instance is dropped, this method calculates and records the elapsed time
    /// since the start time.
    #[inline]
    fn drop(&mut self) {
        if self.skip {
            return;
        }

        let stop = self.start.elapsed();
        let key = self.key.name().to_string();

        let labels: Vec<Label> = self.key.labels().cloned().collect();
        histogram!(key, labels).record(stop.as_secs_f64());
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
/// let listen_address = Url::parse("http://127.0.0.1:57090").unwrap();
/// let prometheus = Prometheus::new(listen_address).unwrap();
/// ```
pub struct Prometheus {}

impl Prometheus {
    /// Creates and starts a new Prometheus server.
    ///
    /// # Arguments
    ///
    /// * `listen_address` - A `[url::Url]` representing the address the server should listen on.
    ///   The URL scheme must be "http". Any other scheme will result in an `Error::InvalidListenAddress`.
    ///
    /// # Examples
    ///
    /// ```
    /// use drive_abci::metrics::Prometheus;
    /// use url::Url;
    ///
    /// let listen_address = Url::parse("http://127.0.0.1:43238").unwrap();
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

        Self::register_metrics();

        Ok(Self {})
    }

    fn register_metrics() {
        static START: Once = Once::new();

        START.call_once(|| {
            describe_counter!(
                COUNTER_LAST_HEIGHT,
                "Last finalized height of platform chain (eg. Tenderdash)"
            );

            describe_counter!(
                COUNTER_LAST_BLOCK_TIME,
                metrics::Unit::Seconds,
                "Time of last finalized block, seconds since epoch"
            );

            describe_histogram!(
                HISTOGRAM_FINALIZED_ROUND,
                "Rounds at which blocks are finalized"
            );

            describe_histogram!(
                HISTOGRAM_ABCI_REQUEST_DURATION,
                metrics::Unit::Seconds,
                "Duration of ABCI request execution inside Drive per endpoint, in seconds"
            );

            describe_histogram!(
                HISTOGRAM_QUERY_DURATION,
                metrics::Unit::Seconds,
                "Duration of query request execution inside Drive per endpoint, in seconds"
            )
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
    counter!(COUNTER_LAST_HEIGHT).absolute(height);
}

/// Add round of last finalized round to [HISTOGRAM_FINALIZED_ROUND] metric.
pub fn abci_last_finalized_round(round: u32) {
    histogram!(HISTOGRAM_FINALIZED_ROUND).record(round as f64);
}

/// Set time of last block into [COUNTER_LAST_BLOCK_TIME].
pub fn abci_last_block_time(time: u64) {
    counter!(COUNTER_LAST_BLOCK_TIME).absolute(time);
}

/// Returns a `[HistogramTiming]` instance for measuring ABCI request duration.
///
/// Duration measurement starts when this function is called, and stops when returned value
/// goes out of scope.
///
/// # Arguments
///
/// * `endpoint` - A string slice representing the ABCI endpoint name.
///
/// # Examples
///
/// ```
/// use drive_abci::metrics::abci_request_duration;
/// let endpoint = "check_tx";
/// let timing = abci_request_duration(endpoint);
/// // Your code here
/// drop(timing); // stop measurement and report the metric
/// ```
pub fn abci_request_duration(endpoint: &str) -> HistogramTiming {
    let labels = vec![Label::new(LABEL_ENDPOINT, endpoint.to_string())];
    HistogramTiming::new(
        metrics::Key::from_name(HISTOGRAM_ABCI_REQUEST_DURATION).with_extra_labels(labels),
    )
}

/// Returns a `[HistogramTiming]` instance for measuring query duration.
///
/// Duration measurement starts when this function is called, and stops when returned value
/// goes out of scope.
///
/// # Arguments
///
/// * `endpoint` - A string slice representing the query name.
///
/// # Examples
///
/// ```
/// use drive_abci::metrics::query_duration_metric;
/// let endpoint = "get_identity";
/// let timing = query_duration_metric(endpoint);
/// // Your code here
/// drop(timing); // stop measurement and report the metric
/// ```
pub fn query_duration_metric(endpoint: &str) -> HistogramTiming {
    let labels = vec![endpoint_metric_label(endpoint)];
    HistogramTiming::new(
        metrics::Key::from_name(HISTOGRAM_QUERY_DURATION).with_extra_labels(labels),
    )
}

/// Create a label for the response code.
pub fn abci_response_code_metric_label(code: Code) -> Label {
    Label::new(
        LABEL_ABCI_RESPONSE_CODE,
        format!("{:?}", code).to_lowercase(),
    )
}

/// Create a label for the endpoint.
pub fn endpoint_metric_label(name: &str) -> Label {
    Label::new(LABEL_ENDPOINT, name.to_string())
}

/// Store a histogram metric for state transition processing duration
pub fn state_transition_execution_histogram(
    elapsed_time: Duration,
    state_transition_name: &str,
    code: u32,
) {
    histogram!(
        HISTOGRAM_STATE_TRANSITION_PROCESSING_DURATION,
        vec![
            Label::new(
                LABEL_STATE_TRANSITION_NAME,
                state_transition_name.to_string()
            ),
            Label::new(LABEL_STATE_TRANSITION_EXECUTION_CODE, code.to_string())
        ],
    )
    .record(elapsed_time.as_secs_f64());
}
