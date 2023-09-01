use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

impl IdentityCreditWithdrawalTransitionAction {
    /// from
    pub fn from_identity_credit_withdrawal(
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransition,
        creation_time_ms: u64,
    ) -> Self {
        match identity_credit_withdrawal {
            IdentityCreditWithdrawalTransition::V0(v0) => {
                IdentityCreditWithdrawalTransitionActionV0::from_identity_credit_withdrawal(
                    v0,
                    creation_time_ms,
                )
                .into()
            }
        }
    }
}
