use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

impl From<IdentityUpdateTransition> for IdentityUpdateTransitionAction {
    fn from(value: IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => IdentityUpdateTransitionActionV0::from(v0).into(),
        }
    }
}

impl From<&IdentityUpdateTransition> for IdentityUpdateTransitionAction {
    fn from(value: &IdentityUpdateTransition) -> Self {
        match value {
            IdentityUpdateTransition::V0(v0) => IdentityUpdateTransitionActionV0::from(v0).into(),
        }
    }
}
