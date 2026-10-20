use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::v0::IdentityTopUpFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;

impl IdentityTopUpFromShieldedPoolTransitionActionV0 {
    /// Builds the v0 action from the v0 transition, the pool total, and the flat fee
    pub fn try_from_transition(
        value: &IdentityTopUpFromShieldedPoolTransitionV0,
        current_total_balance: Credits,
        fee_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        let notes: Vec<ShieldedActionNote> =
            value.actions.iter().map(ShieldedActionNote::from).collect();

        ConsensusValidationResult::new_with_data(IdentityTopUpFromShieldedPoolTransitionActionV0 {
            identity_id: value.identity_id,
            amount: value.top_up_amount,
            notes,
            anchor: value.anchor,
            fee_amount,
            current_total_balance,
        })
    }
}
