use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;

impl ShieldedWithdrawalTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldedWithdrawalTransition,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        current_total_balance: Credits,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldedWithdrawalTransition::V0(v0) => {
                let result = ShieldedWithdrawalTransitionActionV0::try_from_transition(
                    v0,
                    nullifiers,
                    note_commitments,
                    encrypted_notes,
                    anchor,
                    current_total_balance,
                    creation_time_ms,
                );
                result.map(|action| action.into())
            }
        }
    }
}
