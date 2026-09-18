use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use crate::state_transition_action::identity::identity_key_limits_update::IdentityKeyLimitsUpdateTransitionAction;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;

impl IdentityKeyLimitsUpdateTransitionAction {
    /// The action of a borrowed transition, carrying the key as it is stored before the update
    pub fn from_borrowed_transition_with_stored_key(
        value: &IdentityKeyLimitsUpdateTransition,
        stored_key: IdentityPublicKey,
    ) -> Self {
        match value {
            IdentityKeyLimitsUpdateTransition::V0(v0) => {
                IdentityKeyLimitsUpdateTransitionActionV0::from_borrowed_transition_with_stored_key(
                    v0, stored_key,
                )
                .into()
            }
        }
    }
}
