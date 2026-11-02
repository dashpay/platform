use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::v0::IdentityTopUpFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::IdentityTopUpFromShieldedPoolTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;

impl IdentityTopUpFromShieldedPoolTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &IdentityTopUpFromShieldedPoolTransition,
        current_total_balance: Credits,
        fee_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => {
                IdentityTopUpFromShieldedPoolTransitionActionV0::try_from_transition(
                    v0,
                    current_total_balance,
                    fee_amount,
                )
                .map(|action| action.into())
            }
        }
    }
}
