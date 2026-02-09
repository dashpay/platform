use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

impl ShieldedTransferTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldedTransferTransition,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        fee_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldedTransferTransition::V0(v0) => {
                let result = ShieldedTransferTransitionActionV0::try_from_transition(
                    v0,
                    nullifiers,
                    note_commitments,
                    encrypted_notes,
                    anchor,
                    fee_amount,
                );
                result.map(|action| action.into())
            }
        }
    }
}
