use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

impl ShieldedTransferTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldedTransferTransition,
        notes: Vec<ShieldedActionNote>,
        anchor: [u8; 32],
        fee_amount: Credits,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldedTransferTransition::V0(v0) => {
                let result = ShieldedTransferTransitionActionV0::try_from_transition(
                    v0,
                    notes,
                    anchor,
                    fee_amount,
                    current_total_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
