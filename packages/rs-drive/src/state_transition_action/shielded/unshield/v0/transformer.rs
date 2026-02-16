use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::unshield_transition::v0::UnshieldTransitionV0;

impl UnshieldTransitionActionV0 {
    /// Transforms the unshield transition into an action
    pub fn try_from_transition(
        value: &UnshieldTransitionV0,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        // fee_amount = value_balance - amount (validated to be >= 0 in structure validation)
        let fee_amount = (value.value_balance as u64).saturating_sub(value.amount);

        ConsensusValidationResult::new_with_data(UnshieldTransitionActionV0 {
            output_address: value.output_address.clone(),
            amount: value.amount,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            fee_amount,
            current_total_balance,
        })
    }
}
