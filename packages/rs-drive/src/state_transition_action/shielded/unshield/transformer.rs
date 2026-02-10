use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::unshield_transition::UnshieldTransition;

impl UnshieldTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &UnshieldTransition,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            UnshieldTransition::V0(v0) => {
                let result = UnshieldTransitionActionV0::try_from_transition(
                    v0,
                    nullifiers,
                    note_commitments,
                    encrypted_notes,
                    anchor,
                    current_total_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
