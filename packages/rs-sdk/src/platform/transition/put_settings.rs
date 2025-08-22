use std::time::Duration;

use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use rs_dapi_client::RequestSettings;

/// The options when putting something to platform
#[derive(Debug, Clone, Copy, Default)]
pub struct PutSettings {
    pub request_settings: RequestSettings,
    pub identity_nonce_stale_time_s: Option<u64>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
    /// Soft limit of total time to wait for state transition to be executed (included in a block).
    ///
    /// This is an upper limit, and other settings may affect the actual wait time
    /// (like DAPI timeouts, [RequestSettings::timeout], [RequestSettings::retries], etc.).
    /// If you want to use `wait_timeout`, tune `retries` accordingly.
    ///
    /// It can be exceeded due to execution of non-cancellable parts of the Sdk.
    // TODO: Simplify timeout logic when waiting for response in Sdk, as having 3 different timeouts is confusing.
    pub wait_timeout: Option<Duration>,
}

impl From<PutSettings> for RequestSettings {
    fn from(settings: PutSettings) -> Self {
        settings.request_settings
    }
}
