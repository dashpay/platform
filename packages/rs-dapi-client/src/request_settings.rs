//! DAPI client request settings processing.

use std::time::Duration;

/// Default low-level client timeout
const DEFAULT_CONNECT_TIMEOUT: Option<Duration> = None;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RETRIES: usize = 5;
const DEFAULT_BAN_FAILED_ADDRESS: bool = true;
/// The default identity contract nonce stale time in seconds
pub const DEFAULT_IDENTITY_CONTRACT_NONCE_STALE_TIME_S: u64 = 1200; //20 mins

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
    /// Timeout for a request.
    pub timeout: Option<Duration>,
    /// Number of retries until returning the last error.
    pub retries: Option<usize>,
    /// Ban DAPI address if node not responded or responded with error.
    pub ban_failed_address: Option<bool>,
    /// The amount of time after which an identity contract nonce becomes stale
    /// If it is stale we check platform before bumping the version
    /// Setting Some(0) means that we always recheck
    /// Setting a very high number means that we would never recheck
    /// Setting None uses default which is 20 mins
    pub identity_contract_nonce_stale_time_s: Option<u64>,
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
            identity_contract_nonce_stale_time_s: Some(
                DEFAULT_IDENTITY_CONTRACT_NONCE_STALE_TIME_S,
            ),
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
            identity_contract_nonce_stale_time_s: rhs
                .identity_contract_nonce_stale_time_s
                .or(self.identity_contract_nonce_stale_time_s),
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
            identity_contract_nonce_stale_time_s: self
                .identity_contract_nonce_stale_time_s
                .unwrap_or(DEFAULT_IDENTITY_CONTRACT_NONCE_STALE_TIME_S),
        }
    }
}

/// DAPI settings ready to use.
#[derive(Debug, Clone, Copy)]
pub struct AppliedRequestSettings {
    /// Timeout for establishing a connection.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a request.
    pub timeout: Duration,
    /// Number of retries until returning the last error.
    pub retries: usize,
    /// Ban DAPI address if node not responded or responded with error.
    pub ban_failed_address: bool,
    /// The amount of time after which an identity contract nonce becomes stale
    pub identity_contract_nonce_stale_time_s: u64,
}
