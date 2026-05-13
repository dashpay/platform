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

impl PutSettings {
    /// Split a [`PutSettings`] into the two dedicated builder fields
    /// (`user_fee_increase`, `state_transition_creation_options`) and the
    /// remainder of [`PutSettings`] with those two fields cleared.
    ///
    /// Used by the document `with_settings` implementations on the create,
    /// replace, and delete builders so each builder shares one implementation
    /// of the "explicit dedicated setters always win" contract:
    ///
    /// * if the builder's dedicated `user_fee_increase` /
    ///   `state_transition_creation_options` field is **already set**
    ///   (`Some(_)`), keep it — `with_settings` must not clobber an
    ///   explicit setter, regardless of call order.
    /// * otherwise, move the corresponding field out of `settings` into
    ///   the dedicated field.
    /// * in both cases, the returned `PutSettings` has both fields zeroed
    ///   so the dedicated builder fields are the sole source of truth at
    ///   sign time. Every other [`PutSettings`] field (timeouts, retry
    ///   behavior, nonce stale time, etc.) is preserved unchanged for
    ///   nonce allocation and broadcast.
    pub fn split_dedicated_fields(
        mut self,
        dedicated_user_fee_increase: &mut Option<UserFeeIncrease>,
        dedicated_state_transition_creation_options: &mut Option<StateTransitionCreationOptions>,
    ) -> Self {
        if dedicated_user_fee_increase.is_none() {
            *dedicated_user_fee_increase = self.user_fee_increase;
        }
        if dedicated_state_transition_creation_options.is_none() {
            *dedicated_state_transition_creation_options = self.state_transition_creation_options;
        }
        self.user_fee_increase = None;
        self.state_transition_creation_options = None;
        self
    }
}
