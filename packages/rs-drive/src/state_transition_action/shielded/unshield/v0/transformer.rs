use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::unshield_transition::v0::UnshieldTransitionV0;

impl UnshieldTransitionActionV0 {
    /// Transforms the unshield transition into an action
    pub fn try_from_transition(
        value: &UnshieldTransitionV0,
        notes: Vec<ShieldedActionNote>,
        anchor: [u8; 32],
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        // fee_amount = value_balance - amount (validated to be >= 0 in structure validation)
        let fee_amount = (value.value_balance as u64).saturating_sub(value.amount);

        ConsensusValidationResult::new_with_data(UnshieldTransitionActionV0 {
            output_address: value.output_address.clone(),
            amount: value.amount,
            notes,
            anchor,
            fee_amount,
            current_total_balance,
        })
    }
}
