use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::unshield_transition::UnshieldTransition;

impl UnshieldTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &UnshieldTransition,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            UnshieldTransition::V0(v0) => {
                let result =
                    UnshieldTransitionActionV0::try_from_transition(v0, current_total_balance);
                result.map(|action| action.into())
            }
        }
    }
}
