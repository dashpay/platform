use crate::state_transition_action::shielded::token_shielded_transfer_with_shielded_fee::v0::TokenShieldedTransferWithShieldedFeeTransitionActionV0;
use crate::state_transition_action::shielded::token_shielded_transfer_with_shielded_fee::TokenShieldedTransferWithShieldedFeeTransitionAction;
use dpp::fee::Credits;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;

impl TokenShieldedTransferWithShieldedFeeTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &TokenShieldedTransferWithShieldedFeeTransition,
        fee_amount: Credits,
        current_credit_pool_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => {
                TokenShieldedTransferWithShieldedFeeTransitionActionV0::try_from_transition(
                    v0,
                    fee_amount,
                    current_credit_pool_balance,
                )
                .map(|action| action.into())
            }
        }
    }
}
