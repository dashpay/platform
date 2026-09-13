use crate::state_transition_action::shielded::shield_from_identity::v0::ShieldFromIdentityTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;

impl ShieldFromIdentityTransitionActionV0 {
    /// Builds the v0 action from the v0 transition and the pool total read at transform time
    pub fn try_from_transition(
        value: &ShieldFromIdentityTransitionV0,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        let notes: Vec<ShieldedActionNote> =
            value.actions.iter().map(ShieldedActionNote::from).collect();

        ConsensusValidationResult::new_with_data(ShieldFromIdentityTransitionActionV0 {
            identity_id: value.identity_id,
            nonce: value.nonce,
            shield_amount: value.amount,
            notes,
            user_fee_increase: value.user_fee_increase,
            current_total_balance,
        })
    }
}
