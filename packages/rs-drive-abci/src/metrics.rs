//! # Metrics Module
//!
//! This module provides a singleton implementation for managing metrics.
//! It exposes an `instance()` function to get the current instance of the `Metrics` struct.

use lazy_static::lazy_static;
use prometheus_exporter::{
    prometheus::{
        register_histogram_vec_with_registry, register_int_gauge, HistogramTimer, HistogramVec,
        IntGauge, Registry,
    },
    Exporter,
};

/// Default Prometheus port (29090)
pub const DEFAULT_PROMETHEUS_PORT: u16 = 29090;

lazy_static! {
    static ref ABCI_REGISTRY: Registry = Registry::new_custom(Some("abci".to_string()), None)
        .expect("cannot create metrics registry");
}
/// Error returned by metrics subsystem
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Prometheus server failed
    #[error("prometheus server: {0}")]
    ServerFailed(#[from] prometheus_exporter::Error),
    /// Listen address invalid
    #[error("invalid listen address {0}: {1}")]
    InvalidListenAddress(url::Url, String),
}

/// `Prometheus` is a struct that represents a Prometheus exporter server.
///
//
/// # Examples
///
/// ```
/// use your_crate::Prometheus;
/// use url::Url;
///
/// let listen_address = Url::parse("http://127.0.0.1:9090").unwrap();
/// let prometheus = Prometheus::new(listen_address).unwrap();
/// ```
pub struct Prometheus {
    _exporter: Exporter,
}

impl Prometheus {
    /// Creates and starts a new Prometheus server.
    ///
    /// # Arguments
    ///
    /// * `listen_address` - A `url::Url` representing the address the server should listen on.
    ///   The URL scheme must be "tcp". Any other scheme will result in an `Error::InvalidListenAddress`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::Prometheus;
    /// use url::Url;
    ///
    /// let listen_address = Url::parse("tcp://127.0.0.1:9090").unwrap();
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
        if listen_address.scheme() != "tcp" {
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

        let exporter = prometheus_exporter::start(*saddr)?;
        Ok(Self {
            _exporter: exporter,
        })
    }
}
/// Sets the `last_platform_height` metric to the provided height value.
///
/// This function stores the last finalized height of the platform chain.
///
/// # Examples
///
/// ```
/// use crate::metrics::last_platform_height;
///
/// let height = 42;
/// last_platform_height(height);
/// ```
pub fn last_platform_height(height: i64) {
    lazy_static! {
        static ref GAUGE: IntGauge = register_int_gauge!(
            "last_platform_height",
            "Last finalized height of platform  chain"
        )
        .expect("cannot register gauge last_platform_height");
    }
    GAUGE.set(height);
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
/// let timer = abci_request_duration("check_tx");
/// // ... perform some work ...
/// drop(timer);
/// ```
///
/// # Returns
///
/// A `HistogramTimer` instance for the specified `request_name`.
pub fn abci_request_duration(request_name: &str) -> HistogramTimer {
    lazy_static! {
        static ref HISTOGRAM: HistogramVec = register_histogram_vec_with_registry!(
            "request_duration",
            "Duration of ABCI requests",
            &["method"],
            ABCI_REGISTRY
        )
        .expect("cannot register gauge last_platform_height");
    }

    HISTOGRAM.with_label_values(&[request_name]).start_timer()
}
