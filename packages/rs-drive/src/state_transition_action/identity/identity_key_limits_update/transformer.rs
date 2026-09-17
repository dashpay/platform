use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use crate::state_transition_action::identity::identity_key_limits_update::IdentityKeyLimitsUpdateTransitionAction;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;

impl From<IdentityKeyLimitsUpdateTransition> for IdentityKeyLimitsUpdateTransitionAction {
    fn from(value: IdentityKeyLimitsUpdateTransition) -> Self {
        match value {
            IdentityKeyLimitsUpdateTransition::V0(v0) => {
                IdentityKeyLimitsUpdateTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&IdentityKeyLimitsUpdateTransition> for IdentityKeyLimitsUpdateTransitionAction {
    fn from(value: &IdentityKeyLimitsUpdateTransition) -> Self {
        match value {
            IdentityKeyLimitsUpdateTransition::V0(v0) => {
                IdentityKeyLimitsUpdateTransitionActionV0::from(v0).into()
            }
        }
    }
}
