//! DAPI client request settings processing.

use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RETRIES: usize = 5;

/// DAPI request settings.
///
/// There are four levels of settings where each next level can override all previous ones:
/// 1. Defaults for this library;
/// 2. [crate::DapiClient] settings;
/// 3. [crate::DapiRequest]-specific settings;
/// 4. settings for an exact request execution call.
#[derive(Debug, Clone, Copy)]
pub struct RequestSettings {
    /// Timeout for a request.
    pub timeout: Option<Duration>,
    /// Number of retries until returning the last error.
    pub retries: Option<usize>,
}

impl RequestSettings {
    /// Create empty [RequestSettings], which means no overrides will be applied.
    /// Actually does the same as [Default], but it's `const`.
    pub const fn default() -> Self {
        RequestSettings {
            timeout: None,
            retries: None,
        }
    }

    /// Combines two instances of [RequestSettings] with following rules:
    /// 1. in case of [Some] and [None] for one field the [Some] variant will remain,
    /// 2. in case of two [Some] variants, right hand side argument will overwrite the value.
    pub fn override_by(self, rhs: RequestSettings) -> Self {
        RequestSettings {
            timeout: rhs.timeout.or(self.timeout),
            retries: rhs.retries.or(self.retries),
        }
    }

    /// Fill in settings defaults.
    pub fn finalize(self) -> AppliedRequestSettings {
        AppliedRequestSettings {
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            retries: self.retries.unwrap_or(DEFAULT_RETRIES),
        }
    }
}

/// DAPI settings ready to use.
#[derive(Debug, Clone, Copy)]
pub struct AppliedRequestSettings {
    /// Timeout for a request.
    pub timeout: Duration,
    /// Number of retries until returning the last error.
    pub retries: usize,
}
