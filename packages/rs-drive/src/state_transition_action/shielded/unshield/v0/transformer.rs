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
        ConsensusValidationResult::new_with_data(UnshieldTransitionActionV0 {
            output_address: value.output_address.clone(),
            amount: value.amount,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            user_fee_increase: value.user_fee_increase,
            current_total_balance,
        })
    }
}
