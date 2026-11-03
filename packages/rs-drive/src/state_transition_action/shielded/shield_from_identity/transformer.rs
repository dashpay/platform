use crate::state_transition_action::shielded::shield_from_identity::v0::ShieldFromIdentityTransitionActionV0;
use crate::state_transition_action::shielded::shield_from_identity::ShieldFromIdentityTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;

impl ShieldFromIdentityTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldFromIdentityTransition,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldFromIdentityTransition::V0(v0) => {
                ShieldFromIdentityTransitionActionV0::try_from_transition(v0, current_total_balance)
                    .map(|action| action.into())
            }
        }
    }
}
