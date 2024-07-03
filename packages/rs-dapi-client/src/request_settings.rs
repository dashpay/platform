//! DAPI client request settings processing.

use dapi_grpc::tonic::transport::Certificate;
use std::{fs::read_to_string, path::Path, time::Duration};

/// Default low-level client timeout
const DEFAULT_CONNECT_TIMEOUT: Option<Duration> = None;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RETRIES: usize = 5;
const DEFAULT_BAN_FAILED_ADDRESS: bool = true;

/// DAPI request settings.
///
/// There are four levels of settings where each next level can override all previous ones:
/// 1. Defaults for this library;
/// 2. [crate::DapiClient] settings;
/// 3. [crate::DapiRequest]-specific settings;
/// 4. settings for an exact request execution call.
#[derive(Debug, Clone, Default)]
pub struct RequestSettings {
    /// Timeout for establishing a connection.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a request.
    pub timeout: Option<Duration>,
    /// Number of retries until returning the last error.
    pub retries: Option<usize>,
    /// Ban DAPI address if node not responded or responded with error.
    pub ban_failed_address: Option<bool>,
    /// Certificate Authority certificate to use for verifying the server's certificate.
    pub ca_certificate: Option<Certificate>,
}

impl RequestSettings {
    /// Create empty [RequestSettings], which means no overrides will be applied.
    /// Actually does the same as [Default], but it's `const`.
    pub const fn default() -> Self {
        RequestSettings {
            connect_timeout: None,
            timeout: None,
            retries: None,
            ban_failed_address: None,
            ca_certificate: None,
        }
    }

    /// Combines two instances of [RequestSettings] with following rules:
    /// 1. in case of [Some] and [None] for one field the [Some] variant will remain,
    /// 2. in case of two [Some] variants, right hand side argument will overwrite the value.
    pub fn override_by(self, rhs: RequestSettings) -> Self {
        RequestSettings {
            connect_timeout: rhs.connect_timeout.or(self.connect_timeout),
            timeout: rhs.timeout.or(self.timeout),
            retries: rhs.retries.or(self.retries),
            ban_failed_address: rhs.ban_failed_address.or(self.ban_failed_address),
            ca_certificate: rhs.ca_certificate.or(self.ca_certificate),
        }
    }

    /// Fill in settings defaults.
    pub fn finalize(self) -> AppliedRequestSettings {
        AppliedRequestSettings {
            connect_timeout: self.connect_timeout.or(DEFAULT_CONNECT_TIMEOUT),
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            retries: self.retries.unwrap_or(DEFAULT_RETRIES),
            ban_failed_address: self
                .ban_failed_address
                .unwrap_or(DEFAULT_BAN_FAILED_ADDRESS),
            ca_certificate: self.ca_certificate,
        }
    }

    /// Load a certificate from a file and set it as a CA certificate.
    pub fn with_ca_certificate(mut self, path: impl AsRef<Path>) -> std::io::Result<Self> {
        let cert_bytes = read_to_string(path)?;
        let cert = Certificate::from_pem(cert_bytes);

        self.ca_certificate = Some(cert);
        Ok(self)
    }
}

/// DAPI settings ready to use.
#[derive(Debug, Clone)]
pub struct AppliedRequestSettings {
    /// Timeout for establishing a connection.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a request.
    pub timeout: Duration,
    /// Number of retries until returning the last error.
    pub retries: usize,
    /// Ban DAPI address if node not responded or responded with error.
    pub ban_failed_address: bool,
    /// Certificate Authority certificate to use for verifying the server's certificate.
    pub ca_certificate: Option<Certificate>,
}
