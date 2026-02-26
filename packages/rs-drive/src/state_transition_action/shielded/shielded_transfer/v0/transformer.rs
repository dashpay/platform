use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

impl ShieldedTransferTransitionActionV0 {
    /// Transforms the shielded transfer transition into an action
    pub fn try_from_transition(
        _value: &ShieldedTransferTransitionV0,
        notes: Vec<ShieldedActionNote>,
        anchor: [u8; 32],
        fee_amount: Credits,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        ConsensusValidationResult::new_with_data(ShieldedTransferTransitionActionV0 {
            notes,
            anchor,
            fee_amount,
            current_total_balance,
        })
    }
}
