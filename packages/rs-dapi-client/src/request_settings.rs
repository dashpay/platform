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
    /// Maximum gRPC response size in bytes (decoding limit).
    pub max_decoding_message_size: Option<usize>,
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
            max_decoding_message_size: None,
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
            max_decoding_message_size: rhs
                .max_decoding_message_size
                .or(self.max_decoding_message_size),
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
            max_decoding_message_size: self.max_decoding_message_size,
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
    /// Maximum gRPC response size in bytes (decoding limit).
    pub max_decoding_message_size: Option<usize>,
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

    /// Cache key fragment for the [ConnectionPool](crate::ConnectionPool),
    /// covering only the fields that affect the constructed transport client:
    /// connect timeout, response decoding limit and CA certificate.
    /// Per-request knobs (request timeout, retries, address banning) are
    /// deliberately excluded so requests that differ only in those reuse the
    /// same pooled connection.
    pub fn connection_key(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        let ca_certificate = self.ca_certificate.as_ref().map(|cert| {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            cert.as_ref().hash(&mut hasher);
            hasher.finish()
        });
        #[cfg(target_arch = "wasm32")]
        let ca_certificate: Option<u64> = None;

        format!(
            "connect_timeout={:?},max_decoding_message_size={:?},ca_certificate={:?}",
            self.connect_timeout, self.max_decoding_message_size, ca_certificate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_settings_override_by() {
        let base = RequestSettings {
            timeout: Some(Duration::from_secs(5)),
            retries: Some(3),
            connect_timeout: Some(Duration::from_secs(2)),
            ban_failed_address: Some(true),
            max_decoding_message_size: Some(1024),
        };

        // Override with partial settings
        let override_settings = RequestSettings {
            timeout: Some(Duration::from_secs(10)),
            retries: None,
            connect_timeout: None,
            ban_failed_address: None,
            max_decoding_message_size: None,
        };

        let result = base.override_by(override_settings);
        assert_eq!(result.timeout, Some(Duration::from_secs(10))); // overridden
        assert_eq!(result.retries, Some(3)); // preserved from base
        assert_eq!(result.connect_timeout, Some(Duration::from_secs(2))); // preserved
        assert_eq!(result.ban_failed_address, Some(true)); // preserved
        assert_eq!(result.max_decoding_message_size, Some(1024)); // preserved
    }

    #[test]
    fn test_request_settings_override_by_empty() {
        let base = RequestSettings {
            timeout: Some(Duration::from_secs(5)),
            retries: Some(3),
            connect_timeout: None,
            ban_failed_address: None,
            max_decoding_message_size: None,
        };

        let result = base.override_by(RequestSettings::default());
        assert_eq!(result.timeout, Some(Duration::from_secs(5)));
        assert_eq!(result.retries, Some(3));
    }

    #[test]
    fn test_request_settings_finalize_defaults() {
        let settings = RequestSettings::default();
        let applied = settings.finalize();

        assert_eq!(applied.connect_timeout, None);
        assert_eq!(applied.timeout, Duration::from_secs(10));
        assert_eq!(applied.retries, 5);
        assert!(applied.ban_failed_address);
        assert!(applied.max_decoding_message_size.is_none());
    }

    #[test]
    fn test_request_settings_finalize_custom() {
        let settings = RequestSettings {
            connect_timeout: Some(Duration::from_secs(3)),
            timeout: Some(Duration::from_secs(30)),
            retries: Some(10),
            ban_failed_address: Some(false),
            max_decoding_message_size: Some(4096),
        };

        let applied = settings.finalize();
        assert_eq!(applied.connect_timeout, Some(Duration::from_secs(3)));
        assert_eq!(applied.timeout, Duration::from_secs(30));
        assert_eq!(applied.retries, 10);
        assert!(!applied.ban_failed_address);
        assert_eq!(applied.max_decoding_message_size, Some(4096));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_applied_settings_with_ca_certificate_none() {
        let applied = RequestSettings::default().finalize();
        let result = applied.with_ca_certificate(None);
        assert!(result.ca_certificate.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_applied_settings_with_ca_certificate_some() {
        let applied = RequestSettings::default().finalize();
        let cert = Certificate::from_pem("fake-pem-data");
        let result = applied.with_ca_certificate(Some(cert));
        assert!(result.ca_certificate.is_some());
    }

    #[test]
    fn test_connection_key_ignores_per_request_settings() {
        let custom = RequestSettings {
            timeout: Some(Duration::from_secs(30)),
            retries: Some(1),
            ban_failed_address: Some(false),
            ..RequestSettings::default()
        }
        .finalize();
        let default = RequestSettings::default().finalize();

        assert_eq!(
            custom.connection_key(),
            default.connection_key(),
            "timeout/retries/banning must not split pooled connections"
        );
    }

    #[test]
    fn test_connection_key_differs_on_connection_settings() {
        let default = RequestSettings::default().finalize();

        let connect_timeout = RequestSettings {
            connect_timeout: Some(Duration::from_secs(3)),
            ..RequestSettings::default()
        }
        .finalize();
        assert_ne!(default.connection_key(), connect_timeout.connection_key());

        let decode_limit = RequestSettings {
            max_decoding_message_size: Some(16 * 1024 * 1024),
            ..RequestSettings::default()
        }
        .finalize();
        assert_ne!(default.connection_key(), decode_limit.connection_key());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_connection_key_differs_on_ca_certificate() {
        let default = RequestSettings::default().finalize();
        let with_ca = RequestSettings::default()
            .finalize()
            .with_ca_certificate(Some(Certificate::from_pem("fake-pem-data")));

        assert_ne!(default.connection_key(), with_ca.connection_key());

        let with_other_ca = RequestSettings::default()
            .finalize()
            .with_ca_certificate(Some(Certificate::from_pem("other-pem-data")));
        assert_ne!(with_ca.connection_key(), with_other_ca.connection_key());
    }
}
