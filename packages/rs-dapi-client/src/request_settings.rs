//! DAPI client request settings processing.

#[cfg(not(target_arch = "wasm32"))]
use dapi_grpc::tonic::transport::Certificate;
use std::time::Duration;

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
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestSettings {
    /// Timeout for establishing a connection.
    pub connect_timeout: Option<Duration>,
    /// Timeout for single request (soft limit).
    ///
    /// Note that the total maximum time of execution can exceed `(timeout + connect_timeout) * retries`
    /// as it accounts for internal processing time between retries.
    pub timeout: Option<Duration>,
    /// Number of retries in case of failed requests. If max retries reached, the last error is returned.
    /// 1 means one request and one retry in case of error, etc.
    pub retries: Option<usize>,
    /// Ban DAPI address if node not responded or responded with error.
    pub ban_failed_address: Option<bool>,
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
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,
        }
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
    #[cfg(not(target_arch = "wasm32"))]
    pub ca_certificate: Option<Certificate>,
}
impl AppliedRequestSettings {
    /// Use provided CA certificate for verifying the server's certificate.
    ///
    /// If set to None, the system's default CA certificates will be used.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_ca_certificate(mut self, ca_cert: Option<Certificate>) -> Self {
        self.ca_certificate = ca_cert;
        self
    }
}
