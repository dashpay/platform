use std::time::Duration;

use dpp::prelude::UserFeeIncrease;
use rs_dapi_client::RequestSettings;

/// The options when putting something to platform
#[derive(Debug, Clone, Copy, Default)]
pub struct PutSettings {
    pub request_settings: RequestSettings,
    pub identity_nonce_stale_time_s: Option<u64>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    /// The time to wait for the response of a state transition after it has been broadcast
    pub wait_timeout: Option<Duration>,
}

impl From<PutSettings> for RequestSettings {
    fn from(settings: PutSettings) -> Self {
        settings.request_settings
    }
}
