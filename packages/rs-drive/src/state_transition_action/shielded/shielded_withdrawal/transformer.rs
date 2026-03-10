use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;

impl ShieldedWithdrawalTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldedWithdrawalTransition,
        current_total_balance: Credits,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldedWithdrawalTransition::V0(v0) => {
                let result = ShieldedWithdrawalTransitionActionV0::try_from_transition(
                    v0,
                    current_total_balance,
                    creation_time_ms,
                );
                result.map(|action| action.into())
            }
        }
    }
}
