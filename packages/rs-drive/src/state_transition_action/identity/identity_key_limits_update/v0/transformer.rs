use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;

impl IdentityKeyLimitsUpdateTransitionActionV0 {
    /// The action of a borrowed transition, carrying the key as it is stored before the update
    pub fn from_borrowed_transition_with_stored_key(
        value: &IdentityKeyLimitsUpdateTransitionV0,
        stored_key: IdentityPublicKey,
    ) -> Self {
        let IdentityKeyLimitsUpdateTransitionV0 {
            identity_id,
            nonce,
            key_id,
            total_budget,
            expires_at,
            user_fee_increase,
            ..
        } = value;
        IdentityKeyLimitsUpdateTransitionActionV0 {
            identity_id: *identity_id,
            nonce: *nonce,
            key_id: *key_id,
            stored_key,
            total_budget: *total_budget,
            expires_at: *expires_at,
            user_fee_increase: *user_fee_increase,
        }
    }
}
