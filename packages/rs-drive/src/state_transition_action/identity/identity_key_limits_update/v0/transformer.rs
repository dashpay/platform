use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use dpp::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;

impl From<IdentityKeyLimitsUpdateTransitionV0> for IdentityKeyLimitsUpdateTransitionActionV0 {
    fn from(value: IdentityKeyLimitsUpdateTransitionV0) -> Self {
        let IdentityKeyLimitsUpdateTransitionV0 {
            identity_id,
            revision,
            nonce,
            key_id,
            total_budget,
            expires_at,
            user_fee_increase,
            ..
        } = value;
        IdentityKeyLimitsUpdateTransitionActionV0 {
            identity_id,
            revision,
            nonce,
            key_id,
            total_budget,
            expires_at,
            user_fee_increase,
        }
    }
}

impl From<&IdentityKeyLimitsUpdateTransitionV0> for IdentityKeyLimitsUpdateTransitionActionV0 {
    fn from(value: &IdentityKeyLimitsUpdateTransitionV0) -> Self {
        let IdentityKeyLimitsUpdateTransitionV0 {
            identity_id,
            revision,
            nonce,
            key_id,
            total_budget,
            expires_at,
            user_fee_increase,
            ..
        } = value;
        IdentityKeyLimitsUpdateTransitionActionV0 {
            identity_id: *identity_id,
            revision: *revision,
            nonce: *nonce,
            key_id: *key_id,
            total_budget: *total_budget,
            expires_at: *expires_at,
            user_fee_increase: *user_fee_increase,
        }
    }
}
