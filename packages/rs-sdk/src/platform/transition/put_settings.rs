use dpp::prelude::FeeMultiplier;
use rs_dapi_client::RequestSettings;

/// The options when putting something to platform
#[derive(Debug, Clone, Copy, Default)]
pub struct PutSettings {
    pub request_settings: RequestSettings,
    pub identity_nonce_stale_time_s: Option<u64>,
    pub fee_multiplier: Option<FeeMultiplier>,
}
