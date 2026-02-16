use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

impl ShieldedTransferTransitionActionV0 {
    /// Transforms the shielded transfer transition into an action
    pub fn try_from_transition(
        _value: &ShieldedTransferTransitionV0,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        fee_amount: Credits,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        ConsensusValidationResult::new_with_data(ShieldedTransferTransitionActionV0 {
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            fee_amount,
            current_total_balance,
        })
    }
}
