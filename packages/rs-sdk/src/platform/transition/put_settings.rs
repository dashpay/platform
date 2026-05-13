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
    /// of the "explicit dedicated setters always win, with_settings
    /// overrides prior settings-derived values" contract:
    ///
    /// * if the corresponding `*_explicit` flag is `true`, the builder's
    ///   dedicated field was last written by an explicit setter
    ///   ([`with_user_fee_increase`] /
    ///   [`with_state_transition_creation_options`]); keep it — explicit
    ///   setters always win over `with_settings`, regardless of call order.
    /// * otherwise, replace the corresponding dedicated field with the
    ///   value coming from `settings` — **including `None`**. This means
    ///   a second `with_settings` call **overwrites** a prior
    ///   settings-derived value rather than being silently dropped, and a
    ///   second `with_settings` with `field: None` **clears** the
    ///   settings-derived value the previous `with_settings` populated.
    ///   The contract is "last `with_settings` wins for settings-derived
    ///   fields, but an explicit setter always beats every
    ///   `with_settings`" — call order between the two stays irrelevant.
    /// * in both cases, the returned `PutSettings` has both fields zeroed
    ///   so the dedicated builder fields are the sole source of truth at
    ///   sign time. Every other [`PutSettings`] field (timeouts, retry
    ///   behavior, nonce stale time, etc.) is preserved unchanged for
    ///   nonce allocation and broadcast.
    ///
    /// [`with_user_fee_increase`]: crate::platform::documents::transitions::create::DocumentCreateTransitionBuilder::with_user_fee_increase
    /// [`with_state_transition_creation_options`]: crate::platform::documents::transitions::create::DocumentCreateTransitionBuilder::with_state_transition_creation_options
    pub fn split_dedicated_fields(
        mut self,
        dedicated_user_fee_increase: &mut Option<UserFeeIncrease>,
        user_fee_increase_explicit: bool,
        dedicated_state_transition_creation_options: &mut Option<StateTransitionCreationOptions>,
        state_transition_creation_options_explicit: bool,
    ) -> Self {
        if !user_fee_increase_explicit {
            *dedicated_user_fee_increase = self.user_fee_increase;
        }
        if !state_transition_creation_options_explicit {
            *dedicated_state_transition_creation_options = self.state_transition_creation_options;
        }
        self.user_fee_increase = None;
        self.state_transition_creation_options = None;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `with_settings` call must overwrite a prior settings-derived
    /// dedicated value, including down to `None`. This pins the
    /// "last `with_settings` wins for settings-derived fields" semantics
    /// documented on [`split_dedicated_fields`], so a second partial
    /// `PutSettings` does not silently inherit values from an earlier
    /// `with_settings` call.
    #[test]
    fn second_with_settings_with_none_clears_prior_settings_derived_value() {
        let first = PutSettings {
            user_fee_increase: Some(7),
            ..Default::default()
        };
        let mut dedicated_user_fee_increase: Option<UserFeeIncrease> = None;
        let mut dedicated_options: Option<StateTransitionCreationOptions> = None;
        let _carryover = first.split_dedicated_fields(
            &mut dedicated_user_fee_increase,
            false,
            &mut dedicated_options,
            false,
        );
        assert_eq!(dedicated_user_fee_increase, Some(7));

        let second = PutSettings::default();
        let _carryover = second.split_dedicated_fields(
            &mut dedicated_user_fee_increase,
            false,
            &mut dedicated_options,
            false,
        );
        assert_eq!(
            dedicated_user_fee_increase, None,
            "second with_settings with None must clear a prior settings-derived user_fee_increase"
        );
    }

    /// An explicit setter (`*_explicit = true`) must beat every
    /// `with_settings`, regardless of call order. A subsequent
    /// `with_settings` — even with `field: None` — must not clobber
    /// the explicit setter's value.
    #[test]
    fn explicit_setter_wins_over_subsequent_with_settings_none() {
        let mut dedicated_user_fee_increase: Option<UserFeeIncrease> = Some(42);
        let mut dedicated_options: Option<StateTransitionCreationOptions> = None;

        let settings = PutSettings::default();
        let _carryover = settings.split_dedicated_fields(
            &mut dedicated_user_fee_increase,
            // explicit setter previously wrote 42 — must be preserved.
            true,
            &mut dedicated_options,
            false,
        );
        assert_eq!(
            dedicated_user_fee_increase,
            Some(42),
            "explicit setter must win over a later with_settings, even with field: None"
        );
    }

    /// `split_dedicated_fields` must leave every non-dedicated
    /// `PutSettings` field (timeouts, retry behavior, nonce stale time,
    /// request settings) untouched, so the remainder of `PutSettings` is
    /// safe to thread through nonce allocation and broadcast.
    #[test]
    fn split_dedicated_fields_preserves_non_dedicated_fields() {
        let settings = PutSettings {
            user_fee_increase: Some(3),
            identity_nonce_stale_time_s: Some(11),
            wait_timeout: Some(Duration::from_secs(7)),
            ..Default::default()
        };
        let mut dedicated_user_fee_increase: Option<UserFeeIncrease> = None;
        let mut dedicated_options: Option<StateTransitionCreationOptions> = None;
        let remaining = settings.split_dedicated_fields(
            &mut dedicated_user_fee_increase,
            false,
            &mut dedicated_options,
            false,
        );

        assert_eq!(remaining.user_fee_increase, None);
        assert_eq!(remaining.state_transition_creation_options, None);
        assert_eq!(remaining.identity_nonce_stale_time_s, Some(11));
        assert_eq!(remaining.wait_timeout, Some(Duration::from_secs(7)));
    }
}
