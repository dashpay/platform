//! DAPI client settings processing.

use std::time::Duration;

const DEFAULT_NETWORK: &'static str = "testnet";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RETRIES: usize = 5;

/// DAPI request settings.
/// There are four levels of settings where each next level can override all previous ones:
///
/// 1. Defaults for this library;
/// 2. [crate::DapiClient] settings;
/// 3. [crate::DapiRequest]-specific settings;
/// 4. settings for an exact request execution call.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    /// The target network.
    pub network: Option<&'static str>,
    /// Timeout for a request.
    pub timeout: Option<Duration>,
    /// Number of retries until returning the last error.
    pub retries: Option<usize>,
}

impl Settings {
    /// Create empty [Settings], which means no overrides will be applied.
    /// Actually does the same as [Default], but it's `const`.
    pub const fn default() -> Self {
        Settings {
            network: None,
            timeout: None,
            retries: None,
        }
    }

    /// Combines two instances of [Settings] with following rules:
    /// 1. in case of [Some] and [None] for one field the [Some] variant will remain,
    /// 2. in case of two [Some] variants, right hand side argument will overwrite the value.
    pub fn override_by(self, rhs: Settings) -> Self {
        Settings {
            network: rhs.network.or(self.network),
            timeout: rhs.timeout.or(self.timeout),
            retries: rhs.retries.or(self.retries),
        }
    }

    /// Fill in settings defaults.
    pub fn finalize(self) -> AppliedSettings {
        AppliedSettings {
            network: self.network.unwrap_or(DEFAULT_NETWORK),
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            retries: self.retries.unwrap_or(DEFAULT_RETRIES),
        }
    }
}

/// DAPI settings ready to use.
#[derive(Debug, Clone, Copy)]
pub struct AppliedSettings {
    /// The target network.
    pub network: &'static str,
    /// Timeout for a request.
    pub timeout: Duration,
    /// Number of retries until returning the last error.
    pub retries: usize,
}
